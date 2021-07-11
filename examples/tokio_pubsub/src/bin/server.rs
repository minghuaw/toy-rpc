use toy_rpc::Server;
use tokio::net::TcpListener;

use tokio_pubsub::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let server = Server::builder()
        .build();

    let listener = TcpListener::bind(ADDR).await.unwrap();
    server.accept(listener).await.unwrap();
}