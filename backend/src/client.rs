use std::{sync::{Arc, Mutex}, net::{UdpSocket, Ipv4Addr, Ipv6Addr}};

use cpal::{traits::*, SupportedBufferSize};

pub trait SyncClient {
    fn connect(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn change_source_device(&mut self, new_device: cpal::Device) -> Result<(), Box<dyn std::error::Error>>;
    fn is_alive(&mut self) -> bool;
    fn name(&mut self) -> String;
}

pub struct Client {
    name: String,
    is_alive: Arc<Mutex<bool>>,
    connection: Option<Arc<Mutex<Connection>>>,
    device: Option<cpal::Device>
}

impl Client {
    pub fn new(name: &str, device: cpal::Device) -> Client {
        Client {
            name: name.to_owned(),
            is_alive: Arc::new(Mutex::new(false)),
            connection: None,
            device: Some(device)
        }
    }

    fn establish_audio_link(&mut self, alias: String, config: cpal::SupportedStreamConfig, stream_connection: Arc<Mutex<Connection>>) -> cpal::Stream {
        let stream_name = alias.clone();
        let stream = self.device.as_ref().expect("no audio device selected at connect!").build_input_stream(
            &config.into(), 
            move |data: &[f32], _| {
        
                let data_chunks = data.chunks(128).map(|chunk| chunk.to_owned());
                for data in data_chunks {
                    let to_send = crate::BinMessages::BinData(crate::AliasedData{alias: crate::BinStreamAlias{alias: stream_name.clone()}, data: data.to_vec()});
                    let to_send_bin = bincode::serialize(&to_send).expect("could not serialize audio data!");
    
                    let ul_connection = stream_connection.lock().expect("could not lock stream connection!");
                    // println!("sending audio data! {:?}", data);
                    ul_connection.socket.send(&to_send_bin).expect("could not send audio data!");
                    // println!("sent!");
                }
            }, 
            |_| {
                println!("error!");
            }, 
            None
        ).expect("could not start input stream!");

        stream.play().expect("could not start stream");
        stream
    }
}

struct Connection {
    socket: UdpSocket,
    send_endpoint: String,
    stream: Option<Stream>
}

pub struct Stream {
    stream: cpal::Stream
}

unsafe impl Send for Stream {}

impl SyncClient for Client {
    // must have set device already!
    fn connect(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.connection = None;

        let socket = UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0))?;
        println!("connected socket at {}", socket.local_addr().expect("couldnt get local addr"));
        println!("trying to cnonect to {}", url);
        socket.connect(url).expect("could not connect to server from client!");

        {
            let mut is_alive = self.is_alive.lock().expect(&format!("{}, could not lock is_alive on line", line!()));
            *is_alive = true;
        }
        
        let connection = Arc::new(Mutex::new(Connection {
            socket,
            send_endpoint: url.to_owned(),
            stream: None
        }));

        self.connection = Some(Arc::clone(&connection));
        
        let alias = self.name();

        {
            let ul_connection = connection.lock().expect("could not get lock on connection!");
            let bin_hello = bincode::serialize(&crate::BinMessages::BinHello(crate::BinStreamAlias { alias: alias.clone() })).expect("could not serialize hello message!");
            ul_connection.socket.send(&bin_hello).expect("could not send hello!");

        }
        println!("dad");
        

        //send first config!
        let config = self.device.as_ref().expect("tried to connect with no device set").default_input_config().expect("no input config!");

        let buffer_size = match config.buffer_size() {
            SupportedBufferSize::Range { min, max } => max.to_owned(),
            SupportedBufferSize::Unknown => 4096
        };

        let bin_config_struct = crate::BinStreamConfig {
            channels: config.channels(),
            sample_rate: config.sample_rate().0,
            buffer_size: buffer_size
        };  

        let bin_config = bincode::serialize(&crate::BinMessages::BinConfig(crate::AliasedData{alias: crate::BinStreamAlias{alias: alias.clone()}, data: bin_config_struct})).expect("could not deserialize!");

        {
            let ul_connection = connection.lock().expect("could not get lock on connection!");
            ul_connection.socket.send(&bin_config).expect("failed to send binconfig!");
            // sent!
        }

        // neet a mutex here for next step
        let stream_connection = Arc::clone(&connection);

        // start to send audio!
        let stream = self.establish_audio_link(alias, config, stream_connection);

        {
            let mut ul_connection = connection.lock().expect("could not get lock for connection!");
            ul_connection.stream = Some(Stream{stream});
        }
        // started!

        



        // let mut buf = [0; 10];
        // let (amt, src) = socket.recv_from(&mut buf)?;

        // // Redeclare `buf` as slice of the received data and send reverse data back to origin.
        // let buf = &mut buf[..amt];

        println!("connected!");
        Ok(())
    }

    // must be connected
    fn change_source_device(&mut self, new_device: cpal::Device) -> Result<(), Box<dyn std::error::Error>> {

        {
            let mut ul_connection = self.connection.as_mut().unwrap().lock().expect("could not unlock connection");
            ul_connection.stream = None;
        }
        let stream_connection = Arc::clone(self.connection.as_ref().unwrap());
        
        let alias = self.name();
        let stream = self.establish_audio_link(alias, new_device.default_input_config().expect("could not get default input config"), stream_connection);
        
        {
            let mut ul_connection = self.connection.as_mut().unwrap().lock().expect("could not unlock connection");
            ul_connection.stream = Some(Stream{stream});
            self.device = Some(new_device);
        }

        Ok(())

    }

    fn is_alive(&mut self) -> bool {
        let ul_alive = self.is_alive.lock().expect("could not lock client is_alive");
        return *ul_alive;
    }

    // this locks on connection
    fn name(&mut self) -> String {
        let name = match &self.connection {
            Some(connection) => {
                let ul_connection = connection.lock().expect("could not lock connection!");
                let addr = ul_connection.socket.local_addr().expect("could not get peer addr of socket").to_string();
                format!("{}:{}", addr, self.name)
            }
            None => self.name.clone()
        };

        return name;
    }
}

fn main() {
    let mut client = Client::new("client", cpal::default_host().default_input_device().expect("could not get default input device"));
    let _ = client.connect("localhost:9000");
    std::thread::sleep(std::time::Duration::from_secs(3));
}