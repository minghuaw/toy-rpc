use env_logger;
use serde::{Deserialize, Serialize};

use toy_rpc::client::Client;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EchoRequest {
    pub a: u32,
    pub b: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct EchoResponse {
    pub a: u32,
    pub b: u32,
}

#[async_std::main]
async fn main() {
    env_logger::init();

    let client = Client::dial("127.0.0.1:23333").unwrap();
    let args = EchoRequest { a: 1, b: 2 };

    let reply: EchoResponse = client.call("echo_service.echo", args.clone()).unwrap();

    println!("{:?}", reply);

    let handle = client.task("echo_service.increment_counter", args.clone());
    // let handle = task::spawn(
    //     client.async_call("service.increment", args)
    // );
    let reply: EchoResponse = handle.await.unwrap();

    println!("{:?}", reply);
}
