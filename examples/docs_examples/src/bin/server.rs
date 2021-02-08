use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task;
use toy_rpc::macros::service;
use toy_rpc::Server;

use docs_examples::rpc; // assume the rpc module can be found here

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:8080";
    let example_service = Arc::new(
        rpc::ExampleService {
            counter: Mutex::new(0),
        }
    );

    // notice that the second argument in `service!()` macro is a path
    let server = Server::builder()
        .register("example", service!(example_service, rpc::ExampleService))
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Starting listener at {}", &addr);

    let handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });

    // tokio JoinHandle returns an extra result
    handle.await.unwrap();
}