use env_logger;
use toy_rpc::client::{Client, Call};
use toy_rpc::error::Error;
use std::time::Duration;
use futures::{StreamExt, SinkExt};
use tokio::time::sleep;

use actix_v3_integration::{
    Count,
    rpc::{BarRequest, BarResponse, FooRequest, FooResponse}
};

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "ws://localhost:23333/";
    let mut client = Client::dial_websocket(addr).await.unwrap();
    let mut publisher = client.publisher::<Count>();
    let publisher_handle = tokio::spawn(async move {
        let mut count: u32 = 1;
        loop {
            println!("Publisher: {:?}", &count);
            publisher.send(Count(count)).await.unwrap();
            count += 1;
            sleep(Duration::from_millis(1000)).await;
        }
    });

    let mut subscriber = client.subscriber::<Count>(10).unwrap();
    let handle = tokio::spawn(async move {
        while let Some(item) = subscriber.next().await {
            println!("Subscriber: {:?}", item)
        }
    });

    let args = FooRequest { a: 1, b: 3 };
    let reply: Result<FooResponse, Error> = client.call_blocking("FooService.echo", args.clone());
    println!("{:?}", reply);

    let reply: Result<FooResponse, Error> = client.call("FooService.increment_a", args.clone()).await;
    println!("{:?}", reply);

    let call: Call<FooResponse> = client.call("FooService.increment_b", args);
    let reply = call.await.unwrap();
    println!("{:?}", reply);

    // third request, bar echo
    let args = BarRequest {
        content: "bar".to_string(),
    };
    let reply: BarResponse = client.call_blocking("BarService.echo", args.clone()).unwrap();
    println!("{:?}", reply);

    // fourth request, bar exclaim
    let reply: BarResponse = client.call("BarService.exclaim", args.clone()).await.unwrap();
    println!("{:?}", reply);

    let mut call: Call<()> = client.call("BarService.finite_loop", ());
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    println!("Calling cancellation");
    call.cancel();
    let reply = call.await; // this should give a `Canceled` error
    println!("{:?}", reply);

    println!("Calling finite_loop with timeout");
    let reply: Result<(), _> = client
        .set_next_timeout(Duration::from_secs(4))
        .call("BarService.finite_loop", ())
        .await;
    println!("{:?}", reply);

    // third request, get_counter
    let args = ();
    let call: Call<u32> = client.call("FooService.get_counter", args);
    let reply: u32 = call.await.unwrap();
    println!("{:?}", reply);
    // client.close().await;

    publisher_handle.abort();
    publisher_handle.await.unwrap();
    handle.await.unwrap();
}
