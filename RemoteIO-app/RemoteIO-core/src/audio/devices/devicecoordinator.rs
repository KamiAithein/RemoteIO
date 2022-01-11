use crate::audio::devices::device::{Device, DeviceFromStringOpts};
use crate::audio::devices::deviceswitch::{DeviceSwitch, DeviceSwitchOpts, Connection};
use crate::audio::devices::switchboard::{SwitchBoard, SwitchBoardOpts};

use cpal::Host;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use std::collections::HashMap;
use std::error::Error;

pub struct DeviceCoordinator {
    pub host: Host,
    /// switch_board = fn(producer) -> consumer
    pub switch_board: SwitchBoard
}

pub struct DeviceCoordinatorOpts {
    pub host: Host,
    pub switch_board: SwitchBoard
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

impl <'switch> Default for DeviceCoordinator {
    fn default() -> Self {
        Self::new_from(DeviceCoordinatorOpts::default())
    }
}

impl <'switch> Default for DeviceCoordinatorOpts {
    fn default() -> Self {
        Self {
            host: cpal::default_host(),
            switch_board: SwitchBoard::new_from(SwitchBoardOpts{
                host: cpal::default_host()
            }),
        }
    }
}

impl DeviceCoordinator {
    pub fn new() -> Self {
        Self::default() //@TODO implement reasonable constructor
    }

    pub fn new_from(options: DeviceCoordinatorOpts) -> Self {
        let DeviceCoordinatorOpts {host, switch_board} = options;
        Self {
            host,
            switch_board,
        }
    }

    pub fn connect(&mut self, mapping: (Device, Device)) -> Result<(), Box<dyn Error>> {
        let (from, to) = mapping;

        println!("connecting {} : {}", from.name().unwrap(), to.name().unwrap());
        self.switch_board.connect((from, to))
    }

    pub fn devices(&self) -> Result<cpal::Devices, cpal::DevicesError> {
        return self.host.devices();
    }

    pub fn state(&self) -> DeviceCoordinatorState {
        let mut map = HashMap::new();

        self.switch_board.device_map.iter().for_each(|forward| {
            let (source, dest_opt) = forward;
            match dest_opt {
                Some(dest) => {
                    map.insert(&source.device, &dest.device);
                }
                _ => {}
            };
            
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
    pub fn correct_state(&mut self, target: HashMap<String, String>) -> Result<(), Box<dyn Error>> {
        // let mapping = &mut self.switch_board.device_map;
        for (from, to) in target.iter() {
            let handle_conversion = |to_convert: &String| {
                let opts = DeviceFromStringOpts{host: &self.host, serial: to_convert};

                match Device::try_from(opts) {
                    Ok(device) => device,
                    Err(e) => panic!("error converting from string to device! caused by: {}", e)
                }
            };

            
            let from_device = DeviceSwitch::from(handle_conversion(&from));
            let to_device = DeviceSwitch::from(handle_conversion(&to));

            match self.switch_board.device_map.get(&from_device) {
                Some(Some(device)) => {
                    if device != &to_device {
                        self.connect((from_device.device, to_device.device)).expect("could not connect devices!");
                    }
                },
                _ => {
                    self.connect((from_device.device, to_device.device)).expect("could not connect devices!");
                }
            }
            
            
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