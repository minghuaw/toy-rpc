use toy_rpc::Client;
use toy_rpc::error::Error;

use docs_examples::rpc; // assume the rpc module can be found here

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let client = Client::dial(addr).await.unwrap();

    let args = rpc::ExampleRequest{a: 1};
    let reply: Result<rpc::ExampleResponse, Error> = client.call("example.echo", &args);
    println!("{:?}", reply);
 
    client.close();
}