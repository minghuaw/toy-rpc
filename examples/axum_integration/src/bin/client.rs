use toy_rpc::{Client};
use std::time::Duration;

use axum_integration::rpc::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "ws://127.0.0.1:23333/rpc/";
    let client = Client::dial_http(addr).await.unwrap();

    client.set_next_timeout(Duration::from_millis(500));
    let reply = Arith::add(&client, (3i32, 6i32)).await;
    println!("{:?}", reply);

    let reply = Arith::subtract(&client, (9i32, 1i32)).await;
    println!("{:?}", reply);

    client.close().await;
}