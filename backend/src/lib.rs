use serde::{Serialize, Deserialize};

pub mod client;
pub mod server;

#[derive(Serialize, Deserialize, Debug)]
pub struct BinStreamConfig {
    pub channels: u16,
    pub sample_rate: u32,
    pub buffer_size: u32,

}



#[derive(Serialize, Deserialize, Debug)]
pub enum BinMessages {
    BinHello,
    BinConfig(BinStreamConfig),
    BinData(Vec<f32>),
    BinDevicesResponse(Vec<String>),
    BinDevicesRequest,
}