// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{sync::Arc, time::Duration};

use tokio::sync::{Mutex, MutexGuard};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn pop(state: tauri::State<'_, Arc<Mutex<ClientState>>>, name: String) -> Result<String, String> {
    let ul_state = state.lock().await;
    let mut ul_connections = ul_state.connections.lock().await;

    let size_pre = ul_connections.len();
    ul_connections.retain(|client| client.name != name);

    if size_pre != ul_connections.len() + 1 {
        Err(format!("failed to pop {}", name))
    } else {
        Ok(name)
    }

}


#[tauri::command]
async fn connect(state: tauri::State<'_, Arc<Mutex<ClientState>>>, ws_addr: String) -> Result<(), String> {
    println!("connect! {}", ws_addr);
    let client = match remoteio_backend::client::Client::new(&ws_addr).await {
        Ok(client) => client,
        Err(e) => return Err(format!("{}", e))
    };

    let ul_state = state.lock().await;
    let ul_connections = &mut ul_state.connections.lock().await;
    ul_connections.push(client);

    Ok(())
}

#[tauri::command]
async fn list(state: tauri::State<'_, Arc<Mutex<ClientState>>>) -> Result<Vec<String>, String> {
    let ul_state = state.lock().await;
    let ul_connections = ul_state.connections.lock().await;

    let list = ul_connections.iter().map(|client|client.name.clone()).collect::<Vec<String>>();

    println!("calling list! {}", list.join(","));
    return Ok(list);
}


#[derive(Default)]
pub struct ClientState {
    connections: Arc<Mutex<Vec<remoteio_backend::client::Client>>>
}


#[tokio::main]
async fn main() {

    let state = Arc::new(Mutex::new(ClientState{ connections: Arc::new(Mutex::new(Vec::new())) }));
    let check_state = Arc::clone(&state);


    // clean up dead clients
    tokio::spawn(async move {

        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            let ul_check_state = &mut check_state.lock().await;
            let ul_connections = &mut ul_check_state.connections.lock().await;
    
            ul_connections.retain(|client| client.is_alive());

            println!("liveness: {}", ul_connections.iter().map(|client| format!("{}:{}", client.name, client.is_alive())).collect::<Vec<String>>().join(","));

            interval.tick().await;
        }

    });

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![greet, list, pop, connect])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

}
