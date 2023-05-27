pub use std::convert::TryInto;
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

use futures::{FutureExt, StreamExt, Future, SinkExt};


pub static BUFFER_BATCH_SIZE: usize = 4096;

struct Concurrent {}

impl Concurrent {
    pub async fn lock_all_ordered<D>(to_locks: impl Iterator<Item = impl Future<Output=D>>) -> Vec<D> {
        let mut locked = vec![];
        for lock in to_locks.into_iter() {
            locked.push(lock.await);
        }


        locked
    }
}

// what does a server do
// a server binds to a port and listens


#[async_trait]
pub trait Server {
    async fn bind(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn list_clients(&self) -> Result<Vec<Client>, Box<dyn std::error::Error>>;
    async fn disconnect_client(&mut self, url: &str) -> Result<Option<Client>, Box<dyn std::error::Error>>;
    async fn change_output_device(&mut self, cpos: usize, new_output: cpal::Device) -> Result<(), Box<dyn std::error::Error>>;
}
pub struct MetalStream {
    pub stream: cpal::Stream
}

impl MetalStream {
    pub async fn new(connection: &Arc<Mutex<Connection>>) {

        
        let mut connection = connection.lock().await;
        

        let output_stream_buffer = Arc::clone(&connection.buffer);
        let error_stream_is_alive = Arc::clone(&connection.is_alive);


        let stream = connection
            .output_device
            .build_output_stream(
                &connection.config,
                move |data: &mut [f32], _| {
                    
                    
                    let mut buffer = output_stream_buffer.blocking_lock();
                    

                    match buffer.next() {
                        Some(b) => {
                            for (b, d) in b.iter().zip(data.iter_mut()) {
                                *d = b.to_owned();
                            } 
                        },
                        None => {eprintln!("empty buffer!")}
                    }
                },
                move |e| {

                    
                    *error_stream_is_alive.blocking_lock() = false;
                    

                    eprintln!("error in stream due to {e}");
                },
            Some(Duration::from_secs(5)),
            )
            .expect("could not create server output stream!");

        stream.play().expect("could not start server stream!");

        connection.stream = Some(MetalStream { stream });


    }
}


pub struct MetalServer {
    connections: Arc<Mutex<Vec<Arc<Mutex<Connection>>>>>,
    address: String
}

impl Default for MetalServer {
    fn default() -> Self {
        Self { connections: Default::default(), address: "0.0.0.0:8000".to_owned() }
    }
}

impl MetalServer {
    pub fn new(address: &str) -> Self {
        MetalServer {
            connections: Arc::new(Mutex::new(vec![])),
            address: address.to_owned()
        }
    }
}

pub struct BatchBuffer<T> {
    batch_size: usize,
    buffer: Vec<T>
}

impl<T: Clone> BatchBuffer<T> {
    pub fn new(batch_size: usize) -> BatchBuffer<T> {
        BatchBuffer {
            batch_size,
            buffer: vec![]
        }
    }

    pub fn next(&mut self) -> Option<Vec<T>> {
        
        let removed = self.buffer.get(0..self.batch_size)?.to_owned();

        self.buffer.reverse();
        self.buffer.truncate(self.batch_size);
        self.buffer.reverse();

        if removed.is_empty() {
            return None;
        } else {
            return Some(removed);
        }
    }

    pub fn extend(&mut self, iter: &mut dyn Iterator<Item = T>) {
        self.buffer.extend(iter);
    }

    pub fn push(&mut self, to_push: T) {
        self.buffer.push(to_push);
    }
}

unsafe impl std::marker::Send for Connection {}


pub struct Connection {
    client: Client,
    output_device: cpal::Device,
    buffer: Arc<Mutex<BatchBuffer<f32>>>,
    stream: Option<MetalStream>,
    config: StreamConfig,
    websocket: WebSocketStream<TcpStream>,
    is_alive: Arc<Mutex<bool>>
}

impl Drop for Connection {
    fn drop(&mut self) {
        println!("dropping connection");

        match &self.stream {
            Some(stream) => {
                let _ = stream.stream.pause();
            },
            None => {

            }
        }

        let _ = self.websocket.close(None);
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    pub url: String,

}

impl Connection {
    pub async fn new(address: &str, tcp_stream: TcpStream) -> Result<Arc<Mutex<Connection>>, Box<dyn std::error::Error>> {

        let output_device = cpal::default_host().default_output_device().expect("could not find default input device!");
        let client = Client {
            url: address.to_owned()
        };

        let mut websocket = 
            accept_async(tcp_stream)
            .await
            .expect("could not establish websocket stream!");

        let config_message = 
            websocket
            .next()
            .await
            .expect("could not get config message!").expect("error getting config message!")
            .into_data();

        let bin_config = match bincode::deserialize(&config_message).expect("could not deserialize config!") {
            crate::BinMessages::BinConfig(config) => config,
            _ => panic!("unexpected message between client and server!")
        };

        let config = StreamConfig {
            buffer_size: cpal::BufferSize::Fixed(bin_config.buffer_size), 
            channels: bin_config.channels, 
            sample_rate: cpal::SampleRate(bin_config.sample_rate)
        };

        let connection = Arc::new(Mutex::new(Connection {
            client,
            output_device,
            websocket,
            buffer: Arc::new(Mutex::new(BatchBuffer::new(BUFFER_BATCH_SIZE))),
            stream: None,
            config: config,
            is_alive: Arc::new(Mutex::new(true)),
        }));

        MetalStream::new(&connection).await;

        return Ok(connection);
    }
}

#[async_trait]
impl Server for MetalServer {
    // spawn some task that runs the server connection in background
    async fn bind(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.address.clone()).await.unwrap();
        println!("Listening on: {}", self.address);
        
