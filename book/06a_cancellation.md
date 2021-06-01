# Cancellation of RPC call

Cancellation is supported starting from version 0.7.0-alpha.1. The client method `call(...)` returns a type `Call`, which can be either `.await`ed for the response or `cancel()`ed to stop the execution. When an RPC request is started with the method `call(...)`, the request is sent by a background task whether or not the `Call` is `.await`ed. Upon `cancel()`, the client will send a cancellation request to the server; however, it should be noted that if the client is dropped immediately after calling `cancel()`, the server may not be able to receive the cancellation request before the connection is dropped by the client.

Below is a simple example showing cancellation on the `tokio` runtime.In this example, we are going to define a new service with a method that simply runs in loop and sleep for a certain period of time.

File structure:

```
./src
├── /bin
│   ├── server.rs
│   ├── client.rs
└── lib.rs
```

Add dependencies:

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
toy-rpc = { version = "0.7.0-alpha.1", features = ["tokio_runtime", "server", "client"] }
```

Service definition and implementation placed in `src/lib.rs`:

```rust
use std::time::Duration;
use toy_rpc::macros::export_impl;
use tokio::time;

struct Example { }

#[export_impl]
impl Example {
    #[export_method]
    async fn finite_loop(&self, args: ()) -> Result<(), String> {
        for counter in 0..500 {
            time::sleep(Duration::from_millis(500)).await;
        }
    }

    #[export_method]
    async fn echo(&self, args: String) -> Result<String, String> {
        Ok(args)
    }
}
```

Serve the RPC service with `src/bin/server.rs`

```rust
use std::sync::Arc;
use toy_rpc::Server;
use tokio::net::TcpListener;

// assume the name of the crate is "example"
// import service definition and implementation
use example::Example;

#[tokio::main]
async main() {
    let addr = "127.0.0.1:23333";
    let ex = Arc::new(Example { });
    let server = Server::builder()
        .register(ex)
        .build()

    let listener = TcpListener::bind(addr).await.unwrap();
    server.accept(listener).await.unwrap();
}
```

In the client, let's call the `finite_loop` RPC function and wait for two seconds and cancel it.

```rust 
use std::time::Duration;
use tokio::time;
use toy_rpc::client::{Client, Call};

// assume the name of the crate is "example"
// import service definitions and generated client stub functions
use example::*;

#[tokio::main]
async main() {
    let client = Client::dial("127.0.0.1:23333").await
        .expect("Failed to connect to server");

    let call: Call<()> = client
        .example() // access `Example` service
        .finite_loop(()); // access `finite_loop` method

    // wait for 2 seconds and cancel
    time::sleep(Duration::from_secs(2)).await;
    call.cancel();

    // the `Call` type can be `.await`ed to wait for the response
    let call = client
        .example() // access `Example` service
        .echo("hello world".to_string()); // access `echo` method
    let result = call.await;
    assert_eq!(result, Ok("hello world".to_string()));
}
```