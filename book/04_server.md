# Server

The server API look straightforward and fairly close to that of golang's `net/rpc` package. First we should choose an async runtime and enable the `"server"` feature flag in our `Cargo.toml` file. Here, we will choose the `tokio` runtime be enabling the `"tokio_runtime"` feature flag.

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net"] }
toy-rpc = { version = "0.7.1", features = ["tokio_runtime", "server"] }
```

The next step is then to build the service instances and register the service instances onto a server which will be demonstrated below.

You can definitely register services that are defined with `#[export_impl]` and `#[export_trait]` on the same server. However, for simplicity and consistency with the [service definition chapter](https://minghuaw.github.io/toy-rpc/03_define_service.html), in the discussions below, we are going to have separate examples. If you need an example where services defined with `#[export_impl]` and `#[export_trait]` are all registered on the same server, please refer to [this](https://github.com/minghuaw/toy-rpc/tree/main/examples/example-server).

## Example with `#[export_impl]`

Let's just remind ourselves that in this example the service definition and implementation are located in the same file/project, and the file structure is as follows

```
./src
├── /bin
│   ├── server.rs
│   ├── client.rs
└── lib.rs
```

Using the service we have defined and implemented in the [previous chapter](https://minghuaw.github.io/toy-rpc/03_define_service.html#export_impl), we are going instantiate those services and register those instances in `src/bin/server.rs`.

```rust,noplaypen 
// src/bin/server.rs
use tokio::net::TcpListener;
use std::sync::Arc;
use toy_rpc::Server;

// Suppose the name of this crate is "example"
// Let's import the two services 
use example::{Foo, Bar}; 

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:23333";
    let foo = Arc::new(Foo { }); // instance of "Foo" service
    let bar = Arc::new(Bar { }); // instance of "Bar" service

    // Register the services onto a server
    let server = Server::builder()
        .register(foo) // register service instance with default name "Foo"
        .register(bar) // register service instance with default name "Bar"
        .build(); // build the server

    // Open a TcpListener for incoming connections
    let listener = TcpListener::bind(addr).await.unwrap();

    // Start our server and accept incoming connections and requests
    server.accept(listener).await.unwrap();
}
```

## Example with `#[export_trait]` and `#[export_trait_impl]`

Now we are back with the example with `#[export_trait]` and `#[export_trait_impl]`, let's just remind ourselves that we have three separate crates where the service is defined in `"example-service"` and will be implemented in `"example-server"` as shown below.

We will continue to use the `tokio` runtime so that we don't need to change our `Cargo.toml` file, but because we will be implementing the service below, we will need to add `async-trait` and our service definition crate into our dependencies.

```toml
[dependencies]
async-trait = "0.1.50"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net"] }
toy-rpc = { version = "0.7.1", features = ["tokio_runtime", "server"] }

# our service definition 
example-service = { version = "0.1.0", path = "../example-service" }
```

Now, let's implement the service and start the server

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