use tokio::time;
use std::time::Duration;

use toy_rpc::client::{Client, Call};
use tokio_tcp::rpc::*;

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() -> anyhow::Result<()> {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    let call: Call<i32> = client.call("Echo.echo_i32", 13i32);
    let reply = call.await?;
    println!("{:?}", reply);

    let reply: i32 = client.call("Echo.echo_i32", 1313i32).await?;
    println!("{:?}", reply);

    println!("Calling finite loop");
    let mut call: Call<()> = client.call("Echo.finite_loop", ());
    time::sleep(Duration::from_secs(2)).await;
    println!("Calling cancellation");
    call.cancel();

    println!("Calling finite loop again");
    let call: Call<()> = client.call("Echo.finite_loop", ());
    time::sleep(Duration::from_secs(2)).await;
    println!("Just dropping call");
    drop(call);

    // let call: Call<i32> = client.call("Echo.echo_i32", 13i32);
    let call = client.echo().echo_i32(13i32);
    let reply = call.await?;
    println!("{:?}", reply);

    let reply = Arith::add(&client, (3i32, 4i32)).await?;
    println!("{:?}", reply);

    let reply: u32 = Arith::get_num_anyhow(&client, ()).await?;
    println!("{:?}", reply);

    let reply = Arith::get_str_anyhow(&client, ()).await?;
    println!("{:?}", reply);

    client.close().await;
    Ok(())
}