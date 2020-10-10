use async_std::net::TcpListener;
use async_std::task;
use async_trait::async_trait;
use env_logger;
use log;
use serde::{Deserialize, Serialize};
use async_std::sync::{Arc, Mutex};

use toy_rpc::macros::{export_impl, service};
use toy_rpc::server::Server;

#[derive(Serialize, Deserialize, Debug)]
struct FooRequest {
    pub a: u32,
    pub b: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct FooResponse {
    pub a: u32,
    pub b: u32,
}

struct FooService {
    counter: Mutex<u32>,
}

#[async_trait]
trait Rpc {
    async fn echo(&self, req: FooRequest) -> Result<FooResponse, String>;
    async fn increment_a(&self, req: FooRequest) -> Result<FooResponse, String>;
    async fn increment_b(&self, req: FooRequest) -> Result<FooResponse, String>;
    async fn get_counter(&self, _: ()) -> Result<u32, String>;
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
    }

    #[export_method]
    async fn increment_b(&self, req: FooRequest) -> Result<FooResponse, String> {
        log::info!("incrementing b");
        let mut counter = self.counter.lock().await;
        *counter += 1;
        
        let res = FooResponse {
            a: req.a,
            b: req.b + 1,
        };
        Ok(res)
    }

    #[export_method]
    async fn get_counter(&self, _: ()) -> Result<u32, String> {
        log::info!("getting counter");
        let counter = self.counter.lock().await;
        let res = *counter;
        Ok(res)
    }
}

async fn run_server() {
    let echo_service = Arc::new(FooService {
        counter: Mutex::new(0u32),
    });

    let addrs = "127.0.0.1:23333";

    log::info!("Starting server at {}", &addrs);
    let server = Server::builder()
        .register("foo_service", service!(echo_service, FooService))
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
