use std::convert::TryInto;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use bytes::{Buf, Bytes};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use futures::{FutureExt, StreamExt};


#[tokio::main]
async fn main() {
    
    let addr = "127.0.0.1:8000";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on: {}", addr);

    

    
    
    
    
    
    while let Ok((stream, _)) = listener.accept().await {

        let audio_data = Arc::new(Mutex::new(Vec::<u8>::new()));
    
        tokio::spawn(async move {
            
            let audio_data_stream = Arc::clone(&audio_data);
    
            
            let mut websocket = accept_async(stream).await.unwrap();
            println!(
                "New WebSocket connection: {}",
                websocket.get_ref().peer_addr().unwrap()
            );

            let config_message = websocket.next().await.expect("couldnt get config message!").expect("couldnt get config message!");
            let config_data = config_message.into_data();

            let config_des: remoteio::BinStreamConfig = bincode::deserialize(&config_data).expect("couldnt deserialize message!");
            let config = cpal::StreamConfig {
                channels: config_des.channels,
                sample_rate: cpal::SampleRate(config_des.sample_rate), // Audio device default sample rate is set to 192000
                buffer_size: cpal::BufferSize::Default,
            };

            tokio::task::spawn_blocking(move || {
        
                let host = cpal::default_host();
                let output_device = host
                    .default_output_device()
                    .expect("Failed to get default output device");

        
                
                let audio_stream = output_device
                .build_output_stream(
                    &config,
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
        
                audio_stream.play().expect("couldnt play stream");

                loop {}
            });

            while let Some(Ok(message)) = websocket.next().await {
                if let Message::Binary(bytes) = message {
                    let mut audio_data_lock = audio_data.lock().unwrap();
                    audio_data_lock.extend_from_slice(&bytes);
                }
            }

            
        });
    }
}
