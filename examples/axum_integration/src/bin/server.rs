use tokio::net::TcpListener;
use tokio::task;
use std::sync::Arc;
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

    let addr = "127.0.0.1:23333";
    let arith = Arc::new(Abacus { });

    let server = Server::builder()
        .register(arith)
        .build();
    let listener = TcpListener::bind(addr).await.unwrap();

    log::info!("Starting server at {}", &addr);
    server.accept(listener).await.unwrap();
}