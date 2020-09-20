use std::sync::Mutex;
// use async_std::sync::Arc;
use std::collections::HashMap;
// use lazy_static::lazy_static;
use toy_rpc::{
    // export_struct,
    export_impl,
    // export_method,
    // service::Handler
};

// #[export_struct]
struct EchoService {
    count: Mutex<i32>
}

#[export_impl]
impl EchoService {
    pub fn new() -> Self {
        Self {
            count: Mutex::new(0)
        }
    }

    #[export_method]
    pub fn echo(&self, a: i32) -> Result<i32, String> {
        println!("echo");
        Ok(a)
    }
}

fn main() {
    for k in static_toy_rpc_service_EchoService.keys() {
        println!("{}", k);
    }
}