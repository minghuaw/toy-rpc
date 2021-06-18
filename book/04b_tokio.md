# Server with `tokio` runtime

This is just going to be an identical copy of the previous [`#[export_trait]` and `#[export_trait_impl]` example](https://minghuaw.github.io/toy-rpc/04_server.html#example-with-export_trait-and-export_trait_impl).

```toml
[dependencies]
async-trait = "0.1.50"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net"] }
toy-rpc = { version = "0.7.4", features = ["tokio_runtime", "server"] }

# our service definition 
example-service = { version = "0.1.0", path = "../example-service" }
```

```rust,noplaypen 
// src/main.rs

use tokio::net::TcpListener;
use std::sync::Arc;
use async_trait::async_trait;
use toy_rpc::Server;
use toy_rpc::macros::export_trait_impl;

// Import our service definition
// Make sure you import everything to include the auto-generated helper 
// traits that allow convenient service registration
use example_service::*;

struct Abacus { }

#[async_trait]
#[export_trait_impl] // The default service name will be "Arith"
impl Arith for Abacus {
    // Please note that you do NOT need to mark methods with
    // `#[export_method]` another time

    async fn add(&self, args: (i32, i32)) -> Result<i32, String> {
        Ok(args.0 + args.1)
    }

    async fn subtract(&self, args: (i32, i32)) -> Result<i32, String> {
        Ok(args.0 - args.1)
    }

    fn say_hi(&self) {
        println!("hi");
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:23333";
    let arith = Arc::new(Abacus{}); // create an instance of the `Arith` service
    let listener = TcpListener::bind(addr).await.unwrap();
    let server = Server::builder()
        .register(arith) // register service with default name "Arith"
        .build();

    println!("Starting server at {}", &addr);
    server.accept(listener).await.unwrap()
}
```