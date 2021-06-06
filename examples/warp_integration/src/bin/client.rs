use toy_rpc::client::Client;
use toy_rpc::error::Error;

use warp_integration::rpc::{BarRequest, BarResponse, FooRequest, FooResponse};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let addr = "ws://127.0.0.1:23333/rpc/";
    let client = Client::dial_http(addr).await.unwrap();

    let args = FooRequest { a: 1, b: 3 };
    let reply: Result<FooResponse, Error> = client.call_blocking("FooService.echo", args.clone());
    println!("{:?}", reply);

    let reply: Result<FooResponse, Error> = client.call("FooService.increment_a", args.clone()).await;
    println!("{:?}", reply);

    let call = client.call("FooService.increment_b", args);
    let reply: Result<FooResponse, Error> = call.await;
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
    let call = client.call("FooService.get_counter", args);
    let reply: u32 = call.await.unwrap();
    println!("{:?}", reply);
}
