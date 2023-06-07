use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex, MutexGuard};

use cpal::traits::*;

const BATCH_SIZE: usize = 512;
const BUFFER_SIZE: usize = 2048;

struct Batch<T> {
    repr: Arc<Mutex<Vec<T>>>,
    batch_size: usize
}

impl<T: Clone + Sync + Send> Batch<T> {
    fn new(batch_size: usize) -> Batch<T> {
        Batch {
            batch_size,
            repr: Arc::new(Mutex::new(Vec::new())) 
        }
    }
    fn fork(&self) -> Batch<T> {
        let cloned_repr = Arc::clone(&self.repr);

        Batch {
            repr: cloned_repr,
            batch_size: self.batch_size
        }
    }
    fn next(&mut self) -> Vec<T> {
        let mut ul_repr = self.repr.lock().expect("could not get mutex!");

        let batch = match ul_repr.get(0..self.batch_size) {
            Some(batch) => batch.to_vec(),
            None => vec![]
        };

        if !batch.is_empty() {
            ul_repr.drain(0..self.batch_size);
        }

        return batch;
    }
    fn put(&mut self, to_put: impl Iterator<Item=T>) {
        let mut ul_repr = self.repr.lock().expect("could not lock mutex!");

        ul_repr.extend(to_put);
    }
}

struct ServerHelper {}

impl ServerHelper {
    pub fn establish_audio_link(device: &cpal::Device, config: Option<cpal::StreamConfig>, alias: &str, streams: &mut HashMap<String, Stream>, batches: &mut HashMap<String, Batch<f32>>) {
        let config = match config {
            Some(config) => config,
            None => device.default_output_config().expect("could not get output config!").into()
        };

        let batch = Batch::<f32>::new(BATCH_SIZE);
        let mut stream_batch = batch.fork();

        let stream = device.build_output_stream(
            &config, 
            move |data: &mut [f32], _| {
                for (d, b) in data.iter_mut().zip(stream_batch.next().iter()) {
                    *d = b.to_owned();
                }
            }, 
            |_| {}, 
            None
        ).expect("could not build output stream!");

        stream.play().expect("could not play stream!");

        streams.insert(alias.to_owned(), Stream::from(stream));
        batches.insert(alias.to_owned(), batch);
    }
}

