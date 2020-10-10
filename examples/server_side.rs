use async_std::net::TcpListener;
use async_std::task;
use async_trait::async_trait;
use env_logger;
use log;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use toy_rpc::macros::{export_impl, service};
use toy_rpc::server::Server;

#[derive(Serialize, Deserialize, Debug)]
struct EchoRequest {
    pub a: u32,
    pub b: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct EchoResponse {
    pub a: u32,
    pub b: u32,
}

struct EchoService {
    counter: Mutex<u32>,
}

#[async_trait]
trait EchoRpc {
    async fn echo(&self, req: EchoRequest) -> Result<EchoResponse, String>;
    async fn increment_a(&self, req: EchoRequest) -> Result<EchoResponse, String>;
    async fn increment_b(&self, req: EchoRequest) -> Result<EchoResponse, String>;
    async fn increment_counter(&self, req: EchoRequest) -> Result<EchoResponse, String>;
}

#[async_trait]
#[export_impl]
impl EchoRpc for EchoService {
    #[export_method]
    async fn echo(&self, req: EchoRequest) -> Result<EchoResponse, String> {
        let res = EchoResponse { a: req.a, b: req.b };
        Ok(res)
    }

    #[export_method]
    async fn increment_a(&self, req: EchoRequest) -> Result<EchoResponse, String> {
        let res = EchoResponse {
            a: req.a + 1,
            b: req.b,
        };
        Ok(res)
    }

    #[export_method]
    async fn increment_b(&self, req: EchoRequest) -> Result<EchoResponse, String> {
        let res = EchoResponse {
            a: req.a,
            b: req.b + 1,
        };
        Ok(res)
    }

    #[export_method]
    async fn increment_counter(&self, req: EchoRequest) -> Result<EchoResponse, String> {
        let mut _counter = self.counter.lock().map_err(|_| "")?;

        *_counter += 1;
        let res = EchoResponse {
            a: req.a,
            b: *_counter,
        };
        Ok(res)
    }
}

async fn run_server() {
    let echo_service = Arc::new(EchoService {
        counter: Mutex::new(0u32),
    });

    let addrs = "127.0.0.1:23333";

    log::info!("Starting server at {}", &addrs);
    let server = Server::builder()
        .register("echo_service", service!(echo_service, EchoService))
        .build();

    let listener = TcpListener::bind(addrs).await.unwrap();
    server.accept(listener).await.unwrap();
}

async fn async_main() {
    run_server().await;
}

fn main() {
    env_logger::init();

    task::block_on(async_main());
}
