use async_std::sync::Arc;
use std::sync::Mutex;
// use std::collections::HashMap;
// use lazy_static::lazy_static;
use toy_rpc::macros::{
    // export_struct,
    async_export_impl,
    // export_method,
    // service::Handler
    service,
};
use toy_rpc_definitions::async_service::HandleService;

// #[export_struct]
struct EchoService {
    count: Mutex<i32>,
}

#[async_export_impl]
impl EchoService {
    pub fn new() -> Self {
        Self {
            count: Mutex::new(0),
        }
    }

    #[export_method]
    pub async fn echo(&self, a: i32) -> Result<i32, String> {
        let _count = self.count.lock().map_err(|_| "Cannot lock".to_string())?;
        println!("echo");
        println!("count {:?}", *_count);
        Ok(a)
    }
}

fn main() {
    for k in STATIC_TOY_RPC_SERVICE_ECHOSERVICE.keys() {
        println!("{}", k);
    }
    let a = Arc::new(EchoService::new());

    let a_service = service!(a, EchoService);
    println!("{:?}", a_service.get_method("echo").is_some());
}
