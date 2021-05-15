use async_std::net::TcpListener;
use async_std::sync::{Arc};
use async_std::task;

use toy_rpc::Server;

use cancellation::rpc::{Echo};

#[async_std::main]
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
    handle.await;
}
