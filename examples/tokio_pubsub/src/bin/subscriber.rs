
use toy_rpc::{Client};
use futures::StreamExt;

use tokio_pubsub::*;

#[tokio::main]
async fn main() {

    env_logger::init();
    let mut client = Client::builder()
        .set_ack_mode_auto()
        .dial(ADDR).await.unwrap();
    // let mut client = Client::dial(ADDR).await.unwrap();
    let mut count_sub = client.subscriber::<Count>(10).unwrap();

    for _ in 0..300 {
        if let Some(result) = count_sub.next().await {
            // let delivery = result.unwrap();
            // let item = delivery.ack().await.unwrap();
            println!("{:?}", result.unwrap());
        } else {
            break;
        }
    }
    client.close().await;
}