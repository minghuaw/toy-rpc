# Server with `async-std` runtime

The overall usage with `async-std` runtime should look (almost) the same as the examples before. The only difference would be 

  1. add `async_std` to your dependecies
  2. enable feature `"async_std_runtime"` as opposed to `"tokio_runtime"`
  3. use `async_std::net::TcpListener` instead of `tokio::net::TcpListener`

If we were to run the server with `async-std` runtime, our previous [`#[export_trait]` and `#[export_trait_impl]` example](https://minghuaw.github.io/toy-rpc/04_server.html#example-with-export_trait-and-export_trait_impl) would then look like the code below.

```toml
[dependencies]
async-trait = "0.1.50"
async-std = { version = "1.9.0", features = ["attributes"] }
toy-rpc = { version = "0.7.0-alpha.2", features = ["async_std_runtime", "server"] }

# our service definition 
example-service = { version = "0.1.0", path = "../example-service" }
```

```rust,noplaypen 
// src/main.rs

use async_std::net::TcpListener;
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

#[async_std::main]
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

