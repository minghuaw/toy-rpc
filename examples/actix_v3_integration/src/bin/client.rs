use env_logger;
use toy_rpc::client::{Client, Call};
use toy_rpc::error::Error;

use actix_v3_integration::rpc::{BarRequest, BarResponse, FooRequest, FooResponse};

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "ws://127.0.0.1:23333/rpc/";
    let client = Client::dial_http(addr).await.unwrap();

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

    // third request, get_counter
    let args = ();
    let call: Call<u32> = client.call("FooService.get_counter", args);
    let reply: u32 = call.await.unwrap();
    println!("{:?}", reply);
    // client.close().await;
}
