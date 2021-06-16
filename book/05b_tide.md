# HTTP integration with `actix-web`

The `"http_tide"` feature flag should be toggled on to enable HTTP integration with `tide`. Enabling `"http_tide"` feature flag will also enable the `"async_std_runtime"` feature flag.

A convenience method [`Server::handle_http(self)`](https://docs.rs/toy-rpc/0.7.0-alpha.2/toy_rpc/server/struct.Server.html#method.handle_http-1) is available when `"http_tide"` is the only http integration flag that is enabled. If multiple http integration feature flags are enabled, you can use [`Server::into_endpoint(self)`](https://docs.rs/toy-rpc/0.7.0-alpha.2/toy_rpc/server/struct.Server.html#method.into_endpoint) method instead.

We will demonstrate the usage with a new example.

```toml
[dependencies]
tide = "0.16.0"
async-std = { version = "1", features = [ "attributes", ] }  
toy-rpc = { version = "0.7.0", features = ["http_tide", "server"] }
```

```rust,noplaypen 
use std::sync::Arc;
use toy_rpc::macros::export_impl;
use toy_rpc::Server;
use tide::prelude::*;

pub struct Calculator { }

#[export_impl]
impl Calculator {
    #[export_method]
    async fn multiply(&self, args(i32, i32)) -> Result<i32, String> {
        Ok(args.0 * args.1)
    }

    #[export_method]
    async fn divide(&self, args(i32, i32)) -> Result<i32, String> {
        Ok(args.0 / args.1)
    }
}

#[async_std::main]
async fn main() {
    // Get the RPC server ready
    let addr = "127.0.0.1:23333";
    let calculator = Arc::new(Calculator { });
    let server = Server::builder()
        .register(calculator)
        .build();

    // Now we will work with `tide` HTTP server
    let mut app = tide::new();
    app.at("/rpc/").nest(server.handle_http());
    app.listen(addr).await.unwrap();
}
```