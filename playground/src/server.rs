use remoteio_backend::client::Client;
use remoteio_backend::server::Server;

#[tokio::main]
async fn main() {
    let mut server = remoteio_backend::server::MetalServer::new("0.0.0.0:8000");
    let _ = server.bind().await; 

    loop{}
}
