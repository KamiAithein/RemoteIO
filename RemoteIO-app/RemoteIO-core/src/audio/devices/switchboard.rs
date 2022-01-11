use crate::audio::devices::device::{Device, DeviceFromStringOpts};
use crate::audio::devices::deviceswitch::{DeviceSwitch, DeviceSwitchOpts, Connection};

use cpal::Host;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use std::collections::HashMap;
use std::error::Error;

pub struct SwitchBoard {
    // input_devices: Vec<DeviceSwitch>,
    // output_devices: Vec<DeviceSwitch>,
    pub device_map: HashMap<DeviceSwitch, Option<DeviceSwitch>>
}

pub struct SwitchBoardOpts {
    pub host: cpal::Host,
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
        let (mut from, mut to) = (DeviceSwitch::from(from), DeviceSwitch::from(to));
        // let producer: &mut DeviceSwitch = SwitchBoard::find_device_switch_from(&mut self.input_devices, from)
        //     .expect("could not find device switch!");
        // let consumer: &mut DeviceSwitch = SwitchBoard::find_device_switch_from(&mut self.output_devices, to)
        //     .expect("could not find device switch!");
        
        // remove connection
        let (prod, cons_opt) = match self.device_map.remove_entry(&from) {
            Some((prod, cons_opt)) => {
                (prod, cons_opt)
            }
            None => {todo!("dunno")} //@TODO the device was never there in the first place...
        };

        match cons_opt {
            Some(cons) => {
                //there is a connection!
                assert!(prod.connection.is_some(), "there should be a connection!");
                assert!(cons.connection.is_some(), "there should be a connection!");

                let disconnect = |switch| {

                };

                disconnect(prod);
                disconnect(cons);
            }
            _ => {}
        }

        let (from_conn, to_conn) = SwitchBoard::establish_connection((&mut from, &mut to)).expect("could not establish connection!");
        
        from_conn.stream.play().expect("could not start producer stream!");
        to_conn.stream.play().expect("could not start consumer stream!");

        from.connection = Some(from_conn);
        to.connection = Some(to_conn);
        

        self.device_map.insert(
            from,
            Some(to)
        );
    
        Ok(())
    }


    /// Establishes and returns connections between target_conn.0 -> target_conn.1
    /// Does not mutate target_conn values
    fn establish_connection(target_conn: (&DeviceSwitch, &DeviceSwitch)) -> Result<(Connection, Connection), Box<dyn Error>> {
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