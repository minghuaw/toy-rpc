# HTTP integration with `actix-web`

The `"http_warp"` feature flag should be toggled on to enable HTTP integration with `warp` crate. Enabling `"http_warp"` feature flag will also enable the `"tokio_runtime"` featrue flag.

A convenience method [`Server::handle_http(self)`](https://docs.rs/toy-rpc/0.7.0-alpha.2/toy_rpc/server/struct.Server.html#method.handle_http-2) is available when `"http_warp"` is the only enabled http integration feature flag. If you have multiple http integration flags enabled, you can use the [`Server::into_boxed_filter(self)`](https://docs.rs/toy-rpc/0.7.0-alpha.2/toy_rpc/server/struct.Server.html#method.into_boxed_filter) method instead.

We will demonstrate the usage with a new example.

```toml
[dependencies]
warp = "0.3.0"
tokio = { version = "1.4.0", features = ["rt-multi-thread", "macros"] }
toy-rpc = { version = "0.7.1", features = ["http_warp", "server"] }
```

```rust,noplaypen 
use std::sync::Arc;
use toy_rpc::macros::export_impl;
use toy_rpc::Server;

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

#[tokio::main]
async fn main() {
    let calculator = Arc::new(Calculator { });
    let server = Server::builder()
        .register(calculator)
        .build();

    // Serve RPC at "ws://127.0.0.1/rpc/" 
    // (there is a "_rpc_" appended to the end of the path but the client takes care of that) 
    let routes = warp::path("rpc")
        .and(server.handle_http());
    warp::serve(routes).run(([127, 0, 0, 1], 23333)).await;
}
```