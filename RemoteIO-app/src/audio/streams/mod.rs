pub mod devicecoordinator;

use std::error::Error;
use ringbuf::RingBuffer;
use cpal::Device;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct DeviceForwarder {
    pub source: Device,
    pub dest: Device,
    pub config: cpal::StreamConfig,
    pub input_stream: cpal::Stream,
    pub output_stream: cpal::Stream,
}

impl DeviceForwarder {

    pub fn new(source: Device, dest: Device, config: cpal::StreamConfig) -> Result<Self, Box<dyn Error>> {
        let latency_frames = (150. / 1_000.0) * config.sample_rate.0 as f32;
        let latency_samples = latency_frames as usize * config.channels as usize;

        let (mut consumer, mut producer) = ringbuf::RingBuffer::<f32>::new(latency_samples * 2).split();

        let mut output_data_fn = move |data: &mut [f32], cb: &cpal:: OutputCallbackInfo| {
            play_output(&mut producer, data, cb)
        };
        let mut input_data_fn = move |data: &[f32], cb: &cpal::InputCallbackInfo| {
            listen_input(&mut consumer, data, cb);
        };

        let output_stream = dest.build_output_stream(&config, output_data_fn, err_fn)?;
        let input_stream = source.build_input_stream(&config, input_data_fn, err_fn)?;

        Ok(Self {
            source,
            dest,
            config,
            input_stream,
            output_stream
        })
    }
    pub fn play(&mut self) -> Result<(), Box<dyn Error>> {
        

        self.input_stream.play()?;
        self.output_stream.play()?;
        Ok(())
    }

    pub fn stop(self) {
        drop(self.input_stream);
        drop(self.output_stream);
    }

    
}

// fn play_input(producer: &mut ringbuf::Consumer<f32> )

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