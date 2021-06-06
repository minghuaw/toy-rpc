// use anyhow::Result;
use cfg_if::cfg_if;
use std::time::Duration;
use toy_rpc::client::{Client, Call};

use cancellation::{sleep, rpc::*};

async fn run() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    let call: Call<i32> = client.call("Echo.echo_i32", 13i32);
    let reply = call.await;
    println!("{:?}", reply);

    let reply: Result<i32, _> = client.call("Echo.echo_i32", 1313i32).await;
    println!("{:?}", reply);

    println!("Calling finite loop");
    let call: Call<()> = client.call("Echo.finite_loop", ());
    sleep(Duration::from_secs(2)).await;
    println!("Calling cancellation");
    call.cancel();
    let reply = call.await;
    println!("{:?}", reply);

    println!("Calling infinite loop");
    let call: Call<()> = client.echo().infinite_loop(());
    sleep(Duration::from_secs(3)).await;
    println!("Calling cancellation");
    call.cancel();
    let reply = call.await;
    println!("{:?}", reply);

    println!("Calling infinite loop with timeout");
    let call: Call<()> = client.timeout(Duration::from_secs(3))
        .echo()
        .infinite_loop(());
    let reply = call.await;
    println!("{:?}", reply);

    println!("Make another echo call before dropping client");
    let reply = client.echo().echo_i32(31).await;
    println!("{:?}", reply);
}

cfg_if! {
    if #[cfg(feature = "async_std_runtime")] {
        #[async_std::main]
        async fn main() {
            run().await;
            println!("After run");
        }
    } else if #[cfg(feature = "tokio_runtime")] {         
        #[tokio::main]
        async fn main() {
            run().await;
            println!("After run");
        }
    }
}


