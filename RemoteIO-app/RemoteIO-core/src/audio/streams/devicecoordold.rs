use crate::audio::streams::device::{Device, DeviceFromStringOpts};
use crate::audio::streams::deviceswitch::{DeviceSwitch, DeviceSwitchOpts, Connection};

use cpal::Host;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use std::collections::HashMap;
use std::error::Error;

pub struct SwitchBoard {
    // input_devices: Vec<DeviceSwitch>,
    // output_devices: Vec<DeviceSwitch>,
    device_map: HashMap<DeviceSwitch, Option<DeviceSwitch>>
}

pub struct SwitchBoardOpts {
    host: cpal::Host,
}



impl SwitchBoard {
    fn filter_devices(host: &cpal::Host, filter: fn(&cpal::Device)->bool) -> Vec<cpal::Device> {
        let mut devices = Vec::<cpal::Device>::new();
        
        host.devices().expect("could not get devices!")
            .for_each(|device|{
                devices.push(device);
            });
        
        devices.drain_filter(|device| filter(device));

        return devices;
    }
    
    fn get_all_input_devices(host: &cpal::Host) -> Vec<cpal::Device> {
        SwitchBoard::filter_devices(host, |device| {
            let REMOVE = true;
            let KEEP = false;

            if device.default_input_config().is_err() { REMOVE }
            else { KEEP }
        })
    }

    fn get_all_output_devices(host: &cpal::Host) -> Vec<cpal::Device> {
        SwitchBoard::filter_devices(host, |device| {
            let REMOVE = true;
            let KEEP = false;

            if device.default_output_config().is_err() { REMOVE }
            else { KEEP }
        })
    }


    pub fn new_from(opts: SwitchBoardOpts) -> Self {
        let SwitchBoardOpts{host} = opts;
        
        let mut device_map = HashMap::new();

        SwitchBoard::get_all_input_devices(&host).into_iter().for_each(|device| {
            device_map.insert(
                DeviceSwitch::new_from(DeviceSwitchOpts {
                    device: Device::from(device)
                }),
                None
            );
        });

        Self {
            device_map
        }
    }
}

impl SwitchBoard{

    /// Establish a connection between two DeviceSwitches
    ///     On success the audio from mapping.0 will be forwarded to mapping.1
    pub fn connect(&mut self, mapping: (Device, Device)) -> Result<(), Box<dyn Error>> { //@TODO actual error?
        let (from, to) = mapping;
    
        let producer: &'switch mut DeviceSwitch = SwitchBoard::find_device_switch_from(&mut self.input_devices, from)
            .expect("could not find device switch!");
        let consumer: &'switch mut DeviceSwitch = SwitchBoard::find_device_switch_from(&mut self.output_devices, to)
            .expect("could not find device switch!");
        
        let disconnect_if_conn = |device_switch: &mut DeviceSwitch| {
            match &mut device_switch.connection {
                Some(conn) => {
                   todo!("@TODO connect a connected device!");
                },
                _ => { } //nothing we good!
            }
        };
        
        disconnect_if_conn(producer);
        disconnect_if_conn(consumer);
    
        let (producer_conn, consumer_conn) = SwitchBoard::establish_connection((&producer, &consumer)).expect("could not establish connection!");
    
        producer_conn.stream.play().expect("could not start producer stream!");
        consumer_conn.stream.play().expect("could not start consumer stream!");
    
        producer.connection = Some(producer_conn);
        consumer.connection = Some(consumer_conn);
    
        self.device_map.insert(producer, consumer);
    
        Ok(())
    }
    
    /// Finds first DeviceSwitch from device_list with matching Device (via PartialEq)
    fn find_device_switch_from(device_map: &mut HashMap<DeviceSwitch, Option<DeviceSwitch>>, device: Device) -> Result<&&mut DeviceSwitch, Box<dyn Error>> {
        let matches = device_map.iter_mut().flat_map(|(k, v_opt)| match v_opt {
            Some(v) => vec![k, v],
            None => vec![k]
        })
            .collect::<Vec<&mut DeviceSwitch>>()
            .iter()
            .filter(|item|item.device == device);

        return Ok(matches.last().unwrap_or_else(||panic!("could not find DeviceSwitch from Device!")));
    }