pub trait SyncServer {
    fn list_clients(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn disconnect_client(&mut self, url: &str) -> Result<Option<String>, Box<dyn std::error::Error>>;
    // fn change_output_device(&mut self, cpos: usize, new_output: cpal::Device) -> Result<(), Box<dyn std::error::Error>>;
    fn new(addr: &str, device: Option<cpal::Device>) -> Result<Server, Box<dyn std::error::Error>>;
}

pub struct Stream {
    stream: cpal::Stream
}

unsafe impl Send for Stream {}

impl Stream {
    pub fn from(stream: cpal::Stream) -> Stream {
        return Stream {
            stream
        }
    }
}

pub struct Server {
    socket: Arc<Mutex<UdpSocket>>,
    streams: Arc<Mutex<HashMap<String, Stream>>>,
    batches: Arc<Mutex<HashMap<String, Batch<f32>>>>,
    device: Option<Arc<cpal::Device>>
}

impl Server {
    
}
impl SyncServer for Server {
    fn new(addr: &str, mut device: Option<cpal::Device>) -> Result<Server, Box<dyn std::error::Error>> {
        let socket = Arc::new(Mutex::new(UdpSocket::bind("localhost:9000").expect("could not bind to udp socket!")));

        let streams = Arc::new(Mutex::new(HashMap::<String, Stream>::new()));
        let batches = Arc::new(Mutex::new(HashMap::<String, Batch<f32>>::new()));

        let device = Arc::new(match device {
            Some(d) => d,
            //auto initialize if no device given
            None => cpal::default_host().default_output_device().expect("could not get default ouput device!")
        });

        let s_streams = Arc::clone(&streams);
        let s_batches = Arc::clone(&batches);
        let s_device = Arc::clone(&device);
        let s_socket = Arc::clone(&socket);

        //new thread server main loop here
        std::thread::spawn(move || {
            loop {
                let mut buf = ['\0' as u8 ; BUFFER_SIZE];
                
                let recv = {
                    let ul_socket = socket.lock().expect("could not lock socket!");
                    ul_socket.recv(&mut buf).expect("could not recv")
                };

                let buf = &buf[..recv];
                
                match bincode::deserialize(&buf).expect("could not deserialize!") {
                    remoteio_backend::BinMessages::BinConfig(remoteio_backend::AliasedData{alias, data: config}) => {
                        println!("config! {}, {}, {}", config.channels, config.sample_rate, config.buffer_size);
                        let alias = alias.alias;
                        println!("config is for alias {alias}");
                        let config = cpal::StreamConfig {
                            buffer_size: cpal::BufferSize::Default, 
                            channels: config.channels, 
                            sample_rate: cpal::SampleRate(config.sample_rate)
                        };
        
                        // we have a new config from a new device from an old client. Change the config on the appropriate stream
                        // first delete the old stream
                        let mut ul_streams = streams.lock().expect("could not lock streams!");
                        let mut ul_batches = batches.lock().expect("could not lock batches!");
                        match ul_streams.remove(&alias) {
                            None => todo!("got config for uninitialized stream!"),
                            Some(_) => {}
                        }
        
                        match ul_batches.remove(&alias) {
                            
                            None => {
                                println!("still alias as {alias}");
                                todo!("couldn't remove stream!");
                            },
                            Some(_) => {}
                        }
        
                        ServerHelper::establish_audio_link(&device, Some(config), &alias, &mut ul_streams, &mut ul_batches);
        
                    },
                    remoteio_backend::BinMessages::BinData(remoteio_backend::AliasedData{alias, data}) => {
                        let mut ul_batches = batches.lock().expect("could not lock batches!");


                        let alias = alias.alias;
                        let batch = ul_batches.get_mut(&alias).expect("could not find batch!");
                        println!("data: {:?}", &data[..3]);
                        batch.put(data.clone().into_iter());
                    },
                    remoteio_backend::BinMessages::BinHello(remoteio_backend::BinStreamAlias{alias}) => {
                        println!("hello message! {}", alias);
                        let mut ul_batches = batches.lock().expect("could not lock batches!");
                        let mut ul_streams = streams.lock().expect("could not lock streams!");


        
                        if ul_streams.contains_key(&alias) { //we already are connected to this client!
                            // just delete the old client before we reinitialize
                            ul_streams.remove(&alias);
                            ul_batches.remove(&alias);
                        }
        
                        ServerHelper::establish_audio_link(&device, None, &alias, &mut ul_streams, &mut ul_batches);
                    }
                    _ => todo!("unimplemented!")
                };
        
            
            }
    
        });
        
        return Ok(Server {
            socket: s_socket,
            device: Some(s_device),
            streams: s_streams,
            batches: s_batches,
        })
    }

    fn list_clients(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let ul_streams = self.streams.lock().expect("could not lock on streams!");

        let clients = ul_streams.keys().map(|k|k.to_owned()).collect::<Vec<String>>();

        return Ok(clients);
    }

    fn disconnect_client(&mut self, alias: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let (mut ul_streams, mut ul_batches) = (self.streams.lock().expect("could not lock streams!"), self.batches.lock().expect("could not lock batches!"));
        if ul_streams.contains_key(alias) {
            ul_streams.remove(alias);
            ul_batches.remove(alias);
            return Ok(Some(alias.to_owned()));
        }

        return Ok(None);
    }

    // fn change_output_device(&mut self, cpos: usize, new_output: cpal::Device) -> Result<(), Box<dyn std::error::Error>> {
        
    // }
}


//basiclaly
// the client is gonna establish a connection with "hello"
// then every message will be labeled by who it's from
// the server needs to read who the message is from and to and then 
//  distribute the message properly
fn main() {

    let stream = Server::new("localhost:9000", None).expect("could not create server!");
    loop {}

    
}