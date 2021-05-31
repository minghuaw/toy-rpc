# HTTP integration with `actix-web`

A convenience method `Server::handle_http()` is available when `"http_actix_web"` is the only enabled http integration feature flag. If you have multiple http integration feature flags enabled, you can use the `Server::scope_config` method instead (please note that you should use `scope_config` without brackets).

The `"http_actix_web"` feature flag should be toggled on to enable HTTP integration with `actix-web`. Enabling `"http_actix_web"` feature flag will also enable the `"tokio_runtime"` feature flag. We will demonstrate the usage with a new example.

```toml
[dependencies]
actix-web = "3.3.2"
toy-rpc = { version = "0.7.0-alpha.1", features = ["http_actix_web", "server"] }
```

```rust 
use std::sync::Arc;
use toy_rpc::macros::export_impl;
use toy_rpc::Server;
use actix_web::{web, HttpServer, App};

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

#[actix_web::main]
async fn main() {
    let addr = "127.0.0.1:23333";
    let calculator = Arc::new(Calculator { });
    let server = Server::builder()
        .register(calculator)
        .build();

    let app_data = web::Data::new(server);

    HttpServer::new(
        move || {
            App::new()
                .service(
                    web::scope("/rpc/")
                        .app_data(app_data.clone())
                        .configure(Server::handle_http()) // equivalent to the line below
                        // .configure(Server::scope_config)
                )
        }
    )
    .bind(addr).unwrap()
    .run()
    .await;
}
```