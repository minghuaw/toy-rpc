use actix_web::{web, App, HttpServer};
use anyhow::Result;
use flume::{Receiver, Sender};
use std::sync::Arc;
use toy_rpc::{Client, Server};

mod rpc;

async fn test_client(base: &str) -> Result<()> {
    let addr = format!("ws://{}/rpc/", base);
    let client = Client::dial_http(&addr)
        .await
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

    println!("Client received all correct RPC result");
    Ok(())
}

async fn start_server(base: &'static str) -> Result<()> {
    let common_test_service = Arc::new(rpc::CommonTest::new());

    let server = Server::builder().register(common_test_service).build();
    let app_data = web::Data::new(server);

    HttpServer::new(move || {
        App::new().service(
            web::scope("/rpc/")
                .app_data(app_data.clone())
                .configure(Server::scope_config),
        )
    })
    .bind(&base)?
    .run()
    .await?;

    Ok(())
}

async fn run(base: &'static str, server_is_ready: Sender<()>, rx: Receiver<()>) -> Result<()> {
    actix_rt::spawn(async move {
        start_server(base)
            .await
            .expect("Error starting test server");
    });

    println!("server started");
    server_is_ready
        .send_async(())
        .await
        .expect("Error sending ready");

    let _ = rx.recv_async().await.expect("Error receiving ready");
    Ok(())
}

// `#[actix_rt::test]` is needed to
#[actix_rt::test]
async fn http_actix_web_integration() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let (server_is_ready, is_server_ready) = flume::bounded(1);
    let (tx, rx) = flume::bounded(1);

    let handle = rt.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let _: () = is_server_ready
            .recv_async()
            .await
            .expect("Error receiving ready");
        test_client(rpc::ADDR).await.unwrap();
        tx.send_async(()).await.unwrap();
    });

    run(rpc::ADDR, server_is_ready, rx).await.unwrap();
    handle.await.unwrap();
}
