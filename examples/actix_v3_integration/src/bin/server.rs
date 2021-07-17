use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use actix::clock::{delay_for, Duration};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::SinkExt; // This is needed for publisher

use toy_rpc::macros::{export_impl};
use toy_rpc::Server;

use actix_v3_integration::{
    Count,
    rpc::{Rpc, BarService, FooRequest, FooResponse}
};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("hello")
}

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
    let mut publisher = server.publisher::<Count>();

    actix::spawn(async move {
        let mut count: u32 = 1;
        loop {
            println!("Send to publisher");
            publisher.send(Count(count)).await.unwrap();
            count += 1;
            delay_for(Duration::from_millis(1000)).await;
        }
    });

    let app_data = web::Data::new(server);

    HttpServer::new(
        move || {
            App::new()
                .service(hello)
                .service(
                    web::scope("/rpc/")
                        .app_data(app_data.clone())
                        // .configure(Server::scope_config)
                        .configure(Server::handle_http())
                )
        }
    )
    .bind(addr)?
    .run()
    .await
}