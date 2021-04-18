use anyhow::Result;

use async_std::{net::TcpListener, task};
use futures::channel::oneshot::{channel, Receiver};
use std::sync::Arc;
use toy_rpc::{Client, Server};

mod rpc;

async fn test_client(addr: &'static str, mut ready: Receiver<()>) -> Result<()> {
    let _ = ready.try_recv()?.expect("Error receiving ready");

    println!("Client received ready");

    let client = Client::dial(addr).await.expect("Error dialing server");

    rpc::test_get_magic_u8(&client).await;
    rpc::test_get_magic_u16(&client).await;
    rpc::test_get_magic_u32(&client).await;
    rpc::test_get_magic_u64(&client).await;
    rpc::test_get_magic_i8(&client).await;
    rpc::test_get_magic_i16(&client).await;
    rpc::test_get_magic_i32(&client).await;
    rpc::test_get_magic_i64(&client).await;
    rpc::test_get_magic_bool(&client).await;
    rpc::test_get_magic_str(&client).await;
    rpc::test_imcomplete_service_method(&client).await;
    rpc::test_service_not_found(&client).await;
    rpc::test_method_not_found(&client).await;
    rpc::test_execution_error(&client).await;

    println!("Client received correct RPC result");
    Ok(())
}

async fn run(addr: &'static str) {
    let (tx, rx) = channel::<()>();
    let common_test_service = Arc::new(rpc::CommonTest::new());

    // start testing server
    let server = Server::builder().register(common_test_service).build();

    let listener = TcpListener::bind(addr)
        .await
        .expect("Cannot bind to address");

    let server_handle = task::spawn(async move {
        println!("Starting server at {}", &addr);
        server.accept(listener).await.unwrap();
    });

    tx.send(()).expect("Error sending ready");

    let client_handle = task::spawn(test_client(addr, rx));

    // stop server after all clients finishes
    client_handle.await.expect("Error testing client");

    server_handle.cancel().await;
}

#[test]
fn test_main() {
    task::block_on(run(rpc::ADDR));
}
