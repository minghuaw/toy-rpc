use toy_rpc::Client;
use futures::SinkExt;
use std::time::Duration;

use tokio_pubsub::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    let client = Client::dial(ADDR).await.unwrap();
    let mut count_pub = client.publisher::<Count>();
    count_pub.send(3).await;
    // count_pub.flush().await;
    // count_pub.close().await;

    tokio::time::sleep(Duration::from_millis(100)).await; // immediately dropping the client will not have the message fully sent
    // client.close().await;
}