use cpal::traits::HostTrait;
use remoteio_backend::client::Client;
use remoteio_backend::server::Server;

#[tokio::main]
async fn main() {
    let mut host = cpal::default_host().default_input_device().expect("could not get default input device!");
    let mut client = remoteio_backend::client::Metal2RemoteClient::new(host);


    let _ = client.connect("ws://0.0.0.0:8000").await;
    // async_std::task::sleep(tokio::time::Duration::from_secs(10)).await;
}
