use std::convert::TryInto;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc};
use std::time::Duration;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::get;
use tokio::sync::{Mutex, MutexGuard};
use tokio::net::TcpStream;


use async_trait::async_trait;


use bytes::{Buf, Bytes};
use cpal::StreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::net::TcpListener;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use futures::{FutureExt, StreamExt, Future};


// Creates a cpal audio stream pulling from some SyncAudioStream
fn spawn_audio_stream(config: &StreamConfig, audio_data_stream: Arc<Mutex<(impl SyncAudioStream + std::marker::Sync + std::marker::Send + 'static)>>, output_device: &mut cpal::Device, liveness: Arc<Mutex<ClientStatus>>) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    
    let audio_stream = output_device
    .build_output_stream(
        config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            
            let mut ul_audio_data_stream = audio_data_stream.blocking_lock();
            let next = match ul_audio_data_stream.get_next_batch() {
                Some(next) => next,
                None => {
                    *liveness.blocking_lock() = ClientStatus::Dead;
                    return
                },
            };

            for (d, f) in data.into_iter().zip(next.iter()) {
                
                *d = f.clone();
            }


        },
        |err| eprintln!("audio stream error: {}", err),
        Some(Duration::from_secs(5)),
    )
    .unwrap();

    audio_stream.play().expect("couldnt play stream");

    return Ok(audio_stream);
}


