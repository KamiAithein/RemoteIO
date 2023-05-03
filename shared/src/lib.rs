use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Deserialize, Debug)]
pub struct RemoteIOConfig {
    pub rest_endpoint: &'static str,
    pub ws_endpoint: &'static str,
    pub server_endpoint: &'static str
}

pub static config: RemoteIOConfig  = RemoteIOConfig {
    rest_endpoint: "http://0.0.0.0:3000",
    server_endpoint: "http://0.0.0.0:8000",
    ws_endpoint: "ws://0.0.0.0:8000"

};