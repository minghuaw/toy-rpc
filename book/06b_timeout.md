# Set timeout for an RPC call

The client can set the timeout for the next RPC request using the `timeout(duration: Duration)` method, which can be chained with the `call` method. Please note that the timeout is **ONLY** set for the immediate next RPC call, and all RPC calls do not have timeout if not explicitly set using the `timeout` method.

We will re-use the example service definition in the [cancellation chapter](https://minghuaw.github.io/toy-rpc/06a_cancellation.html). For you convenience, the service definition and server code are copied below.

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
toy-rpc = { version = "0.7.3", features = ["tokio_runtime", "server", "client"] }
```

Service definition and implementation placed in `src/lib.rs`:

```rust,noplaypen
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

Serve the RPC service with `src/bin/server.rs`.

```rust,noplaypen
use std::sync::Arc;
use toy_rpc::Server;
use tokio::net::TcpListener;

// assume the name of the crate is "example"
// import service definition and implementation
use example::Example;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:23333";
    let ex = Arc::new(Example { });
    let server = Server::builder()
        .register(ex)
        .build()

    let listener = TcpListener::bind(addr).await.unwrap();
    server.accept(listener).await.unwrap();
}
```

On the client side, let's call the `finite_loop` RPC function with a timeout of three seconds.

```rust,noplaypen 
use std::time::Duration;
use tokio::time;
use toy_rpc::client::{Client, Call};

// assume the name of the crate is "example"
// import service definitions and generated client stub functions
use example::*;

#[tokio::main]
async fn main() {
    let client = Client::dial("127.0.0.1:23333").await
        .expect("Failed to connect to server");

    let call: Call<()> = client
        .timeout(Duration::from_secs(3))
        .example() // access `Example` service
        .finite_loop(()); // access `finite_loop` method
    let result = call.await; // This should give you `Err(Error::Timeout)`
}
```