    /// Establishes and returns connections between target_conn.0 -> target_conn.1
    /// Does not mutate target_conn values
    fn establish_connection(target_conn: (&'switch DeviceSwitch, &'switch DeviceSwitch)) -> Result<(Connection, Connection), Box<dyn Error>> {
        let (producer, consumer) = target_conn;

        let producer_config: cpal::StreamConfig = producer.device.default_input_config().expect("could not get default input config!").into();
        let consumer_config: cpal::StreamConfig = consumer.device.default_output_config().expect("could not get default output config!").into();

        let latency_frames = (150. / 1_000.0) * producer_config.sample_rate.0 as f32; //@TODO make variable
        let latency_samples = latency_frames as usize * producer_config.channels as usize;

        let (mut producer_buf, mut consumer_buf) = ringbuf::RingBuffer::<f32>::new(latency_samples * 2).split();

        let output_data_fn = move |data: &mut [f32], cb: &cpal:: OutputCallbackInfo| {
            SwitchBoard::play_output(&mut consumer_buf, data, cb)
        };
        let input_data_fn = move |data: &[f32], cb: &cpal::InputCallbackInfo| {
            SwitchBoard::listen_input(&mut producer_buf, data, cb);
        };

        //@TODO if connected then disconnect!
        let producer_stream = producer.device.build_input_stream(&producer_config, input_data_fn, SwitchBoard::err_fn).expect("could not build input stream!");
        let consumer_stream = consumer.device.build_output_stream(&consumer_config, output_data_fn, SwitchBoard::err_fn).expect("could not build output stream!");

        assert!(producer.connection.is_none(), "connection must be disconnected at this point!");
        assert!(consumer.connection.is_none(), "connection must be disconnected at this point!");

        let producer_connection = Connection {
            config: producer_config,
            stream: producer_stream,
        };

        let consumer_connection = Connection {
            config: consumer_config,
            stream: consumer_stream,
        };

        Ok((consumer_connection, producer_connection))

    }


    fn play_output(consumer: &mut ringbuf::Consumer<f32>, data: &mut [f32], _: &cpal::OutputCallbackInfo) {
        let mut input_fell_behind = false;
        for sample in data {
            // *sample = 10_f32
            *sample = match consumer.pop() {
                Some(s) => s,
                None => {
                    input_fell_behind = true;
                    0.0
                }
            };
        }
        if input_fell_behind {
            eprintln!("input stream fell behind: try increasing latency");
        }
    }
    
    fn listen_input(producer: &mut ringbuf::Producer<f32>, data: &[f32], cb: &cpal::InputCallbackInfo) {
        let mut output_fell_behind = false;
        for &sample in data {
            if producer.push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            eprintln!("output stream fell behind: try increasing latency");
        }
    }
    
    fn err_fn(err: cpal::StreamError) {
        eprintln!("an error occurred on stream: {}", err);
    }
}



pub struct DeviceCoordinator<'switch> {
    pub host: Host,
    /// switch_board = fn(producer) -> consumer
    pub switch_board: SwitchBoard<'switch>
}

pub struct DeviceCoordinatorOpts<'switch> {
    pub host: Host,
    pub switch_board: SwitchBoard<'switch>
}

pub struct DeviceCoordinatorState<'device> {
    pub mapping: HashMap<&'device Device, &'device Device>
}

impl <'device> Default for DeviceCoordinatorState<'device> {
    fn default() -> Self {
        DeviceCoordinatorState {
            mapping: HashMap::new(),
        }
    }
}

impl <'switch> Default for DeviceCoordinator<'switch> {
    fn default() -> Self {
        Self::new_from(DeviceCoordinatorOpts::default())
    }
}

impl <'switch> Default for DeviceCoordinatorOpts<'switch> {
    fn default() -> Self {
        Self {
            host: cpal::default_host(),
            switch_board: SwitchBoard::new_from(SwitchBoardOpts{
                host: cpal::default_host()
            }),
        }
    }
}