        let connections = &mut self.connections;
        let listener_connections = Arc::clone(&connections);
        let liveness_connections = Arc::clone(&connections);

        //async tasks
        tokio::spawn(async move {
            // remove dead connections
            tokio::spawn(async move {
                loop {
                    
                    let mut ul_connections = liveness_connections.lock().await;
                    

                    
                    let ul_connections_ul = Concurrent::lock_all_ordered(
                        ul_connections
                        .iter()
                        .map(|connection| connection.lock())
                    )
                    .await;
                    
                    

                    let mut to_keep: Vec<bool> = vec![];

                    for mut connection in ul_connections_ul {
                        
                        if *connection.is_alive.lock().await {
                            to_keep.push(true);
                        } else {
                            to_keep.push(false);
                            connection.stream = None;
                        }
                    }

                    // actual removal
                    let mut keeper_iter = to_keep.iter();

                    ul_connections.retain(|_| *keeper_iter.next().expect("error with iterator this should not happen!"));
                }
            });
            // manage connections
            loop {                
                

                let tcp_stream = match listener.accept().await {
                    Ok((tcp_stream, _)) => tcp_stream,
                    Err(e) => {
                        eprintln!("{}", e);
                        continue;
                    }
                };

                
                let mut ul_connections = listener_connections.lock().await;
                

                let name = tcp_stream.peer_addr().unwrap().to_string();
                
                let connection = Connection::new(&name, tcp_stream).await.expect("could not create connection!");
                let buffer_connection = Arc::clone(&connection);

                //write data from websocket to buffer
                tokio::spawn(async move {
                    loop {
                        
                        let mut ul_connection = buffer_connection.lock().await;
                        

                        match ul_connection.websocket.next().now_or_never() {
                            Some(Some(Ok(websocket_message))) => {

                                let deserialized = match bincode::deserialize(&websocket_message.into_data()) {
                                    Ok(deserialized) => deserialized,
                                    Err(e) => {
                                        eprintln!("could not deserialize client to server message on server due to {}", e);
                                        continue;
                                    }
                                };



                                match deserialized {
                                    crate::BinMessages::BinData(data) => {
                                        ul_connection.buffer.lock().await.extend(&mut data.into_iter());
                                    },
                                    crate::BinMessages::BinConfig(config) => {
                                        println!("received config!");
                                        let config = cpal::StreamConfig {
                                            channels: config.channels,
                                            sample_rate: cpal::SampleRate(config.sample_rate),
                                            buffer_size: cpal::BufferSize::Fixed(config.buffer_size)
                                        };

                                        ul_connection.config = config;
                                    },
                                    _ => todo!("have not implemented this part of communication deserialization yet!")
                                };

                                

                            },
                            Some(Some(Err(e))) => {
                                
                                *ul_connection.is_alive.lock().await = false;
                                

                                eprintln!("error writing server output due to {}", e);
                                return;
                            },
                            Some(None) => {
                                
                                
                                *ul_connection.is_alive.lock().await = false;
                                

                                println!("Received None, stream is dead");
                                return;
                            },
                            None => {
                                continue;
                            }
                        }
                    }
                });



                
                
                ul_connections.push(connection);
            }
        });
        

        Ok(())
    }

    async fn list_clients(&self) -> Result<Vec<Client>, Box<dyn std::error::Error>> {
        
        let connections = self.connections.lock().await;
        let ret = Ok(Concurrent::lock_all_ordered(
            connections
            .iter()
            .map(
                |connection| connection.lock()
            )
        ).await
        .iter()
        .map(
            |connection| connection.client.clone()
        ).collect::<Vec<Client>>());
        

        return ret;
    }

    async fn disconnect_client(&mut self, url: &str) -> Result<Option<Client>, Box<dyn std::error::Error>> {
        
        let mut ul_connections = self.connections.lock().await;
        

        
        let clients = 
            Concurrent::lock_all_ordered(
                ul_connections
                .iter()
                .map(|connection| connection.lock())
            ).await
            .iter()
            .map(|connection| connection.client.clone())
            .enumerate()
            .collect::<Vec<(usize, Client)>>();
        

        match clients.iter().find(|(_, client)| client.url == url) {
            Some((i, client)) => {
        
                let removed = ul_connections.remove(i.to_owned());
                
                match &removed.lock().await.stream {
                    Some(stream) => {let _ = stream.stream.pause();},
                    None => {}
                }
                

                return Ok(Some(client.clone()));
            },
            None => {}
        }

        return Ok(None);
        
        

        
    }

    async fn change_output_device(&mut self, cpos: usize,  new_output: cpal::Device) -> Result<(), Box<dyn std::error::Error>> {
        let new_name = new_output.name().expect("could not get device name!");

        
        let mut ul_connections = self.connections.lock().await;
        let rx_connection = ul_connections.get_mut(cpos).expect("could not get connection in change_output_device!");

        {
            let mut connection = rx_connection.lock().await;
            
            
            match &connection.stream {
                Some(stream) => {let _ = stream.stream.pause();}
                None => {}
            };

            connection.stream = None;


            //"clone" device
            let cloned_output = cpal::default_host()
                .output_devices()
                .expect("could not get output devices!")
                .filter(
                    |device| device.name().expect("could not get device name!") == new_name
                ).next()
                .expect("could not find output device!");
            
            
            connection.output_device = cloned_output;
        }
        
        MetalStream::new(&rx_connection).await;

        Ok(())
    }
}

