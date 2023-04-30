use std::{convert::TryInto, slice::from_mut};

use std::sync::{Arc, Mutex};
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
        let (socket, resp) = connect_async(url).await?;
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
        let bin_config_struct = remoteio::BinStreamConfig {
            channels:  connection.config.channels(),
            sample_rate: connection.config.sample_rate().0,
            buffer_size: 4096
        };
    
        let bin_config = bincode::serialize(&bin_config_struct)?;
    
        connection.writer.send(Message::binary(bin_config)).await.expect("error sending config!");
    
        Ok(())
    }

    pub async fn new(url: &str, config: cpal::SupportedStreamConfig) -> Result<Connection, Box<dyn std::error::Error>> {
        let (mut connection, _response) = Connection::connect_to_server(url, config).await?;

        Connection::send_client_config(&mut connection).await?;

        Ok(connection)
    }
}

struct AudioLink {
    stream: cpal::Stream,
    reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>
}

impl AudioLink {
    async fn create_remote_audio_link(connection: Connection, input_device: &cpal::Device) -> Result<AudioLink, Box<dyn std::error::Error>> {
        let Connection { url, mut writer, reader, config } = connection;
    
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
    
        return Ok(AudioLink {
            stream,
            reader
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







#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    // first get config of this device
    let (config, input_device) = obtain_input_device().await?;

    // then get connection
    let url = "ws://127.0.0.1:8000";
    let connection = Connection::new(url, config).await?;

    // create stream where every float is send to the server
    let mut audio_link = AudioLink::new(connection, &input_device).await?;

    audio_link.stream.play().unwrap();

    while let Some(Ok(msg)) = audio_link.reader.next().await {
        println!("Received message: {}", msg);
    }

    Ok(())
}
