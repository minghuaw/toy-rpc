use std::num::NonZeroUsize;
use toy_rpc::{Client};
use futures::StreamExt;

use tokio_pubsub::*;

#[tokio::main]
async fn main() {

    env_logger::init();
    let mut client = Client::builder()
        .set_ack_mode_manual()
        .dial(ADDR).await.unwrap();
    // let mut client = Client::dial(ADDR).await.unwrap(); // This will give a client with `AckModeNone`
    // let cap = NonZeroUsize::new(10); // This will create a local buffer size of 10
    let cap = None; // This will create an unbounded buffer size
    let mut count_sub = client.subscriber::<Count>(cap).unwrap();

    for _ in 0..300 {
        if let Some(result) = count_sub.next().await {
            let delivery = result.unwrap();
            let item = delivery.ack().await.unwrap();
            println!("{:?}", item);
        } else {
            break;
        }
    }
    client.close().await;
}