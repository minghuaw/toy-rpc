use toy_rpc::Client;
use futures::SinkExt;
use std::time::Duration;

use tokio_pubsub::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    let client = Client::dial(ADDR).await.unwrap();
    let mut count_pub = client.publisher::<Count>();
    count_pub.send(Count(3)).await.unwrap();

    // immediately dropping the client will not have the message fully sent
    tokio::time::sleep(Duration::from_millis(100)).await; 
    client.close().await;
}