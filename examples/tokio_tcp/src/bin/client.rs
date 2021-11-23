use tokio::time;
use std::time::Duration;

use toy_rpc::client::{Client, Call};
use tokio_tcp::rpc::*;

#[tokio::main]
async fn main() {
    let _ = run().await;
}

async fn run() -> anyhow::Result<()> {
    env_logger::init();

    // Establish connection
    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    // Perform RPC using `call()` method
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

    // Equivalent to `let call: Call<i32> = client.call("Echo.echo_i32", 13i32);`
    let call = client.echo().echo_i32(13i32);
    let reply = call.await?;
    println!("{:?}", reply);

    let ok_result = client.echo().echo_if_equal_to_one(1).await;
    let err_result = client.echo().echo_if_equal_to_one(2).await;
    println!("Ok result: {:?}", ok_result);
    println!("Err result: {:?}", err_result);

    // Demo usage with the `Arith` trait
    let addition = client
        .arith() // generated for `Arith` service
        .add((1, 3)).await; // call `add` method
    println!("{:?}", addition);

    // Although the return type of `divide` is a `Result<T, E>`,
    // the execution result will be mapped to `Result<T, toy_rpc::Error>`
    // where `E` is mapped to `toy_rpc::Error::ExecutionError` so that 
    //   (1) the Error type doesn't need to implement `Serialize` and
    //   (2) the users don't need to unwrap twice
    let division = client
        .arith()
        .divide((3, 1)).await;
    println!("{:?}", division);

    // let's try to get an execution error
    let divide_by_zero = client
        .arith()
        .divide((3, 0)).await;
    println!("{:?}", divide_by_zero);

    // Now let's take a look at using the generated trait impl for the client.
    let addition = Arith2::add(&client, (7, 8)).await;
    println!("{:?}", addition);

    let division = Arith2::divide(&client, (7, 2)).await;
    println!("{:?}", division);
    let divide_by_zero = Arith2::divide(&client, (7, 0)).await;
    println!("{:?}", divide_by_zero);

    client.close().await;
    Ok(())
}