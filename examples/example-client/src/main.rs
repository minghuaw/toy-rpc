use toy_rpc::{Client};

use example_service::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    let call = client.arith().add((1, 3));
    let reply = call.await;
    println!("[Arith]: 1 + 3 = {:?}", reply);

    let reply = client.arith().subtract((1, 89)).await;
    println!("[Arith]: 1 - 89 = {:?}", reply);

    let reply: Result<i32, _> = client.call("Calculator.multiply", (3i32, 4i32)).await;
    println!("[Calculator]: 3 * 4 = {:?}", reply);

    let reply: Result<i32, _> = client.call("Calculator.divide", (15i32, 3i32)).await;
    println!("[Calculator]: 15 / 3 = {:?}", reply);

    let message = Message::Old(Bar {id: 13 });
    let reply: anyhow::Result<u8> = client.consume(message).await;
    println!("[Consumer]: {:?}", reply);
}
