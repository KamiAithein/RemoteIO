// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, list, pop])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
