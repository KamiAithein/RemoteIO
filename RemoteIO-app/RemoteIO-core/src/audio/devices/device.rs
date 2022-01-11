use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{StreamConfig, BuildStreamError, Sample, InputCallbackInfo, OutputCallbackInfo, StreamError, Stream, SupportedStreamConfig, DefaultStreamConfigError};

use std::hash::{Hash, Hasher};

pub struct Device {
    device: cpal::Device,
}

// delegations, declarations ripped right from docs.rs
impl Device {
    pub fn name(&self) -> Result<String, cpal::DeviceNameError> {
        return self.device.name();
    }
    
    pub fn build_input_stream<T, D, E>(
        &self,
        config: &StreamConfig,
        data_callback: D,
        error_callback: E
    ) -> Result<Stream, BuildStreamError>
    where
        T: Sample,
        D: FnMut(&[T], &InputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static {
            self.device.build_input_stream(config, data_callback, error_callback)
        }
    
    pub fn build_output_stream<T, D, E>(
        &self,
        config: &StreamConfig,
        data_callback: D,
        error_callback: E
    ) -> Result<Stream, BuildStreamError>
    where
        T: Sample,
        D: FnMut(&mut [T], &OutputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static {
            self.device.build_output_stream(config, data_callback, error_callback)
        }
    
    pub fn default_input_config(
        &self
    ) -> Result<SupportedStreamConfig, DefaultStreamConfigError> {
        self.device.default_input_config()
    }

    pub fn default_output_config(
        &self
    ) -> Result<SupportedStreamConfig, DefaultStreamConfigError> {
        self.device.default_output_config()
    }
}

impl From<cpal::Device> for Device {
    fn from(device: cpal::Device) -> Self {
        Device {
            device
        }
    }
}

pub struct DeviceFromStringOpts<'host> {
    pub serial: &'host str,
    pub host: &'host cpal::Host
}

impl <'host> TryFrom<DeviceFromStringOpts<'host>> for Device {
    type Error = &'static str; //@TODO please change this
    fn try_from(opts: DeviceFromStringOpts) -> Result<Self, Self::Error> {
        let DeviceFromStringOpts{serial, host} = opts;
        
        match host.devices() {
            Ok(devices) => {
                let device = devices.filter(|device| device
                    .name()
                    .unwrap_or_else(|_|"ERR".to_owned())
                    .to_lowercase()
                    .eq(&serial.to_lowercase())
                ).last();

                match device {
                    Some(d) => Ok(Device::from(d)),
                    None => panic!("could not find device from serial <{}>!", serial)
                }
            },
            Err(e) => panic!("{}", e)
        }
    }
}

fn name_equal(lhs: &cpal::Device, rhs: &cpal::Device) -> bool {
    match lhs.name() {
        Ok(name) => {
            let rhs_name = rhs.name();
            if rhs_name.is_err() {
                return false;
            }

            return name == rhs_name.unwrap();
        }

        Err(e) => {
            return false;
        }
    }
}
impl PartialEq for Device {
    /// return true iff name match. If error then false!
    /// @todo come up with better equality operation...
    fn eq(&self, other: &Self) -> bool {
        name_equal(&self.device, &other.device)
    }
}
impl Eq for Device { }

impl PartialEq<cpal::Device> for Device {
    fn eq(&self, other: &cpal::Device) -> bool {
        name_equal(&self.device, other)
    } 
}

impl Hash for Device {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().unwrap().hash(state); //@TODO come up with actual hash!!
    }
}