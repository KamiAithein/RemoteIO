use std::fmt::Display;
use std::time::Duration;
use std::{convert::TryInto, slice::from_mut};

use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use async_trait::async_trait;
use tokio::sync::Mutex;
use cpal::StreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures::sink::Send;


use futures::{StreamExt, SinkExt};



#[async_trait]
pub trait Client {
    async fn connect(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn change_source_device(&mut self, new_device: cpal::Device) -> Result<(), Box<dyn std::error::Error>>;
    async fn is_alive(&mut self) -> bool;
    async fn name(&mut self) -> String;
    
}

pub struct Connection {
    url: String,
    writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}
pub struct Batch<T> {
    pub repr: Vec<T>
}

pub struct Metal2RemoteStream {
    stream: cpal::Stream,
    liveness: Arc<AtomicBool>,
}

pub struct Metal2RemoteClient {
    connection: Option<Arc<Mutex<Connection>>>,
    stream: Option<Metal2RemoteStream>,
    device: cpal::Device
}

unsafe impl std::marker::Send for Metal2RemoteClient {}
unsafe impl std::marker::Send for Connection {}

impl Metal2RemoteClient {
    pub fn new(device: cpal::Device) -> Self {
        Self {
            connection: None,
            stream: None,
            device,
        }
    }
}

struct ClientHelper {}

impl ClientHelper {
    fn build_input_stream(device: &cpal::Device, config: cpal::SupportedStreamConfig, input_stream_connection: Arc<Mutex<Connection>>, stream_liveness: Arc<AtomicBool>) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    let bytes = data.iter().flat_map(|f| f.to_be_bytes()).collect::<Vec<u8>>();
                    
                    let runtime = tokio::runtime::Runtime::new().expect("could not create runtime!");
                    
                    runtime.block_on(async {
                        let mut connection = input_stream_connection.lock().await;

                        match connection.writer.send(Message::binary(bytes)).await {
                            Ok(_) => {},
                            Err(e) => {
                                stream_liveness.store(false, Ordering::Relaxed);
                            },
                        } 
                    })
                },
                |err| eprintln!("errored in input stream {}", err),
                Some(Duration::from_secs(5))
            ).unwrap();

        Ok(stream)
    }
}

#[async_trait]
impl Client for Metal2RemoteClient {
    async fn change_source_device(&mut self, new_device: cpal::Device) -> Result<(), Box<dyn std::error::Error>> {
        self.device = new_device;

        let config = self.device.default_input_config().expect("could not get default input config!");

        let input_stream_connection = Arc::clone(self.connection.as_ref().unwrap());

        let liveness: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
        let stream_liveness: Arc<AtomicBool> = Arc::clone(&liveness);

        let stream = ClientHelper::build_input_stream(&self.device, config, input_stream_connection, stream_liveness).expect("could not build input stream!");

        self.stream = Some(Metal2RemoteStream {
            stream,
            liveness
        });

        Ok(())
    }




    async fn connect(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
       
        // first establish connection
        let (socket, resp) = connect_async(url).await?;
        let (mut writer, reader) = socket.split();

        //then send config
        let config = self.device.default_input_config().expect("could not get default input device!");


        let bin_config_struct = crate::BinStreamConfig {
            channels:  config.channels(),
            sample_rate: config.sample_rate().0,
            buffer_size: 4096
        };
    
        let bin_config = bincode::serialize(&crate::BinMessages::BinConfig(bin_config_struct))?;
    
        writer.send(Message::binary(bin_config)).await.expect("error sending config!");

        // remember to store connection in self
        let mut connection = Connection {
            url: url.to_owned(),
            writer,
            reader
        };

        let self_connection = Arc::new(Mutex::new(connection));
        let input_stream_connection = Arc::clone(&self_connection);

        self.connection = Some(self_connection);

        let liveness: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
        let stream_liveness: Arc<AtomicBool> = Arc::clone(&liveness);

        //then set up stream
        let stream = ClientHelper::build_input_stream(&self.device, config, input_stream_connection, stream_liveness).expect("could not build input stream!");

        self.stream = Some(Metal2RemoteStream {
            stream,
            liveness
        });

        Ok(())
    }

    async fn is_alive(&mut self) -> bool {

        match &self.stream {
            Some(stream) => stream.liveness.load(Ordering::Relaxed),
            None => false

        }
    }

    async fn name(&mut self) -> String {
        let device_name = match self.device.name() {
            Ok(name) => name,
            Err(e) => "N/A".to_owned()
        };

        let url = match &self.connection {
            Some(connection) => connection.lock().await.url.clone(),
            None => "N/A".to_owned()
        };

        format!("{device_name}:{url}")
    }
}