use tokio::net::TcpListener;
use tokio::task;
use std::sync::{Arc};
use async_trait::async_trait;

use toy_rpc::Server;
use toy_rpc::macros::export_trait_impl;

use tokio_tcp::rpc::*;

struct Abacus { }

/// We will simply use the default implementation provided in the trait 
/// definition for all except for add
#[async_trait]
#[export_trait_impl]
impl Arith for Abacus { 
    /// We are overriding the default implementation just for 
    /// the sake of demo
    async fn add(&self, args: (i32, i32)) -> i32 {
        args.0 + args.1
    }
}

/// For now, you need a separate type for a new service
struct Abacus2 { }

#[async_trait]
#[export_trait_impl]
impl Arith2 for Abacus2 {
    async fn add(&self, args: (i32, i32)) -> anyhow::Result<i32> {
        Ok(args.0 + args.1)
    }

    async fn subtract(&self, args: (i32, i32)) -> anyhow::Result<i32> {
        Ok(args.0 - args.1)
    }

    async fn multiply(&self, args: (i32, i32)) -> anyhow::Result<i32> {
        Ok(args.0 * args.1)
    }

    async fn divide(&self, args: (i32, i32)) -> anyhow::Result<i32> {
        let (numerator, denominator) = args;
        match denominator {
            0 => return Err(anyhow::anyhow!("Divide by zero!")),
            _ => Ok( numerator / denominator )
        }
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
    let arith2 = Arc::new(Abacus2 { });

    let server = Server::builder()
        .register(echo_service)
        .register(arith)
        .register(arith2)
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();

    log::info!("Starting server at {}", &addr);

    let handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });
    handle.await.expect("Error");
}