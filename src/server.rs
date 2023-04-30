use std::convert::TryInto;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::net::TcpStream;


use async_trait::async_trait;


use bytes::{Buf, Bytes};
use cpal::StreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::net::TcpListener;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use futures::{FutureExt, StreamExt};

fn spawn_audio_stream(config: StreamConfig, mut audio_data_stream: (impl AsyncAudioStream + std::marker::Sync + std::marker::Send + 'static), output_device: cpal::Device) {

    tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Runtime::new().expect("Unable to create a runtime");
        
        let audio_stream = output_device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {

                runtime.block_on(async {
                    let next = audio_data_stream.get_next_batch().await.expect("couldnt get next value!");

                    for (d, f) in data.into_iter().zip(next.iter()) {
                        
                        *d = f.clone();
                    }
                });


            },
            |err| eprintln!("audio stream error: {}", err),
            None,
        )
        .unwrap();

        audio_stream.play().expect("couldnt play stream");

        loop {}
    });
}

// map input -> output
// list[]

struct State {
    virtual_devices: Vec<VirtualOutputDevice>
}


#[async_trait]
trait AsyncAudioStream {
    async fn get_next_batch(&mut self) -> Option<Vec<f32>>;
}

struct RealOutputDevice {
    buffer: Arc<Mutex<Vec<f32>>>,
    device: cpal::Device
}

impl RealOutputDevice {
    fn new(device: cpal::Device, config: &cpal::StreamConfig) -> (cpal::Stream, Self) {
        let buffer = Arc::new(Mutex::new(vec![]));
        let buffer_rx = Arc::clone(&buffer);

        let runtime = tokio::runtime::Runtime::new().expect("Unable to create a runtime");
        

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                runtime.block_on(async {
                    let mut buffer = buffer_rx.lock().await;

                    buffer.append(&mut data.to_vec());
                });

            },
            |_|{},
            None
        ).expect("couldnt build device output stream for real output device");

        stream.play().expect("couldnt play stream for real output device");



        return 
            (stream, 
            Self {
                buffer: buffer,
                device: device,
            });
        
    }
}

#[async_trait]
impl AsyncAudioStream for RealOutputDevice {
    async fn get_next_batch(&mut self) -> Option<Vec<f32>> {
        let mut buffer = self.buffer.lock().await;
        if buffer.len() == 0 {
            return None;
        }

        let clone = buffer.clone();
        buffer.clear();

        return Some(clone);
    }
}
struct VirtualOutputDevice {
    buffer: Arc<Mutex<Vec<f32>>>,
    websocket: WebSocketStream<TcpStream>
}

impl VirtualOutputDevice {
    fn new(websocket: WebSocketStream<TcpStream>) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(vec![])),
            websocket: websocket
        }
    }
}

#[async_trait]
impl AsyncAudioStream for VirtualOutputDevice {
    async fn get_next_batch(&mut self) -> Option<Vec<f32>> {
        let next_message = 
            self.websocket
                .next()
                .await;

        if let Some(Ok(message)) = next_message {

            let mut buffer = self.buffer.lock().await;
            
            let mut message_data = 
                message.into_data()
                            .chunks(4)
                            .map(|chunk| f32::from_be_bytes(chunk.try_into().expect("couldnt turn chunk into slice")))
                            .collect::<Vec<f32>>();
            
            buffer.append(&mut message_data);

            buffer.reverse();
            buffer.truncate(1024);
            buffer.reverse();
        }

        let mut buffer = self.buffer.lock().await;

        if buffer.len() == 0 {
            println!("buffer is empty from asyncaudiostream!");
            return None;
        }

        let clone = buffer.clone();
        buffer.clear();

        return Some(clone);
    }
}

#[tokio::main]
async fn main() {

    
    let addr = "127.0.0.1:8000";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {

        tokio::spawn(async move {
            

            let mut virtual_device = VirtualOutputDevice::new(accept_async(stream).await.unwrap());


            println!(
                "New WebSocket connection: {}",
                virtual_device.websocket.get_ref().peer_addr().unwrap()
            );

            let config_message = virtual_device.websocket.next().await.expect("couldnt get config message!").expect("couldnt get config message!");
            let config_data = config_message.into_data();

            let config_des: remoteio::BinStreamConfig = bincode::deserialize(&config_data).expect("couldnt deserialize message!");
            let config = cpal::StreamConfig {
                channels: config_des.channels,
                sample_rate: cpal::SampleRate(config_des.sample_rate), // Audio device default sample rate is set to 192000
                buffer_size: cpal::BufferSize::Default,
            };

            let host = cpal::default_host();
            let output_device = host
                .default_output_device()
                .expect("Failed to get default output device");
            
            spawn_audio_stream(config, virtual_device, output_device);

        });
    }
}