trait SyncAudioStream {
    fn get_next_batch(&mut self) -> Option<Vec<f32>>;
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


impl SyncAudioStream for VirtualOutputDevice {
    fn get_next_batch(&mut self) -> Option<Vec<f32>> {
        let runtime = tokio::runtime::Runtime::new().expect("Unable to create a runtime");

        let next_message = runtime.block_on(async { 
            self.websocket
                .next()
                .await
        });

            if let Some(Ok(message)) = next_message {

                let mut buffer = self.buffer.blocking_lock();
                
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

            let mut buffer = self.buffer.blocking_lock();

            if buffer.len() == 0 {
                println!("buffer is empty from syncaudiostream!");
                return None;
            }

            let clone = buffer.clone();
            buffer.clear();

            return Some(clone);
        // })

        
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

#[derive(Clone, Copy)]
pub enum ClientStatus {
    Alive,
    Dead
}
pub struct Client {
    pub name: String,
    stream: cpal::Stream,
    pub stream_status: Arc<Mutex<ClientStatus>>,
    pub stream_config: cpal::StreamConfig,
    virtual_output: Arc<Mutex<VirtualOutputDevice>>


}




unsafe impl core::marker::Send for Client {}

pub struct ServerState {
    pub clients: Arc<Mutex<Vec<Client>>>,
    pub output_devices: Arc<Mutex<Vec<String>>>,
    pub api_endpoint: String,
    pub ws_endpoint: String,
    pub current_device: Arc<Mutex<cpal::Device>>
}

impl Default for ServerState {
    fn default() -> Self {
        let default_output_device = cpal::default_host().default_output_device().expect("could not get default output device!");

        Self { 
            current_device: Arc::new(Mutex::new(default_output_device)),
            clients: Default::default(),
            output_devices: Default::default(),
            api_endpoint: "http://0.0.0.0:8000".to_owned(),
            ws_endpoint: "ws://0.0.0.0:80000".to_owned(),
        }
    }
}


impl Client {
    pub async fn new(stream: TcpStream, device_name: &str) -> Result<Client, Box<dyn std::error::Error>> {
        let mut virtual_device = Arc::new(Mutex::new(VirtualOutputDevice::new(accept_async(stream).await.unwrap())));
        let mut ul_virtual_device = virtual_device.lock().await;

        let name = ul_virtual_device.websocket.get_ref().peer_addr().unwrap();
        println!(
            "New WebSocket connection: {}",
            name
        );

        let config_message = ul_virtual_device.websocket.next().await.expect("couldnt get config message!").expect("couldnt get config message!");
        let config_data = config_message.into_data();
        
        let config_des = match bincode::deserialize(&config_data).expect("couldnt deserialize message!") {
            crate::BinMessages::BinConfig(config_des) => config_des,
            _ => panic!("config message not sent first!")
        };

        let config = cpal::StreamConfig {
            channels: config_des.channels,
            sample_rate: cpal::SampleRate(config_des.sample_rate), // Audio device default sample rate is set to 192000
            buffer_size: cpal::BufferSize::Default,
        };


        let host = cpal::default_host();
        let mut output_device = host
            .output_devices()
            .unwrap()
            .filter(|d|d.name().expect("could not get audio device name!") == device_name)
            .next()
            .expect(&format!("could not find audio device with name {}", device_name));

        let liveness = Arc::new(Mutex::new(ClientStatus::Alive));
        let stream_liveness = Arc::clone(&liveness);

        let stream = spawn_audio_stream(&config, Arc::clone(&virtual_device), &mut output_device, stream_liveness)?;

        return Ok(Client {name: name.to_string(), stream, stream_status: liveness, stream_config: config.clone(), virtual_output: Arc::clone(&virtual_device)});


    } 
}


async fn audio_link_listener(program_state: Arc<ServerState>) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(&program_state.ws_endpoint).await.unwrap();
    println!("Listening on: {}", &program_state.ws_endpoint);
    
    while let Ok((tcp_stream, _)) = listener.accept().await {
        let name = tcp_stream.peer_addr().unwrap().to_string();
            
        let client = Client::new(tcp_stream, &program_state.current_device.lock().await.name().expect("couldnt get device name!")).await?;

        let mut ul_clients = program_state.clients.lock().await;
        
        ul_clients.push(client);
    }

    Ok(())
}

async fn audio_link_control(program_state: Arc<ServerState>) -> Result<(), Box<dyn std::error::Error>> {
    
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {

        let program_state = Arc::clone(&program_state);
        
        let clients = &program_state.clients;
        
        let mut ul_clients = clients.lock().await;
        
        for client in ul_clients.iter_mut() {
            let status = client.stream_status.lock().await;
            match *status {
                ClientStatus::Dead => {let _ = client.stream.pause();},
                _ => {}
            }
        }
        
        // tokio::task::spawn_blocking(move || {
        let statuses = 
            futures::future::join_all(
                ul_clients
                        .iter_mut()
                        .map(|client| client.stream_status.lock()))
                        .await
                        .iter()
                        .map(|cs|ClientStatus::from(**cs))
                        .collect::<Vec<ClientStatus>>();

        let mut s_iter = statuses.iter();
        
        ul_clients.retain(
            |_| {
                match s_iter.next() {
                    Some(ClientStatus::Alive) => true,
                    _ => false
                }
            }
        );
        // }).await.expect("could not remove clients due to error");

        println!("clients: {}", ul_clients.iter().map(|c| c.name.to_owned()).collect::<Vec<String>>().join(","));
        interval.tick().await;
        
    }

    Ok(())
}

async fn audio_link_main(state: Arc<ServerState>) -> Result<(), Box<dyn std::error::Error>> {

    let listener_state = Arc::clone(&state);

    let control_state = Arc::clone(&state);

    // audio stream creator / destroyer signal generator
    tokio::spawn(async move {
        let _ = audio_link_listener(listener_state).await; 
    });

    // creator / destroyer signal handler
    tokio::spawn(async move {
        let _ = audio_link_control(control_state).await;
    });


    Ok(())
    
}





#[derive(Default)]
pub struct Server {
    pub server_state: Arc<ServerState>
}

impl Server {
    pub async fn set_current_device(&mut self, dname: &str) -> Result<(), Box<dyn std::error::Error>> {

        let mut ul_current_device = self.server_state.current_device.lock().await;


        *ul_current_device =
            cpal::default_host()
            .output_devices()
            .expect("could not get output devices")
            .filter(|d|d.name().expect("could not get device name") == dname)
            .next()
            .expect(&format!("could not find device with name {}", dname));

        // for every client, delete the old stream and create a new one with the new device
        for client in self.server_state.clients.lock().await.iter_mut() {
            *client.stream_status.lock().await = ClientStatus::Dead;
            let _ = client.stream.pause();
            
            client.stream_status = Arc::new(Mutex::new(ClientStatus::Alive));


            match spawn_audio_stream(
                &client.stream_config, 
                Arc::clone(&client.virtual_output), 
                &mut ul_current_device, 
                Arc::clone(&client.stream_status)
            ) {
                Ok(stream) => client.stream = stream,
                Err(e) => panic!("error changing audio device!")
            };

            // spawn_audio_stream(config, audio_data_stream, output_device, liveness)
        }

        Ok(())
    }

    pub async fn new(api_endpoint: &str, ws_endpoint: &str, device: Option<String>) -> Result<Server, Box<dyn std::error::Error>> {

        let output_devices = Arc::new(Mutex::new(vec![]));
        let state_output_devices = Arc::clone(&output_devices);

        let device = match device {
            Some(device) => cpal::default_host()
                                        .output_devices()
                                        .expect("could not get output devices")
                                        .filter(|d|d.name().expect("could not get device name") == device)
                                        .next()
                                        .expect("could not find device!"),

            None => cpal::default_host().default_output_device().expect("could not get default output device!")
        };

        let state: Arc<ServerState> = 
            Arc::new(
                ServerState {
                    clients: Arc::new(Mutex::new(vec![])), 
                    api_endpoint: api_endpoint.to_owned(), 
                    ws_endpoint: ws_endpoint.to_owned(), 
                    output_devices: state_output_devices,
                    current_device: Arc::new(Mutex::new(device))
                }
            );

        let return_state = Arc::clone(&state);
        let audio_link_state = Arc::clone(&state);
        
        // Links audio devices
        tokio::spawn(async move {
            let _ = audio_link_main(audio_link_state).await;
        });

        // keeps audio output devices up to date
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                let input_devices = 
                cpal::default_host()
                .output_devices()
                .expect("could not get input devices!")
                .map(|d| d.name().expect("could not get device name!"))
                .collect::<Vec<String>>();
            
                *output_devices.lock().await = input_devices;

                interval.tick().await;
            }
            
        });

        // let _ = futures::join!(audio_control_future, audio_link_future);

        return Ok(Server { server_state: return_state});
    } 
}

// #[tokio::main]
// async fn main() {
//     let _ = Server::start(remoteio_shared::config.rest_endpoint, remoteio_shared::config.ws_endpoint).await;
    
    
// }
