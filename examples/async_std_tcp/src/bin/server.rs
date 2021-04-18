use async_std::net::TcpListener;
use async_std::sync::{Arc, Mutex};
use async_std::task;

use toy_rpc::Server;

// use async_std_tcp::rpc::*;
use async_std_tcp::rpc::{Foo, Bar};

#[async_std::main]
async fn main() {
    pretty_env_logger::init();

    let addr = "127.0.0.1:23333";
    let foo_service = Arc::new(Foo {
        counter: Mutex::new(0),
    });
    let bar_service = Arc::new(Bar {});

    let server = Server::builder()
        .register(foo_service)
        .register(bar_service)
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
