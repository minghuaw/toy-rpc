use toy_rpc::Server;
use tokio::net::TcpListener;
use tokio::task;
use std::time::Duration;
use futures::SinkExt;

use tokio_pubsub::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let server = Server::builder()
        .build();

    let mut count_pub = server.publisher::<Count>();
    task::spawn(async move {
        let mut count = 0;
        loop {
            count_pub.send(count).await.unwrap();
            count += 1;
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    });
    let listener = TcpListener::bind(ADDR).await.unwrap();
    server.accept(listener).await.unwrap();
}