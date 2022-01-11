// #![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

use remoteio_core::audio::devices::devicecoordinator::DeviceCoordinator;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let dc = DeviceCoordinator::default();
    dc.devices().unwrap().for_each(|device| {
        println!("{}", device.name().unwrap_or("aa".to_owned()))
    });
    let app = eframe_template::TemplateApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
