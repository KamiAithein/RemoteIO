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
fn spawn_audio_stream(config: StreamConfig, mut audio_data_stream: (impl SyncAudioStream + std::marker::Sync + std::marker::Send + 'static), output_device: cpal::Device, liveness: Arc<Mutex<ClientStatus>>) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    
    let audio_stream = output_device
    .build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {

            let next = match audio_data_stream.get_next_batch() {
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

pub enum ClientStatus {
    Alive,
    Dead
}
pub struct Client {
    pub name: String,
    stream: cpal::Stream,
    pub stream_status: Arc<Mutex<ClientStatus>>

}




unsafe impl core::marker::Send for Client {}

#[derive(Default)]
pub struct ServerState {
    pub clients: Arc<Mutex<Vec<Client>>>,
    pub api_endpoint: String,
    pub ws_endpoint: String
}



impl Client {
    pub async fn new(stream: TcpStream) -> Result<Client, Box<dyn std::error::Error>> {
        let mut virtual_device = VirtualOutputDevice::new(accept_async(stream).await.unwrap());

        let name = virtual_device.websocket.get_ref().peer_addr().unwrap();
        println!(
            "New WebSocket connection: {}",
            name
        );

        let config_message = virtual_device.websocket.next().await.expect("couldnt get config message!").expect("couldnt get config message!");
        let config_data = config_message.into_data();

        let config_des: crate::BinStreamConfig = bincode::deserialize(&config_data).expect("couldnt deserialize message!");
        let config = cpal::StreamConfig {
            channels: config_des.channels,
            sample_rate: cpal::SampleRate(config_des.sample_rate), // Audio device default sample rate is set to 192000
            buffer_size: cpal::BufferSize::Default,
        };

        let host = cpal::default_host();
        let output_device = host
            .default_output_device()
            .expect("Failed to get default output device");

        let liveness = Arc::new(Mutex::new(ClientStatus::Alive));
        let stream_liveness = Arc::clone(&liveness);

        let stream = spawn_audio_stream(config, virtual_device, output_device, stream_liveness)?;

        return Ok(Client {name: name.to_string(), stream, stream_status: liveness});


    } 
}

//this is a sort of black magic as to why it doesn't deadlock
async fn acquire_locks2<'t1, 't2, T1, T2>(l1: &'t1 Arc<Mutex<T1>>, l2: &'t2 Arc<Mutex<T2>>) -> (MutexGuard<'t1, T1>, MutexGuard<'t2, T2>) {
    return (l1.lock().await, l2.lock().await);
}

async fn audio_link_listener(program_state: Arc<ServerState>) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(&program_state.ws_endpoint).await.unwrap();
    println!("Listening on: {}", &program_state.ws_endpoint);
    
    while let Ok((tcp_stream, _)) = listener.accept().await {
        let name = tcp_stream.peer_addr().unwrap().to_string();
            
        let client = Client::new(tcp_stream).await?;

        let mut ul_clients = program_state.clients.lock().await;
        
        ul_clients.push(client);
    }

    Ok(())
}

async fn audio_link_control(program_state: Arc<ServerState>) -> Result<(), Box<dyn std::error::Error>> {
    
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {

        let program_state = Arc::clone(&program_state);
        let print_program_state = Arc::clone(&program_state);
        
        tokio::task::spawn_blocking(move || {
            let clients = &program_state.clients;
    
            let mut ul_clients = clients.blocking_lock();
            ul_clients.retain(|client| match *client.stream_status.blocking_lock() { ClientStatus::Alive => true, _ => false });
        }).await.expect("could not remove clients due to error");

        println!("clients: {}", print_program_state.clients.lock().await.iter().map(|c| c.name.to_owned()).collect::<Vec<String>>().join(","));
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

async fn pop(State(state): State<Arc<ServerState>>) -> String {
    let mut ul_clients: MutexGuard<Vec<Client>> = state.clients.lock().await;


    return match ul_clients.pop() {
        Some(value) => format!("received {}", value.name),
        None => "no clients!".to_owned()
    };
}

async fn pop_client(State(state): State<Arc<ServerState>>, Path(client_name): Path<String>) -> String {
    let mut ul_clients = state.clients.lock().await;

    let i = ul_clients.iter().map(|client| client.name.clone()).position(|name| name == client_name);
    match i {
        Some(i) => ul_clients.remove(i).name,
        None => format!("could not find {client_name}")
    }
    
}

async fn list(State(state): State<Arc<ServerState>>) -> axum::Json<Vec<String>> {
    let ul_clients = state.clients.lock().await;

    let clients = ul_clients
        .iter()
        .map(|client| client.name.clone())
        .collect::<Vec<String>>();

    if clients.is_empty() {
        return axum::Json::from(vec![]);
    } else {
        return axum::Json::from(clients);
    }
}

async fn audio_control_main(state: Arc<ServerState>) -> Result<(), Box<dyn std::error::Error>> {
    let app_state = Arc::clone(&state);
    
    let app = Router::new()
        .route("/pop", get(pop))
        .route("/pop/:client_name", get(pop_client))
        .route("/list", get(list))
        .with_state(app_state);

    axum::Server::bind(&state.api_endpoint.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[derive(Default)]
pub struct Server {
    pub server_state: Arc<ServerState>
}

impl Server {
    pub async fn new(api_endpoint: &str, ws_endpoint: &str) -> Result<Server, Box<dyn std::error::Error>> {

        let state: Arc<ServerState> = Arc::new(ServerState{clients: Arc::new(Mutex::new(vec![])), api_endpoint: api_endpoint.to_owned(), ws_endpoint: ws_endpoint.to_owned()});

        let return_state = Arc::clone(&state);
        let audio_link_state = Arc::clone(&state);
        let audio_control_state = Arc::clone(&state);
        
        // Links audio devices
        tokio::spawn(async move {
            let _ = audio_link_main(audio_link_state).await;
        });

        // controls audio devices
        tokio::spawn(async move {
            let _ = audio_control_main(audio_control_state).await;
        });

        // let _ = futures::join!(audio_control_future, audio_link_future);

        return Ok(Server { server_state: return_state});
    } 
}

// #[tokio::main]
// async fn main() {
//     let _ = Server::start(remoteio_shared::config.rest_endpoint, remoteio_shared::config.ws_endpoint).await;
    
    
// }
