// use env_logger;
// use toy_rpc::client::Client;
// use toy_rpc::error::Error;

// use tide_integration::rpc::{BarRequest, BarResponse, FooRequest, FooResponse};

// #[async_std::main]
// async fn main() {
//     env_logger::init();

//     let addr = "http://127.0.0.1:23333/rpc/";
//     let mut client = Client::dial_http(addr).await.unwrap();

//     let args = FooRequest { a: 1, b: 3 };
//     let reply: Result<FooResponse, Error> = client.call_http("foo_service.echo", &args);
//     println!("{:?}", reply);

//     let reply: Result<FooResponse, Error> = client.call_http("foo_service.increment_a", &args);
//     println!("{:?}", reply);

//     let reply: Result<FooResponse, Error> = client.call_http("foo_service.increment_b", &args);
//     println!("{:?}", reply);

//     // third request, bar echo
//     let args = BarRequest {
//         content: "bar".to_string(),
//     };
//     let reply: BarResponse = client.call_http("bar_service.echo", &args).unwrap();
//     println!("{:?}", reply);

//     // fourth request, bar exclaim
//     let reply: BarResponse = client.call_http("bar_service.exclaim", &args).unwrap();
//     println!("{:?}", reply);

//     // third request, get_counter
//     let args = ();
//     let reply: u32 = client.call_http("foo_service.get_counter", &args).unwrap();
//     println!("{:?}", reply);
// }



use serde::{Serialize, Deserialize};
use toy_rpc::Client;
use toy_rpc::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct ExampleRequest {
    a: u32
}

#[derive(Debug, Serialize, Deserialize)]
struct ExampleResponse {
    a: u32
}

#[async_std::main]
async fn main() {
    // note that the endpoint path must be specified
    let path = "http://127.0.0.1:8888/rpc/";
    let mut client = Client::dial_http(path).await.unwrap();

    let args = ExampleRequest{a: 1};
    let reply: Result<ExampleResponse, Error> = client.call_http("example.echo", &args);
    println!("{:?}", reply);
}