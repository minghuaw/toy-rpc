use warp::Filter;
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;

use toy_rpc::macros::{export_impl, service};
use toy_rpc::Server;

use warp_integration::rpc;
use warp_integration::rpc::{BarService, FooRequest, FooResponse, Rpc};

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

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // let addr = "127.0.0.1:23333";
    let foo_service = Arc::new(FooService {
        counter: Mutex::new(0),
    });
    let bar_service = Arc::new(BarService {});

    let server = Server::builder()
        .register("foo_service", service!(foo_service, FooService))
        .register("bar_service", service!(bar_service, rpc::BarService))
        .build();

    // let server = Arc::new(server);

    // let state = warp::any().map(move || server.clone());
    // let rpc_route = warp::path(Server::handler_path())
    //     .and(state)
    //     .and(warp::ws())
    //     .map(Server::warp_websocket_handler);
    let routes = warp::path("rpc")
        .and(server.handle_http());
    
    warp::serve(routes).run(([127, 0, 0, 1], 23333)).await;
}