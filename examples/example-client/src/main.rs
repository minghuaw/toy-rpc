use toy_rpc::{Client};

use example_service::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    let call = client.arith().add((1, 3));
    let reply = call.await;
    println!("1 add 3 yields result: {:?}", reply);

    let reply = client.arith().subtract((1, 89)).await;
    println!("1 subtracted by 89 yields result: {:?}", reply);
}
