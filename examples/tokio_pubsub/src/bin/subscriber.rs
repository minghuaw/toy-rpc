
use toy_rpc::Client;
use futures::StreamExt;
use std::time::Duration;

use tokio_pubsub::*;

#[tokio::main]
async fn main() {

    env_logger::init();
    let mut client = Client::dial(ADDR).await.unwrap();
    let mut count_sub = client.subscriber::<Count>(10).unwrap();

    for _ in 0..3 {
        if let Some(item) = count_sub.next().await {
            let item = item.unwrap();
            println!("{}", item);
        } else {
            break;
        }
    }

    // tokio::time::sleep(Duration::from_millis(100)).await;
    // println!("calling unsub");
    // client.unsubscribe::<Count>().await;
    
    // client.close().await;
    // tokio::time::sleep(Duration::from_millis(500)).await;
}