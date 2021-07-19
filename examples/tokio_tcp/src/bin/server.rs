use tokio::net::TcpListener;
use tokio::task;
use std::sync::{Arc};
use async_trait::async_trait;

use toy_rpc::Server;
use toy_rpc::macros::export_trait_impl;
use toy_rpc::Error;

use tokio_tcp::rpc::*;

fn return_anyhow_result() -> anyhow::Result<u32> {
    Ok(7)
}
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

    async fn get_num_anyhow(&self, _:()) -> anyhow::Result<u32> {
        let success = return_anyhow_result()?;
        Ok(success)
    }

    async fn get_str_anyhow(&self, _:()) -> Result<String, Error> {
        let success = return_anyhow_result()?;
        Ok(success.to_string())
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let echo_service = Arc::new(
        Echo { }
    );
    let arith = Arc::new(Abacus { });

    let server = Server::builder()
        .register(echo_service)
        .register(arith)
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();

    log::info!("Starting server at {}", &addr);

    let handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });
    handle.await.expect("Error");
}