// use anyhow::Result;

use toy_rpc::client_new::{Client, Call};

#[async_std::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    let call: Call<i32> = client.call("Echo.echo_i32", 13i32);
    let reply = call.await;
    println!("{:?}", reply);

    let reply: Result<i32, _> = client.call("Echo.echo_i32", 1313i32).await;
    println!("{:?}", reply);
}