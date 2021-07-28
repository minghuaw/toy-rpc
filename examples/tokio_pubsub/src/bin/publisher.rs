use toy_rpc::Client;
use futures::SinkExt;
use std::time::Duration;

use tokio_pubsub::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    // let client = Client::dial(ADDR).await.unwrap();
    let client = Client::builder()
        .set_ack_mode_auto()
        .set_publisher_retry_timeout(Duration::from_secs(1))
        .set_publisher_max_num_retries(1)
        .dial(ADDR).await.unwrap();
    let mut count_pub = client.publisher::<Count>();
    
    let mut count = 90;
    // immediately dropping the client will not have the message fully sent
    while count >= 0  {
        count_pub.send(Count(count)).await.unwrap();
        count -= 1;
        tokio::time::sleep(Duration::from_millis(500)).await; 
    }
    client.close().await;
}