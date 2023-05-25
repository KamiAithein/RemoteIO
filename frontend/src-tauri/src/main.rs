// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{sync::Arc, time::Duration};

use remoteio_backend::client::Client;
use remoteio_backend::server::Server;
use tokio::sync::{Mutex, MutexGuard};

use cpal::{Device, traits::{DeviceTrait, HostTrait}};



// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}


#[tauri::command]
async fn get_server_connections(state: tauri::State<'_, Arc<Mutex<ProgramState>>>) -> Result<Vec<String>, String> {

    let ul_state = state.lock().await;

    let clients = match ul_state.server_state.list_clients().await {
        Ok(clients) => clients,
        Err(e) => panic!("panicking when getting server connections due to {}", e),
    };

    return Ok(clients.iter().map(|client| client.url.clone()).collect::<Vec<String>>());
}


#[tauri::command]
async fn get_server_devices(state: tauri::State<'_, Arc<Mutex<ProgramState>>>) -> Result<Vec<String>, String> {
    return Ok(
        cpal::default_host()
        .output_devices()
        .expect("could not get output devices!")
        .map(|device|device.name().expect("could not get name!"))
        .collect::<Vec<String>>()
    );
    
}

#[tauri::command]
async fn get_client_connections(state: tauri::State<'_, Arc<Mutex<ProgramState>>>) -> Result<Vec<String>, String> {
    
    let mut ul_state = state.lock().await;

    let names = futures::future::join_all(ul_state.client_server_connections.iter_mut().map(|client| client.name())).await;

    return Ok(names);
}

#[tauri::command]
async fn get_client_devices(state: tauri::State<'_, Arc<Mutex<ProgramState>>>) -> Result<Vec<String>, String> {
    return Ok(
        cpal::default_host()
        .input_devices()
        .expect("could not get input devices!")
        .map(|device|device.name().expect("could not get name!"))
        .collect::<Vec<String>>()
    );
}

#[tauri::command]
async fn connect_client(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, address: String) -> Result<(), String> { 
    let mut ul_state = state.lock().await;

    //default device should be changed 
    let default_device = cpal::default_host().default_input_device().expect("could not get default input device!");

    let mut client = remoteio_backend::client::Metal2RemoteClient::new(default_device);
    client.connect(&address).await.expect("could not connect client to given address!");

    ul_state.client_server_connections.push(client);

    Ok(())
}

#[tauri::command]
async fn client_disconnect_client(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, client: String) -> Result<(), String> { 

    println!("client disconnect client {}", client);
    let mut ul_state = state.lock().await;

    let connections = &mut ul_state.client_server_connections;

    
    let cnames =  
        futures::future::join_all(
            connections
            .iter_mut()
            .map(|connection|connection.name())
        )
        .await;

    let mut to_keep =
        cnames
        .iter()
        .map(|cname| cname.to_owned() != client)
        .into_iter();

        connections.retain(|_| to_keep.next().expect("this should never happen iterator error"));

    Ok(())
}

#[tauri::command]
async fn change_server_output_device(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, dname: String) -> Result<(), String> {
    let device = cpal::default_host().output_devices().expect("could not get output devices!").filter(|device| device.name().expect("could not get device name!") == dname).next().expect("could not find server output device!");
 
    let mut ul_state = state.lock().await;
    ul_state.server_state.change_output_device(device).await.expect("could not change output device!");

    Ok(())
}

#[tauri::command]
async fn change_client_input_device(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, dname: String) -> Result<(), String> {
    let mut ul_state = state.lock().await;
    
    for client in &mut ul_state.client_server_connections {
        let device = cpal::default_host().input_devices().expect("could not get input devices!").filter(|device| device.name().expect("could not get device name!") == dname).next().expect("could not find client input device!");
        client.change_source_device(device).await.expect("could not change source device!");
    }
    
    Ok(())
}


#[derive(Default)]
pub struct ProgramState {
    client_server_connections: Vec<remoteio_backend::client::Metal2RemoteClient>,
    server_state: remoteio_backend::server::MetalServer,
}

#[tokio::main]
async fn main() {

    let mut server_state = remoteio_backend::server::MetalServer::new("0.0.0.0:8000");
    let _ = server_state.bind().await;

    let state = 
        Arc::new(Mutex::new(ProgramState { 
            client_server_connections: vec![],
            server_state: server_state,
    }));
    


    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            greet, 
            get_server_connections,
            get_server_devices,
            get_client_connections,
            get_client_devices,
            connect_client,
            client_disconnect_client,
            change_server_output_device,
            change_client_input_device,
            ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

}
