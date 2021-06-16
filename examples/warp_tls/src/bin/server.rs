use warp::Filter;
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;

use toy_rpc::macros::{export_impl};
use toy_rpc::Server;

use warp_tls::rpc::{BarService, FooRequest, FooResponse, Rpc};

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

// This is for demonstration purpose only
const SERVER_CERT_PATH: &str = "certs/service.pem";
const SERVER_KEY_PATH: &str = "certs/service.key";

// fn load_certs(path: &str) -> Result<Vec<Certificate>> {
//     certs(&mut BufReader::new(File::open(Path::new(path))?))
//         .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert").into())
// }

// fn load_keys(path: &str) -> Result<Vec<PrivateKey>> {
//     rsa_private_keys(&mut BufReader::new(File::open(Path::new(path))?))
//         .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key").into())
// }

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let foo_service = Arc::new(FooService {
        counter: Mutex::new(0),
    });
    let bar_service = Arc::new(BarService {});

    let server = Server::builder()
        .register(foo_service)
        .register(bar_service)
        .build();

    let routes = warp::path("rpc")
        .and(server.handle_http());

    // RPC will be served at "ws://127.0.0.1/rpc/_rpc_"
    warp::serve(routes)
        .tls()
        .cert_path(SERVER_CERT_PATH)
        .key_path(SERVER_KEY_PATH)
        .run(([127, 0, 0, 1], 23333)).await;
}