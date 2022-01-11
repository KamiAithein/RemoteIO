use crate::audio::devices::device::Device;

use std::hash::{Hash, Hasher};
use std::cmp::{PartialEq, Eq};

pub struct Connection {
    pub stream: cpal::Stream,
    pub config: cpal::StreamConfig,
}

//ON CONNECTION AUDIO STREAM WILL BE ESTABLISHED!
pub struct DeviceSwitch {
    pub device: Device,
    pub connection: Option<Connection>,
}

pub struct DeviceSwitchOpts {
    pub device: Device
}

impl DeviceSwitch {
    pub fn new_from(opts: DeviceSwitchOpts) -> DeviceSwitch {
        let DeviceSwitchOpts{device} = opts;

        DeviceSwitch {
            device,
            connection: None,
        }
    }
}

impl Hash for DeviceSwitch {
    fn hash<H: Hasher>(&self, state: &mut H) { 
        self.device.hash(state)
    }
}

impl PartialEq for DeviceSwitch {
    fn eq(&self, other: &Self) -> bool {
        self.device == other.device 
    } 
}

impl From<Device> for DeviceSwitch {
    fn from(device: Device) -> Self {
        DeviceSwitch::new_from(DeviceSwitchOpts {
            device,
        })
    }
}

impl Eq for DeviceSwitch { }