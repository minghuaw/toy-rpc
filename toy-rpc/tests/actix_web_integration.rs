// use std::sync::Arc;
// use anyhow::Result;
// use tokio::task;
// // use futures::channel::oneshot::{channel, Receiver};
// use actix_web::{App, HttpServer, web};

// use toy_rpc::{Client, Server};

// mod rpc;

// async fn test_client(base: &str) -> Result<()> {
//     // let _ = ready.try_recv()?.expect("Error receiving ready");
//     // println!("Client received ready");

//     actix_rt::time::delay_for(std::time::Duration::from_secs(2)).await;

//     let addr = format!("ws://{}/rpc/", base);
//     let client = Client::dial_http(&addr).await
//         .expect("Error dialing http server");

//     panic!("intentional panic");
//     rpc::test_get_magic_u8(&client).await;
//     rpc::test_get_magic_u16(&client).await;
//     rpc::test_get_magic_u32(&client).await;
//     rpc::test_get_magic_u64(&client).await;
//     rpc::test_get_magic_i8(&client).await;
//     rpc::test_get_magic_i16(&client).await;
//     rpc::test_get_magic_i32(&client).await;
//     rpc::test_get_magic_i64(&client).await;
//     rpc::test_get_magic_bool(&client).await;
//     rpc::test_get_magic_str(&client).await;
//     rpc::test_imcomplete_service_method(&client).await;
//     rpc::test_service_not_found(&client).await;
//     rpc::test_method_not_found(&client).await;
//     rpc::test_execution_error(&client).await;
//     Ok(())
// }

// // async fn run(base: &'static str) {
//     // let (tx, rx) = channel::<()>();
//     // let common_test_service = Arc::new(rpc::CommonTest::new());

//     // // start testing server
//     // let server = Server::builder()
//     //     .register(common_test_service)
//     //     .build();

//     // // let local = task::LocalSet::new();

//     // let app_data = web::Data::new(server);
//     // let server_handle = local.spawn_local(
//     //     HttpServer::new(
//     //         move || {
//     //             App::new()
//     //             .service(
//     //                 web::scope("/rpc/")
//     //                 .app_data(app_data.clone())
//     //                 .configure(Server::handle_http())
//     //             )
//     //         }
//     //     )
//     //     .bind(base).unwrap()
//     //     .run()
//     // );

//     // tx.send(()).expect("Error sending ready");
//     // let client_handle = task::spawn(test_client(base, rx));

//     // client_handle.await
//     //     .expect("Error awaiting client task")
//     //     .expect("Error testing client");
//     // // ending server task
//     // server_handle.abort();
// // }

// // #[actix_rt::test]
// #[test]
// fn http_actix_web_integration() {
//     let mut rt = actix_rt::Runtime::new().unwrap();
//     // rt.block_on(run(rpc::ADDR));

//     let base = rpc::ADDR;

//     // let (tx, rx) = channel::<()>();
//     let common_test_service = Arc::new(rpc::CommonTest::new());

//     // start testing server
//     let server = Server::builder()
//         .register(common_test_service)
//         .build();

//     // let local = task::LocalSet::new();

//     let app_data = web::Data::new(server);
//     rt.spawn(async move {
//         HttpServer::new(
//             move || {
//                 App::new()
//                 .service(
//                     web::scope("/rpc/")
//                     .app_data(app_data.clone())
//                     .configure(Server::handle_http())
//                 )
//             }
//         )
//         .bind(base).unwrap()
//         .run()
//         .await
//         .unwrap();
//     });

//     // tx.send(()).expect("Error sending ready");
//     rt.block_on(async move {
//         test_client(base).await.unwrap();
//     });

//     // client_handle.await
//     //     .expect("Error awaiting client task")
//     //     .expect("Error testing client");
//     // // ending server task
//     // server_handle.abort();

// }
