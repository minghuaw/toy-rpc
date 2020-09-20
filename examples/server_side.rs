use async_std::net::TcpListener;
use async_std::task;
use env_logger;
use lazy_static::lazy_static;
use log;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use toy_rpc::server::Server;
use toy_rpc::service::Handler;
use toy_rpc::service::Service;
use toy_rpc::macros::{
    export_impl,
    service
};

#[derive(Serialize, Deserialize, Debug)]
struct EchoRequest {
    pub a: u32,
    pub b: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct EchoResponse {
    pub a: u32,
    pub b: u32,
}

struct EchoService {
    counter: Mutex<u32>,
}

trait EchoRpc {
    fn echo(&self, req: EchoRequest) -> Result<EchoResponse, String>;
    fn increment_a(&self, req: EchoRequest) -> Result<EchoResponse, String>;
    fn increment_b(&self, req: EchoRequest) -> Result<EchoResponse, String>;
    fn increment_counter(&self, req: EchoRequest) -> Result<EchoResponse, String>;
}

#[export_impl]
impl EchoRpc for EchoService {
    #[export_method]
    fn echo(&self, req: EchoRequest) -> Result<EchoResponse, String> {
        let res = EchoResponse { a: req.a, b: req.b };

        Ok(res)
    }

    #[export_method]
    fn increment_a(&self, req: EchoRequest) -> Result<EchoResponse, String> {
        let res = EchoResponse {
            a: req.a + 1,
            b: req.b,
        };

        Ok(res)
    }

    #[export_method]
    fn increment_b(&self, req: EchoRequest) -> Result<EchoResponse, String> {
        let res = EchoResponse {
            a: req.a,
            b: req.b + 1,
        };

        Ok(res)
    }

    #[export_method]
    fn increment_counter(&self, req: EchoRequest) -> Result<EchoResponse, String> {
        let mut _counter = self.counter.lock().map_err(|_| "")?;

        *_counter += 1;
        let res = EchoResponse {
            a: req.a,
            b: *_counter,
        };
        Ok(res)
    }
}

// lazy_static! {
//     static ref HANDLERS: HashMap<&'static str, Handler<EchoService>> = {
//         let builder = Service::<EchoService>::builder()
//             .register_method("echo", EchoService::echo)
//             .register_method("increment_a", EchoService::increment_a)
//             .register_method("increment_b", EchoService::increment_b)
//             .register_method("increment_counter", EchoService::increment_counter);

//         builder.handlers
//     };
// }

async fn run_server() {
    let echo = Arc::new(EchoService {
        counter: Mutex::new(0u32),
    });
    // let echo_service = Service::builder()
    //     .register_state(Arc::new(echo))
    //     .register_method("echo", EchoService::echo)
    //     .register_method("increment_a", EchoService::increment_a)
    //     .register_method("increment_b", EchoService::increment_b)
    //     .register_method("increment_counter", EchoService::increment_counter)
    //     .build();

    // let echo_service = Service::builder()
    //     .register_state(Arc::new(echo))
    //     .register_handlers(&*HANDLERS)
    //     .build();

    let addrs = "127.0.0.1:23333";
    // let fut = accept_loop(addrs);

    log::info!("Starting server at {}", &addrs);
    let server = Server::builder().register("service", service!(echo, EchoService)).build();

    // server.serve_tcp(addrs).await.unwrap();
    let listener = TcpListener::bind(addrs).await.unwrap();
    server.accept(listener).await.unwrap();
}

async fn async_main() {
    run_server().await;
}

fn main() {
    env_logger::init();

    task::block_on(async_main());
}
