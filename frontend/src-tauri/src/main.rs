// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use tokio::sync::Mutex;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn list() -> Vec<String> {
    let list = 
        reqwest::get("http://".to_owned() + remoteio_shared::config.rest_endpoint + "/list")
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap();

    println!("{:?}", list);

    return list;
}

#[tauri::command]
async fn pop(name: String) -> String {
    let response = reqwest::get("http://".to_owned() + remoteio_shared::config.rest_endpoint + "/pop" + "/" + &name)
    .await
    .unwrap()
    .text()
    .await
    .unwrap();

    println!("removoing {}", response);

    response
}


#[tauri::command]
async fn connect(state: tauri::State<'_, ClientState>, ws_addr: String) -> Result<(), String> {
    println!("connect! {}", ws_addr);
    let client = match remoteio_backend::client::Client::new(&ws_addr).await {
        Ok(client) => client,
        Err(e) => return Err(format!("{}", e))
    };

    let connections = &mut state.0.lock().await.connections;
    connections.push(Arc::new(Mutex::new(client)));

    Ok(())
}


#[derive(Default)]
pub struct InnerClientState {
    connections: Vec<Arc<Mutex<remoteio_backend::client::Client>>>
}

pub struct ClientState(pub Mutex<InnerClientState>);

#[tokio::main]
async fn main() {

    // tokio::spawn(async move {

    // });

    tauri::Builder::default()
        .manage(ClientState(Default::default()))
        .invoke_handler(tauri::generate_handler![greet, list, pop, connect])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

}
