// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{sync::Arc, time::Duration};

use tokio::sync::{Mutex, MutexGuard};

use cpal::{Device, traits::DeviceTrait};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn client_disconnect_server(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, name: String) -> Result<String, String> {
    println!("calling client_disconnect_server");
    let ul_state = state.lock().await;
    let mut ul_client_server_connections = ul_state.client_server_connections.lock().await;

    let size_pre = ul_client_server_connections.len();
    ul_client_server_connections.retain(|client| client.name != name);

    if size_pre != ul_client_server_connections.len() + 1 {
        Err(format!("failed to pop {}", name))
    } else {
        Ok(name)
    }

}


#[tauri::command]
async fn client_connect_server(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, ws_addr: String) -> Result<(), String> {
    println!("connect! {}", ws_addr);
    let client = match remoteio_backend::client::Client::new(&ws_addr).await {
        Ok(client) => client,
        Err(e) => return Err(format!("{}", e))
    };

    let ul_state = state.lock().await;
    let ul_client_server_connections = &mut ul_state.client_server_connections.lock().await;
    ul_client_server_connections.push(client);

    Ok(())
}

#[tauri::command]
async fn client_server_list(state: tauri::State<'_, Arc<Mutex<ProgramState>>>) -> Result<Vec<String>, String> {
    println!("calling client_server_list");

    let ul_state = state.lock().await;
    let ul_client_server_connections = ul_state.client_server_connections.lock().await;

    let list = ul_client_server_connections.iter().map(|client|client.name.clone()).collect::<Vec<String>>();

    println!("calling list! {}", list.join(","));
    return Ok(list);
}

#[tauri::command]
async fn server_client_list(state: tauri::State<'_, Arc<Mutex<ProgramState>>>) -> Result<Vec<String>, String> {
    println!("calling server_client_list");

    let ul_state = state.lock().await;
    let ul_server_state = ul_state.server_state.lock().await;
    let ul_server_client_connections = ul_server_state.server_state.clients.lock().await;

    let list = ul_server_client_connections.iter().map(|client| client.name.to_owned()).collect::<Vec<String>>();


    Ok(list)
}

#[tauri::command]
async fn server_output_device_list(state: tauri::State<'_, Arc<Mutex<ProgramState>>>) -> Result<Vec<String>, String> {
    println!("calling server_output_device_list");

    let ul_state = state.lock().await;
    let ul_server_state = ul_state.server_state.lock().await;
    let server_devices = ul_server_state.server_state.output_devices.lock().await.clone();

    return Ok(server_devices);

}

#[tauri::command]
async fn change_server_output_device(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, dname: String) -> Result<(), String> {
    println!("calling change_server_output_Device");

    let mut ul_state = state.lock().await;
    let mut ul_server_state = ul_state.server_state.lock().await;
    ul_server_state.set_current_device(&dname).await.expect("could not change output device!");

    return Ok(());
}

#[tauri::command]
async fn change_client_input_device(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, cname: String, dname: String) -> Result<(), String> {
    println!("calling change_client_input_Device");

    let mut ul_state = state.lock().await;
    let mut connections = ul_state.client_server_connections.lock().await;
    
    let mut client = connections.iter_mut().filter(|client| client.name == cname).next().expect(&format!("could not find client with name {}", cname));

    let (config, device) = client.get_devices().expect("could not get devices!").into_iter().filter(|(config, device)| device.name().expect("could not get device name!") == dname).next().expect(&format!("could not find device with name {} on client {}", dname, cname));


    client.change_device(&device, config).await;

    return Ok(());
}

#[tauri::command]
async fn client_input_device_list(state: tauri::State<'_, Arc<Mutex<ProgramState>>>, name: String) -> Result<Vec<String>, String> {
    println!("calling client_input_device_list");

    // ask client for devices
    let ul_state = state.lock().await;
    let connections = ul_state.client_server_connections.lock().await;

    let client = connections
        .iter()
        .filter(|client| client.name == name)
        .next()
        .expect(&format!("no device found with name {}", name));

    Ok(client
        .get_devices()
        .expect("could not get devices!")
        .iter()
        .map(|(_, device)| device.name().expect("could not get device name!"))
        .collect::<Vec<String>>()
    )
}

#[tauri::command]
async fn client_list(state: tauri::State<'_, Arc<Mutex<ProgramState>>>) -> Result<Vec<String>, String> {
    let ul_state = state.lock().await;
    let connections = ul_state.client_server_connections.lock().await;

    Ok(connections
        .iter()
        .map(|client| client.name.clone())
        .collect::<Vec<String>>()
    )
}


#[derive(Default)]
pub struct ProgramState {
    client_server_connections: Arc<Mutex<Vec<remoteio_backend::client::Client>>>,
    server_state: Arc<Mutex<remoteio_backend::server::Server>>,
}


#[tokio::main]
async fn main() {

    // set up server
    let server_state = match remoteio_backend::server::Server::new("0.0.0.0:8080", "0.0.0.0:8000", None).await {
        Ok(server_state) => server_state,
        Err(e) => panic!("panicking with {}", e)
    };

    let state = 
        Arc::new(Mutex::new(ProgramState { 
            client_server_connections: Arc::new(Mutex::new(Vec::new())),
            server_state: Arc::new(Mutex::new(server_state)),
    }));
    let check_state = Arc::clone(&state);


    // clean up dead clients
    tokio::spawn(async move {

        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            let ul_check_state = &mut check_state.lock().await;
            let ul_client_server_connections = &mut ul_check_state.client_server_connections.lock().await;

            // let mut to_retain = 
            //     futures::future::join_all(
            //         ul_client_server_connections
            //         .iter_mut()
            //         .map(|connection| connection.is_alive())
            //     )
            //     .await
            //     .iter()
            // ;
    
            ul_client_server_connections.retain(|client| client.is_alive());

            println!("liveness: {}", ul_client_server_connections.iter().map(|client| format!("{}:{}", client.name, client.is_alive())).collect::<Vec<String>>().join(","));

            interval.tick().await;
        }

    });


    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            greet, 
            client_server_list, 
            client_disconnect_server, 
            client_connect_server,
            server_client_list,
            server_output_device_list,
            change_server_output_device,
            change_client_input_device,
            client_input_device_list,
            client_list,
            ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

}
