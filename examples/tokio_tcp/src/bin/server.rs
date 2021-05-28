use tokio::net::TcpListener;
use tokio::task;
use std::sync::{Arc};

use toy_rpc::Server;

use async_std_tcp::rpc::{Echo};

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let echo_service = Arc::new(
        Echo { }
    );

    let server = Server::builder()
        .register(echo_service)
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();

    log::info!("Starting server at {}", &addr);

    let handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });
    handle.await.expect("Error");
}