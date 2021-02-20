use async_std::net::TcpListener;
use async_std::sync::{Arc, Mutex};
use async_std::task;
use async_trait::async_trait;

use toy_rpc::macros::{export_impl, service};
use toy_rpc::Server;

use async_std_tcp::rpc;
use async_std_tcp::rpc::*;

#[async_std::main]
async fn main() {
    pretty_env_logger::init();

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

    handle.await;
    // task::sleep(std::time::Duration::from_secs(10)).await;
}
