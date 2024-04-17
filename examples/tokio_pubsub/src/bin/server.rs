use toy_rpc::Server;
use tokio::net::TcpListener;
use tokio::task;
use std::time::Duration;
use futures::{SinkExt, StreamExt}; // Make sure this is imported to use `send` or `next`

use tokio_pubsub::*;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let server = Server::builder()
        // .set_ack_mode_auto()
        .build();

    let mut count_pub = server.publisher::<Count>();
    let mut count_sub = server.subscriber::<Count>(10).unwrap();
    
    // Periodically publish a message
    task::spawn(async move {
        tokio::time::sleep(Duration::from_millis(2000)).await;
        let mut count = 0;
        loop {
            // let item = Count(count);
            let item = count;
            count_pub.send(item).await.unwrap();
            count += 1;
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    });

    // Prints message published on topic `Count`
    task::spawn(async move {
        while let Some(item) = count_sub.next().await {
            let item = item.unwrap();
            println!("topic: Count, item: {:?}", item);
        }
    });

    let listener = TcpListener::bind(ADDR).await.unwrap();
    server.accept(listener).await.unwrap();
}