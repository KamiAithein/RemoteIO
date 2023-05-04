use serde::{Serialize, Deserialize};

pub mod client;

#[derive(Serialize, Deserialize, Debug)]
pub struct BinStreamConfig {
    pub channels: u16,
    pub sample_rate: u32,
    pub buffer_size: u32,

}