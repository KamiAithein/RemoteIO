use serde::{Serialize, Deserialize};

pub mod client;
pub mod server;
pub mod sync_client;

#[derive(Serialize, Deserialize, Debug)]
pub struct BinStreamConfig {
    pub channels: u16,
    pub sample_rate: u32,
    pub buffer_size: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BinStreamAlias {
    pub alias: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AliasedData<T> {
    pub alias: BinStreamAlias,
    pub data: T
}



#[derive(Serialize, Deserialize, Debug)]
pub enum BinMessages {
    BinHello(BinStreamAlias),
    BinConfig(AliasedData<BinStreamConfig>),
    BinData(AliasedData<Vec<f32>>),
    BinDevicesResponse(Vec<String>),
    BinDevicesRequest,
}