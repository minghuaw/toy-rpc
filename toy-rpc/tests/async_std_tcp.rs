use anyhow::Result;

// use cfg_if::cfg_if;
// cfg_if!{
//     if #[cfg(all(
//         feature = "async_std_runtime",
//         not(feature = "http_tide"),
//         any(
//             feature = "serde_bincdoe",
//             feature = "serde_cbor",
//             feature = "serde_json",
//             feature = "serde_rmp",
//         )
//     ))] {
//     }
// }

use async_std::net::ToSocketAddrs;
use async_std::{net::TcpListener, task};
use futures::channel::oneshot::{channel, Receiver};
use std::sync::Arc;
use toy_rpc::macros::service;
use toy_rpc::{Client, Server};

mod rpc;

async fn test_client(addr: impl ToSocketAddrs, mut ready: Receiver<()>) -> Result<()> {
    let _ = ready.try_recv()?.expect("Error receiving ready");

    println!("Client received ready");

    let client = Client::dial(addr).await.expect("Error dialing server");

    let service_method = format!("{}.{}", rpc::COMMON_TEST_SERVICE_NAME, "get_magic_u8");
    let reply: u8 = client
        .async_call(service_method, ())
        .await
        .expect("Unexpected error executing RPC");

    assert_eq!(rpc::COMMON_TEST_MAGIC_U8, reply);
    println!("Client received correct RPC result");
    Ok(())
}

async fn run() {
    let addr = "127.0.0.1:8080";
    let (tx, rx) = channel::<()>();
    let common_test_service = Arc::new(rpc::CommonTestService::new());

    // start testing server
    let server = Server::builder()
        .register(
            rpc::COMMON_TEST_SERVICE_NAME,
            service!(common_test_service, rpc::CommonTestService),
        )
        .build();

    let listener = TcpListener::bind(addr)
        .await
        .expect("Cannot bind to address");

    let server_handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });

    println!("Starting server at {}", &addr);
    // let server_handle = task::spawn(test_server(addr.clone()));u

    tx.send(()).expect("Error sending ready");

    let client_handle = task::spawn(test_client(addr, rx));

    // stop server after all clients finishes
    client_handle.await.expect("Error testing client");

    server_handle.cancel().await;
}

#[test]
fn start_server_client_test() {
    task::block_on(run());
}
