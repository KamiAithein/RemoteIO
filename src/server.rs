use std::convert::TryInto;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc};
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

use futures::{FutureExt, StreamExt};


// Creates a cpal audio stream pulling from some SyncAudioStream
fn spawn_audio_stream(config: StreamConfig, mut audio_data_stream: (impl SyncAudioStream + std::marker::Sync + std::marker::Send + 'static), output_device: cpal::Device) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    
    let audio_stream = output_device
    .build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {

            let next = audio_data_stream.get_next_batch().expect("couldnt get next value!");

            for (d, f) in data.into_iter().zip(next.iter()) {
                
                *d = f.clone();
            }


        },
        |err| eprintln!("audio stream error: {}", err),
        None,
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

#[derive(Debug)]
struct Client {
    name: String
}
struct ProgramState {
    clients: Arc<Mutex<Vec<Client>>>
}

type AudioState = std::collections::HashMap<String, cpal::Stream>;

impl Client {
    pub async fn new(stream: TcpStream) -> Result<(Client, cpal::Stream), Box<dyn std::error::Error>> {
        let mut virtual_device = VirtualOutputDevice::new(accept_async(stream).await.unwrap());

        let name = virtual_device.websocket.get_ref().peer_addr().unwrap();
        println!(
            "New WebSocket connection: {}",
            name
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

        let stream = spawn_audio_stream(config, virtual_device, output_device)?;

        return Ok((Client {name: name.to_string()}, stream));


    } 
}

//this is a sort of black magic as to why it doesn't deadlock
async fn acquire_locks2<'t1, 't2, T1, T2>(l1: &'t1 Arc<Mutex<T1>>, l2: &'t2 Arc<Mutex<T2>>) -> (MutexGuard<'t1, T1>, MutexGuard<'t2, T2>) {
    return (l1.lock().await, l2.lock().await);
}

async fn audio_link_listener(program_state: Arc<ProgramState>, audio_state: Arc<Mutex<AudioState>>) -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8000";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on: {}", addr);
    
    while let Ok((tcp_stream, _)) = listener.accept().await {
        let name = tcp_stream.peer_addr().unwrap().to_string();
            
        let (client, stream) = Client::new(tcp_stream).await?;

        let (mut ul_clients, mut ul_audio_state) = acquire_locks2(&program_state.clients, &audio_state).await;
        
        ul_clients.push(client);
        ul_audio_state.insert(name, stream);
    }

    Ok(())
}

async fn audio_link_control(program_state: Arc<ProgramState>, audio_state: Arc<Mutex<AudioState>>) -> Result<(), Box<dyn std::error::Error>> {

    loop {

        let (ul_clients, mut ul_streams) = acquire_locks2(&program_state.clients, &audio_state).await;

        let names = ul_clients.iter().map(|client| client.name.clone() ).collect::<Vec<String>>();
        let to_removes = ul_streams.keys().map(|k| k.to_owned() ).filter(|key| !names.contains(key)).collect::<Vec<String>>();
        
        for to_remove in to_removes {
            // ul_streams.get(&to_pause).unwrap().pause()?;
            ul_streams.remove(&to_remove);
        }
        
    }

    Ok(())
}

async fn audio_link_main(state: Arc<ProgramState>) -> Result<(), Box<dyn std::error::Error>> {

    let mut streams = Arc::new(Mutex::new(std::collections::HashMap::new()));

    let listener_state = Arc::clone(&state);
    let listener_streams = Arc::clone(&streams);

    let control_state = Arc::clone(&state);
    let control_streams = Arc::clone(&streams);

    // audio stream creator / destroyer signal generator
    let listener_future = audio_link_listener(listener_state, listener_streams);

    // creator / destroyer signal handler
    let control_future = audio_link_control(control_state, control_streams);

    let _ = futures::join!(listener_future, control_future);

    Ok(())
    
}

async fn pop(State(state): State<Arc<ProgramState>>) -> String {
    let mut ul_clients = state.clients.lock().await;


    return match ul_clients.pop() {
        Some(value) => format!("received {}", value.name),
        None => "no clients!".to_owned()
    };
}

async fn pop_client(State(state): State<Arc<ProgramState>>, Path(client_name): Path<String>) -> String {
    let mut ul_clients = state.clients.lock().await;

    let i = ul_clients.iter().map(|client| client.name.clone()).position(|name| name == client_name);
    match i {
        Some(i) => ul_clients.remove(i).name,
        None => format!("could not find {client_name}")
    }
    
}

async fn list(State(state): State<Arc<ProgramState>>) -> String {
    let ul_clients = state.clients.lock().await;

    let clients = ul_clients
        .iter()
        .map(|client| client.name.clone())
        .collect::<Vec<String>>()
        .join(", ");

    if clients.is_empty() {
        return "no clients!".to_string();
    } else {
        return clients;
    }
}

async fn audio_control_main(state: Arc<ProgramState>) -> Result<(), Box<dyn std::error::Error>> {

    
    let app = Router::new()
        .route("/pop", get(pop))
        .route("/pop/:client_name", get(pop_client))
        .route("/list", get(list))
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[tokio::main]
async fn main() {

    let state: Arc<ProgramState> = Arc::new(ProgramState{clients: Arc::new(Mutex::new(vec![]))});

    let audio_link_state = Arc::clone(&state);
    let audio_control_state = Arc::clone(&state);
    
    // Links audio devices
    let audio_link_future = audio_link_main(audio_link_state);

    // allows user to manipulate audio devices
    let audio_control_future = audio_control_main(audio_control_state);

    let _ = futures::join!(audio_control_future, audio_link_future);
    
}
