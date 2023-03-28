use std::convert::TryInto;
use std::sync::{Arc, Mutex};

use bytes::{Buf, Bytes};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use futures::{FutureExt, StreamExt};

#[tokio::main]
async fn main() {
    let host = cpal::default_host();
    let output_device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let input_device = host 
        .default_input_device()
        .expect("Failed to get default input device");
    let config = input_device
        .default_input_config()
        .expect("Failed to get default output config");

    let audio_data = Arc::new(Mutex::new(Vec::<u8>::new()));
    let audio_data_stream = Arc::clone(&audio_data);

    let stream = output_device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut audio_data = audio_data_stream.lock().unwrap();
                audio_data
                    .chunks(4)
                    .map(|chunk| {
                        f32::from_be_bytes(
                            chunk.try_into().expect("couldn't turn u8 slice into f32"),
                        )
                    })
                    .collect::<Vec<f32>>()
                    .iter()
                    .zip(data.iter_mut())
                    .for_each(|(f, d)| {
                        *d = f.clone();
                    });

                audio_data.clear();

            },
            |err| eprintln!("audio stream error: {}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();

    let addr = "127.0.0.1:8000";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let audio_data = Arc::clone(&audio_data);
        tokio::spawn(async move {
            let mut websocket = accept_async(stream).await.unwrap();
            println!(
                "New WebSocket connection: {}",
                websocket.get_ref().peer_addr().unwrap()
            );

            while let Some(Ok(message)) = websocket.next().await {
                if let Message::Binary(bytes) = message {
                    let mut audio_data_lock = audio_data.lock().unwrap();
                    audio_data_lock.extend_from_slice(&bytes);
                }
            }
        });
    }
}
