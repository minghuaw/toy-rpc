use std::{net::SocketAddr, sync::Arc};
use anyhow::Result;
use tokio::task;
use futures::channel::oneshot::{channel, Receiver};
use warp::Filter;

use toy_rpc::{Client, Server};

mod rpc;

async fn test_client(base: &str, mut ready: Receiver<()>) -> Result<()> {
    let _ = ready.try_recv()?.expect("Error receiving ready");
    println!("Client received ready");
    
    let addr = format!("ws://{}/rpc/", base);
    let client = Client::dial_http(&addr).await
        .expect("Error dialing http server");

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
    Ok(())
}

async fn run(base: &'static str) {
    let (tx, rx) = channel::<()>();
    let common_test_service = Arc::new(rpc::CommonTest::new());

    // start testing server
    let server = Server::builder()
        .register(common_test_service)
        .build();

    let routes = warp::path("rpc")
        .and(server.into_boxed_filter());

    let addr: SocketAddr = base.parse().expect("Unable to parse addr");
    let server_handle = task::spawn(async move {
        warp::serve(routes).run(addr).await;
    });
    tx.send(()).expect("Error sending ready");
    let client_handle = task::spawn(test_client(base, rx));

    client_handle.await
        .expect("Error awaiting client task")
        .expect("Error testing client");
    // ending server task
    server_handle.abort();
}

#[test]
fn http_warp_integration() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run(rpc::ADDR));
}