use async_std::sync::{Mutex, Arc};
use async_std::net::TcpListener;
use toy_rpc_macros::{
    export_impl,
    service
};

// use toy_rpc_definitions::{Error, RpcError};
use toy_rpc::server::Server;
// use toy_rpc::async_service;

struct EchoService {
    count: Mutex<i32>
}

#[export_impl]
impl EchoService {
    pub fn new() -> Self {
        Self {
            count: Mutex::new(13)
        }
    }

    #[export_method]
    async fn echo(&self, a: i32) -> Result<i32, String> {
        Ok(a)
    }

    #[export_method]
    async fn add(&self, val: i32) -> Result<i32, String> {
        let mut count = self.count.lock().await;
        *count += val;
        let ret = *count;
        Ok(ret)
    }
}

#[async_std::main]
async fn main() {
    // let mut map: HashMap<&str, ArcAsyncHandler<EchoService>> = HashMap::new();
    // map.insert("echo", Arc::new(EchoService::echo_handler));
    // map.insert("add", Arc::new(EchoService::add_handler));

    for key in STATIC_TOY_RPC_SERVICE_ECHOSERVICE.keys() {
        println!("{:?}", key);
    }

    let echo_service = Arc::new(EchoService{count: Mutex::new(0)});
    // let echo_service_server = ;
    // let echo_service_server = async_service::build_service(echo_service.clone(), &*STATIC_TOY_RPC_SERVICE_ECHOSERVICE);
    let addrs = "127.0.0.1:23333";
    let server = Server::builder()
        .register("echo_service", service!(echo_service, EchoService))
        .build();

    let listener = TcpListener::bind(addrs).await.unwrap();
    server.accept(listener).await.unwrap();
}
