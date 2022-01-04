// #![allow(non_upper_case_globals)]
// #![allow(non_camel_case_types)]
// #![allow(non_snake_case)]
// include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// use std::ffi::CString;
// use std::ffi::c_void;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::RingBuffer;
use rand::Rng; 
use std::error::Error;

mod audio;
use audio::streams::devicecoordinator::DeviceCoordinator;
use audio::streams::DeviceForwarder;

fn main() -> Result <(), Box<dyn Error>> {
    let mut coordinator = DeviceCoordinator::new();

    let vb_cables = coordinator.host.devices()?.filter(|device| device.name()
                                                          .unwrap_or("".to_string())
                                                          .to_lowercase()
                                                          .contains("vb-audio")
    );

    let input_cables = coordinator.host.devices()?
        .filter(|device| device.name()
        .unwrap_or("".to_string())
        .to_lowercase()
        .contains("vb-audio")
    )
        .filter(|device| device.name()
        .unwrap()
        .to_lowercase()
        .contains("output") //naming convention switched

    );

    let output_cables = coordinator.host.devices()?
        .filter(|device| device.name()
        .unwrap_or("".to_string())
        .to_lowercase()
        .contains("vb-audio")
    )
        .filter(|device| device.name()
        .unwrap()
        .to_lowercase()
        .contains("input") //naming convention switched

    );

    let virtual_input = input_cables.last().unwrap();
    let virtual_output = output_cables.last().unwrap();
    let default_output = coordinator.host.default_output_device().unwrap();
    let default_input = coordinator.host.default_input_device().unwrap();

    //test
    let output_cables2 = coordinator.host.devices()?
        .filter(|device| device.name()
        .unwrap_or("".to_string())
        .to_lowercase()
        .contains("vb-audio")
    )
        .filter(|device| device.name()
        .unwrap()
        .to_lowercase()
        .contains("input") //naming convention switched

    );
    let virtual_output2 = output_cables2.last().unwrap();
    let config2: cpal::StreamConfig = virtual_output.default_output_config()?.into();
    //--
    
    // // We'll try and use the same configuration between streams to keep it simple.
    let config: cpal::StreamConfig = virtual_output.default_output_config()?.into();

    let latency_frames = (150. / 1_000.0) * config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;

    let mut forward1 = DeviceForwarder::new(default_output, virtual_output, config)?;


    let mut forward2 = DeviceForwarder::new(default_input, virtual_output2, config2)?;

    forward1.play()?;
    forward2.play()?;

    println!("Playing for 3 seconds... ");
    std::thread::sleep(std::time::Duration::from_secs(10));
    forward1.stop();
    forward2.stop();
    println!("Done!");

    // coordinator.forwards.iter().for_each(|forward|forward.play(&config).unwrap());

    Ok(())
}

fn min_example() -> Result<(), Box<dyn Error>> {
    let host = cpal::default_host();

    let vb_cables = host.devices()?.filter(|device| device.name()
                                                          .unwrap_or("".to_string())
                                                          .to_lowercase()
                                                          .contains("vb-audio")
    );

    let input_cables = host.devices()?
        .filter(|device| device.name()
        .unwrap_or("".to_string())
        .to_lowercase()
        .contains("vb-audio")
    )
        .filter(|device| device.name()
        .unwrap()
        .to_lowercase()
        .contains("output") //naming convention switched

    );

    let output_cables = host.devices()?
        .filter(|device| device.name()
        .unwrap_or("".to_string())
        .to_lowercase()
        .contains("vb-audio")
    )
        .filter(|device| device.name()
        .unwrap()
        .to_lowercase()
        .contains("input") //naming convention switched

    );

    let virtual_input = input_cables.last().unwrap();
    let virtual_output = output_cables.last().unwrap();
    let default_output = host.default_output_device().unwrap();

    // // Find devices.
    // let input_device = host.default_input_device()
    // .expect("failed to find input device");

    // let output_device = host.default_output_device()
    // .expect("failed to find output device");

    // // We'll try and use the same configuration between streams to keep it simple.
    let config: cpal::StreamConfig = virtual_output.default_output_config()?.into();

    // // Create a delay in case the input and output devices aren't synced.
    let latency_frames = (150. / 1_000.0) * config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;

    let ring = RingBuffer::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();

    // // Fill the samples with 0.0 equal to the length of the delay.
    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut output_fell_behind = false;
        for &sample in data {
            if producer.push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            eprintln!("output stream fell behind: try increasing latency");
        }
    };

    // let input_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
    //     let mut input_fell_behind = false;
    //     for sample in data {
    //         if producer.push(*sample).is_err() {
    //             input_fell_behind = true;
    //         }
    //     }
    //     if input_fell_behind {
    //         eprintln!("input stream fell behind: try increasing latency");
    //     }
    // };
    
    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
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
    };

    let input_stream = default_output.build_input_stream(&config, input_data_fn, err_fn)?;
    let output_stream = virtual_output.build_output_stream(&config, output_data_fn, err_fn)?;

    input_stream.play()?;
    output_stream.play()?;

    // Run for 3 seconds before closing.
    println!("Playing for 3 seconds... ");
    std::thread::sleep(std::time::Duration::from_secs(10));
    drop(input_stream);
    drop(output_stream);
    println!("Done!");

    Ok(())
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}