impl <'switch> DeviceCoordinator<'switch> {
    pub fn new() -> Self {
        Self::default() //@TODO implement reasonable constructor
    }

    pub fn new_from(options: DeviceCoordinatorOpts<'switch>) -> Self {
        let DeviceCoordinatorOpts {host, switch_board} = options;
        Self {
            host,
            switch_board,
        }
    }

    pub fn connect(&'switch mut self, mapping: (Device, Device)) -> Result<(), Box<dyn Error>> {
        self.switch_board.connect(mapping)
    }

    pub fn devices(&self) -> Result<cpal::Devices, cpal::DevicesError> {
        return self.host.devices();
    }

    pub fn state(&'switch self) -> DeviceCoordinatorState {
        let mut map = HashMap::new();

        self.switch_board.device_map.iter().for_each(|forward| {
            let (source, dest) = forward;
            map.insert(&source.device, &dest.device);
        });

        DeviceCoordinatorState{
            mapping: map
        }
    }


    /// @TODO this relies HEAVILY on Device::try_from(&str)
    /// for all k, v in target
    /// for all u, w in self.switch_board
    ///     if k == u then v == w
    /// ;
    /// for all k in target
    ///     k in self.switch_board
    pub fn correct_state(&'switch mut self, target: HashMap<&str, &str>) -> Result<(), Box<dyn Error>> {
        // let mapping = &mut self.switch_board.device_map;

        for (from, to) in target.iter() {
            let handle_conversion = |to_convert: &&str| {
                let opts = DeviceFromStringOpts{host: &self.host, serial: to_convert};

                match Device::try_from(opts) {
                    Ok(device) => device,
                    Err(e) => panic!("error converting from string to device! caused by: {}", e)
                }
            };

            
            let from_device = handle_conversion(&from);
            let to_device = handle_conversion(&to);
            
            self.connect((from_device, to_device));
        }

        // mapping.drain_filter(|from_device, to_device| {
        //     let REMOVE = true; //i'm literally dumb
        //     let KEEP = false;
            
        //     let from: String = from_device.device.name().unwrap(); //@TODO handle
        //     let to: String = to_device.device.name().unwrap(); //@TODO handle

        //     let (from_str, to_str) = (&from[..], &to[..]);

        //     if !target.contains_key(from_str) { REMOVE } // from in switch_board; from not in target
        //     else if target.get(from_str).unwrap_or(&to_str) != &to_str { REMOVE } //switch_board#from -> a, target#from -> b, a != b
        //     else { KEEP } //switch_board#from -> a, target#from -> a
        // });
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};

    #[test]
    fn correct_state_local() {
        let mut coordinator = DeviceCoordinator::default();

        assert!(coordinator.switch_board.input_devices.len() > 0);
        assert!(coordinator.switch_board.output_devices.len() > 0);

        let host = cpal::default_host();
        coordinator.switch_board.connect((Device::from(host.default_input_device().expect("")), Device::from(host.default_output_device().expect("")))).expect("fail!");
        thread::sleep(time::Duration::from_secs(5));
        //@TODO make more general
        // let mut coordinator = DeviceCoordinator::default();

        // let mut target = HashMap::<&str, &str>::new();

        // let output_str_1 = "CABLE Input (VB-Audio Virtual Cable)";
        // target.insert(&output_str_1, &output_str_1);

        // let output_str_2 = "Speakers (Realtek(R) Audio)";
        // target.insert(&output_str_2, &output_str_2);

        // assert_eq!(coordinator.switch_board.rep.len(), 0);
        // coordinator.correct_state(target);
        // assert_eq!(coordinator.switch_board.rep.len(), 2);

        // for (from, to) in coordinator.switch_board.rep.iter() {
        //     assert!(from == to);
        // } 

        // let mut target = HashMap::<&str, &str>::new();

        // target.insert(&output_str_1, &output_str_2);
        // coordinator.correct_state(target);
        // assert_eq!(coordinator.switch_board.rep.len(), 1);
        
        //@TODO actually test tho lmao

        
    }
}