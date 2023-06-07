// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{sync::{Mutex, MutexGuard, Arc}, time::Duration};

use remoteio_backend::{client::{Client, SyncClient}, server::SyncServer};
use remoteio_backend::server::Server;

use cpal::{Device, traits::{DeviceTrait, HostTrait}};



// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}


#[tauri::command]
async fn get_server_connections(state: tauri::State<'_, Arc<Mutex<ProgramState>>>) -> Result<Vec<String>, String> {

    let ul_state = state.lock().expect("Could not lock state!");

    let clients = match ul_state.server_state.list_clients() {
        Ok(clients) => clients,
        Err(e) => panic!("panicking when getting server connections due to {}", e),
    };

    return Ok(clients.iter().map(|client| client.clone()).collect::<Vec<String>>());
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
    
    let mut ul_state = state.lock().expect("Could not lock state!");

    let names = ul_state.client_server_connections.iter_mut().map(|client| client.name()).collect();

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
    let mut ul_state = state.lock().expect("Could not lock state!");

    //default device should be changed 
    let default_device = cpal::default_host().default_input_device().expect("could not get default input device!");

    //TODO what name??
    let mut client = remoteio_backend::client::Client::new(&address, default_device);
    client.connect(&address).expect("could not connect client to given address!");

    ul_state.client_server_connections.push(client);

    Ok(())
}

#[tauri::command]
async fn client_disconnect_client(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, cpos: usize) -> Result<(), String> { 

    println!("client disconnect client {}", cpos);
    let mut ul_state = state.lock().expect("Could not lock state!");

    let connections = &mut ul_state.client_server_connections;

    let to_disconnect = connections.remove(cpos);

    Ok(())
}

#[tauri::command]
async fn change_server_output_device(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, cpos: usize, dname: String) -> Result<(), String> {
    let device = cpal::default_host().output_devices().expect("could not get output devices!").filter(|device| device.name().expect("could not get device name!") == dname).next().expect("could not find server output device!");
 
    let mut ul_state = state.lock().expect("Could not lock state!");
    //TODO uncomment and fix thsi
    // ul_state.server_state.change_output_device(cpos, device).expect("could not change output device!");

    Ok(())
}

#[tauri::command]
async fn change_client_input_device(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, cpos: usize, dname: String) -> Result<(), String> {
    let mut ul_state = state.lock().expect("Could not lock state!");

    let client = ul_state.client_server_connections.get_mut(cpos).expect("could not get client from tauri clients state");
    
    let device = cpal::default_host().input_devices().expect("could not get input devices!").filter(|device| device.name().expect("could not get device name!") == dname).next().expect("could not find client input device!");
    client.change_source_device(device).expect("could not change source device!");

    
    Ok(())
}

pub struct ProgramState {
    client_server_connections: Vec<remoteio_backend::client::Client>,
    server_state: remoteio_backend::server::Server,
}

fn main() {

    let server_state = remoteio_backend::server::Server::new("0.0.0.0:8000", None).expect("could not create server!");

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
