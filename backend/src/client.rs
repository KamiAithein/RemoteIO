use std::fmt::Display;
use std::time::Duration;
use std::{convert::TryInto, slice::from_mut};

use std::sync::{Arc};
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

struct Connection {
    url: String,
    writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    config: cpal::SupportedStreamConfig
}

impl Connection {
    async fn connect_to_server(url: &str, config: cpal::SupportedStreamConfig) -> Result<(Connection, Response), Box<dyn std::error::Error>> {
        println!("connecting to server {}", url);
        let (socket, resp) = connect_async(url).await?;
        println!("got connection!");
        let (writer, reader) = socket.split();
        
        let connection = Connection {
         url: url.to_owned(),
         writer,
         reader,
         config
        };

     
        return Ok((connection, resp));
     }


     async fn send_client_config(connection: &mut Connection) -> Result<(), Box<dyn std::error::Error>> {
        let bin_config_struct = crate::BinStreamConfig {
            channels:  connection.config.channels(),
            sample_rate: connection.config.sample_rate().0,
            buffer_size: 4096
        };
    
        let bin_config = bincode::serialize(&crate::BinMessages::BinConfig(bin_config_struct))?;
    
        connection.writer.send(Message::binary(bin_config)).await.expect("error sending config!");
    
        Ok(())
    }

    pub async fn new(url: &str, config: cpal::SupportedStreamConfig) -> Result<Connection, Box<dyn std::error::Error>> {
        let (mut connection, _response) = Connection::connect_to_server(url, config).await?;

        Connection::send_client_config(&mut connection).await?;

        Ok(connection)
    }
}
#[derive(Clone)]
pub enum AudioLinkStatus {
    Alive,
    Dead
}
struct AudioLink {
    stream: cpal::Stream,
    reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    status: Arc<Mutex<AudioLinkStatus>>
}

impl AudioLink {
    async fn create_remote_audio_link(connection: Connection, input_device: &cpal::Device) -> Result<AudioLink, Box<dyn std::error::Error>> {
        let Connection { url, mut writer, reader, config } = connection;
        let status = Arc::new(Mutex::new(AudioLinkStatus::Alive));
        let status_stream = Arc::clone(&status);

        let stream = input_device
            .build_input_stream(&config.into(), move |data: &[f32], _| {
                let bytes = data.iter().flat_map(|f| f.to_be_bytes()).collect::<Vec<u8>>();
    
                let runtime = tokio::runtime::Runtime::new().expect("Unable to create a runtime");
    
                runtime.block_on(async {
                    match writer.send(Message::binary(bytes)).await {
                        Ok(msg) => {},
                        Err(e) => {
                            let mut ul_status_stream = status_stream.lock().await;
                            *ul_status_stream = AudioLinkStatus::Dead;
                        }
                    }
                });
           
            }, |err| eprintln!("audio stream error: {}", err),
            Some(Duration::from_secs(5)))
            .unwrap();
    
        return Ok(AudioLink {
            stream,
            reader,
            status
        });
    }

    pub async fn new(connection: Connection, input_device: &cpal::Device) -> Result<AudioLink, Box<dyn std::error::Error>> {
        return AudioLink::create_remote_audio_link(connection, input_device).await;
    }
}



async fn obtain_input_device() -> Result<(cpal::SupportedStreamConfig, cpal::Device), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let input_device = host.default_input_device().expect("couldn't find default input device!");
    let config = input_device.default_input_config()?;

    return Ok((config, input_device));
}

 
pub struct Client {
    audio_link: AudioLink,
    pub name: String,
    pub url: String
}

impl Drop for Client {
    fn drop(&mut self) {
        println!("dropping {}", self.name);
        let _ = self.audio_link.stream.pause();
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Client({})", self.name)
    }
}

// my first step into unsafe. Fuck.
unsafe impl core::marker::Send for Client {}

impl Client {
    pub async fn new_with(url: &str, device: &cpal::Device, config: cpal::SupportedStreamConfig) -> Result<Client, Box<dyn std::error::Error>> {
        // get connection
        let connection = Connection::new(url.clone(), config).await?;

        // create stream where every float is send to the server
        let audio_link = AudioLink::new(connection, device).await?;

        audio_link.stream.play().unwrap(); 

        return Ok(Client {
            name: url.to_owned(),
            url: url.to_owned(),
            audio_link,
        });
    }
    pub async fn new(url: &str) -> Result<Client, Box<dyn std::error::Error>> {
        let (config, input_device) = obtain_input_device().await?;

        return Client::new_with(url, &input_device, config).await;
    }

    pub async fn change_device(&mut self, device: &cpal::Device, config: cpal::SupportedStreamConfig) {

        *self = Client::new_with(&self.url, device, config).await.expect("could not change device!");
    }

    pub fn get_devices(&self) -> Result<Vec<(cpal::SupportedStreamConfig, cpal::Device)>, Box<dyn std::error::Error>> {

        Ok(cpal::default_host()
            .input_devices()
            .expect("could not get input devices")
            .map(|device| (device.default_input_config().expect("could not get default input config!"), device))
            .collect::<Vec<(cpal::SupportedStreamConfig, cpal::Device)>>()
        )
    }

    pub async fn next_message(&mut self) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        match self.audio_link.reader.next().await {
            None => Ok(None),
            Some(res) => 
                match res {
                    Ok(msg) => Ok(Some(msg)),
                    Err(e) => panic!("when trying to get next message from audio_link, got: {}", e)
                }
        }
    }

    pub fn is_alive(&self) -> bool {
        let check = self.audio_link.status.blocking_lock();
        match *check {
            AudioLinkStatus::Alive => true,
            AudioLinkStatus::Dead => false
        }
    }
}




// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let config = &remoteio_shared::config;
    
//     let mut client = Client::new(&config.ws_endpoint).await?;

//     while let Ok(Some(msg)) = client.next_message().await {
//         println!("Received message: {}", msg);
//     }
    

//     Ok(())
// }
