use toy_rpc::{Client, Error};

use axum_integration::rpc::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    let reply = Arith::add(&client, (3i32, 6i32)).await;
    println!("{:?}", reply);

    let reply = Arith::subtract(&client, (9i32, 1i32)).await;
    println!("{:?}", reply);

    client.close().await;
}