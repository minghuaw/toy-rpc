use env_logger;
use serde::{Deserialize, Serialize};

use toy_rpc::client::Client;

#[derive(Serialize, Deserialize, Debug)]
struct FooRequest {
    pub a: u32,
    pub b: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct FooResponse {
    pub a: u32,
    pub b: u32,
}

#[async_std::main]
async fn main() {
    env_logger::init();

    let client = Client::dial("127.0.0.1:23333").unwrap();
    let args = FooRequest { a: 1, b: 2 };

    let reply: FooResponse = client.call("foo_service.echo", &args).unwrap();

    println!("{:?}", reply);

    let handle = client.task("foo_service.increment_b", args);
    let reply: FooResponse = handle.await.unwrap();
    println!("{:?}", reply);

    let args = ();
    let handle = client.task("foo_service.get_counter", args);
    let reply: u32 = handle.await.unwrap();

    println!("{:?}", reply);
}
