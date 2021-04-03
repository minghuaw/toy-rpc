// use async_std::net::TcpListener;
// use async_std::sync::{Arc, Mutex};
// use async_std::task;

use tokio::net::TcpListener;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;

use async_trait::async_trait;

use toy_rpc::macros::{export_impl};
use toy_rpc::Server;

use websocket::rpc::{BarService, FooRequest, FooResponse, Rpc};

pub struct FooService {
    counter: Mutex<u32>,
}

#[async_trait]
#[export_impl]
impl Rpc for FooService {
    #[export_method]
    async fn echo(&self, req: FooRequest) -> Result<FooResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = FooResponse { a: req.a, b: req.b };
        Ok(res)
        // Err("echo error".into())
    }

    #[export_method]
    async fn increment_a(&self, req: FooRequest) -> Result<FooResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = FooResponse {
            a: req.a + 1,
            b: req.b,
        };
        Ok(res)
        // Err("increment_a error".into())
    }

    #[export_method]
    async fn increment_b(&self, req: FooRequest) -> Result<FooResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = FooResponse {
            a: req.a,
            b: req.b + 1,
        };

        Ok(res)
        // Err("increment_b error".into())
    }

    #[export_method]
    async fn get_counter(&self, _: ()) -> Result<u32, String> {
        let counter = self.counter.lock().await;
        let res = *counter;
        Ok(res)
    }
}

// #[async_std::main]
#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let foo_service = Arc::new(FooService {
        counter: Mutex::new(0),
    });
    let bar_service = Arc::new(BarService {});

    let server = Server::builder()
        .register(foo_service)
        .register(bar_service)
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();
    
    let handle = task::spawn(async move {
        log::info!("Starting WebSocket server at {}", &addr);
        match server.accept_websocket(listener).await {
            Ok(_) => {},
            Err(e) => println!("{}", e)
        }
    });

    let _ = handle.await;
}
