use std::sync::Arc;
use tokio::net::TcpListener;
use async_trait::async_trait;
use toy_rpc::Server;
use toy_rpc::macros::{export_impl, export_trait_impl};

use example_service::*;

struct Abacus { }

#[async_trait]
#[export_trait_impl]
impl Arith for Abacus {
    async fn add(&self, args: (i32, i32)) -> Result<i32, String> {
        Ok(args.0 + args.1)
    }

    async fn subtract(&self, args: (i32, i32)) -> Result<i32, String> {
        Ok(args.0 - args.1)
    }
}

struct Calculator { }

#[export_impl]
impl Calculator {
    #[export_method]
    async fn multiply(&self, args: (i32, i32)) -> Result<i32, String> {
        Ok(args.0 * args.1)
    }

    #[export_method]
    async fn divide(&self, args: (i32, i32)) -> Result<i32, String> {
        Ok(args.0 / args.1)
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let abacus = Arc::new(Abacus{});
    let calculator = Arc::new(Calculator{});
    let listener = TcpListener::bind(addr).await.unwrap();
    let server = Server::builder()
        .register(abacus)
        .register(calculator)
        .build();

    log::info!("Starting server at {}", &addr);
    server.accept(listener).await.unwrap()
}
