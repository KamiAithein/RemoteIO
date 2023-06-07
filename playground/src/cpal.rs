
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[tokio::main]
async fn main() {
    let to_record = cpal::default_host().default_output_device().expect("no default output device!");
    assert!(to_record.name().expect("could not get default output device name!") == "Komfortzone");

    
    let stream = to_record.build_output_stream(
        &to_record.default_output_config().expect("could not get output config!").into(), 
        move |data: &mut [f32], _| {
            println!("{:?}", data);
        }, 
        |_| {}, 
        None
    );
    
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    ()
}