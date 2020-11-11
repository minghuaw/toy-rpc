use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use tide::Request;
use tide::prelude::*;

use toy_rpc::macros::{export_impl, service};
use toy_rpc::server::Server;

// use tide_integration::rpc;
use tide_integration::rpc::{FooRequest, FooResponse, Rpc};

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

#[derive(Debug, Deserialize)]
struct Animal {
    name: String,
    legs: u8,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let foo_service = Arc::new(FooService{
        counter: Mutex::new(0)
    });
    let server = Server::builder()
        .register("foo_service", service!(foo_service, FooService))
        .build();

    let mut app = tide::new();
    app.at("/orders/shoes").post(order_shoes);
    app.at("/rpc").nest( {
        server.handle_http()
    });

    app.listen(addr).await?;
    Ok(())
}

async fn order_shoes(mut req: Request<()>) -> tide::Result {
    let body = req.body_string().await?;
    println!("{:?}", body);

    Ok("".into())
    // let Animal { name, legs } = req.body_json().await?;
    // Ok(format!("Hello, {}! I've put in an order for {} shoes", name, legs).into())
}