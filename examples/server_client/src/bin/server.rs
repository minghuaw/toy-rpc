use async_std::net::TcpListener;
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use async_std::task;

use toy_rpc::macros::{export_impl, service};
use toy_rpc::server::Server;

use server_client::rpc;
use server_client::rpc::{BarService, FooRequest, FooResponse, Rpc};

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
        let counter = self.counter.lock().await;
        let res = *counter;
        Ok(res)
    }
}

#[async_std::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let foo_service = Arc::new(FooService {
        counter: Mutex::new(0),
    });
    let bar_service = Arc::new(BarService {});

    let server = Server::builder()
        .register("foo_service", service!(foo_service, FooService))
        .register("bar_service", service!(bar_service, rpc::BarService))
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();

    log::info!("Starting server at {}", &addr);

    // server.accept(listener).await.unwrap();

    let handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });

    // handle.await;
    task::sleep(std::time::Duration::from_secs(10)).await;
}
