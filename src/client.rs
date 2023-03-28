use std::{convert::TryInto, slice::from_mut};

use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use bytes::Bytes;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures::sink::Send;


use futures::{StreamExt, SinkExt};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://127.0.0.1:8000";
    let (raw_socket, _) = connect_async(url).await.expect("Failed to connect");
    
    let (mut writer, mut reader) = raw_socket.split();

    let host = cpal::default_host();
    let input_device = host.default_input_device().expect("Failed to get default input device");
    let config = input_device.default_input_config().expect("Failed to get default input config");
    
    let bin_config_struct = remoteio::BinStreamConfig {
        channels:  config.channels(),
        sample_rate: config.sample_rate().0,
        buffer_size: 4096

    };

    let bin_config = bincode::serialize(&bin_config_struct)?;

    writer.send(Message::binary(bin_config)).await.expect("error sending config!");

    let stream = input_device
        .build_input_stream(&config.into(), move |data: &[f32], _| {
            let bytes = data.iter().flat_map(|f| f.to_be_bytes()).collect::<Vec<u8>>();

            let runtime = tokio::runtime::Runtime::new().expect("Unable to create a runtime");

            runtime.block_on(async {
                writer.send(Message::binary(bytes)).await.expect("couldnt send message!");
            });
       
        }, |err| eprintln!("audio stream error: {}", err),
        None)
        .unwrap();

    stream.play().unwrap();

    while let Some(Ok(msg)) = reader.next().await {
        println!("Received message: {}", msg);
    }

    Ok(())
}
