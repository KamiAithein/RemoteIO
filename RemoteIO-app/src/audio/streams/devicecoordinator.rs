use crate::audio::streams::DeviceForwarder;

use cpal::Host;

pub struct DeviceCoordinator {
    pub host: Host,
    pub forwards: Vec<DeviceForwarder>
}

impl DeviceCoordinator {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
            forwards: vec![],
        }
    }
}