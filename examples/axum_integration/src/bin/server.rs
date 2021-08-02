use axum::{prelude::*, routing::{nest}};
use std::sync::Arc;
use std::net::SocketAddr;
use async_trait::async_trait;

use toy_rpc::{
    Server, macros::export_trait_impl, Error
};

use axum_integration::rpc::*;

struct Abacus { }

#[async_trait]
#[export_trait_impl]
impl Arith for Abacus {
    async fn add(&self, args: (i32, i32)) -> Result<i32, Error> {
        Ok(args.0 + args.1)
    }

    async fn subtract(&self, args: (i32, i32)) -> Result<i32, Error> {
        Ok(args.0 - args.1)
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = SocketAddr::from(([127, 0, 0, 1], 23333));
    let arith = Arc::new(Abacus { });

    let server = Server::builder()
        .register(arith)
        .build();
    // let app = route("/rpc/_rpc_", ws(Server::<AckModeNone>::handle_axum_websocket))
    //     .layer(AddExtensionLayer::new(server));
    let app = nest(
        "/rpc",
        server.handle_http()
    );

    log::info!("Starting server at {}", &addr);
    hyper::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}