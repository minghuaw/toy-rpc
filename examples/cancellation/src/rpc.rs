// use serde::{Deserialize, Serialize};
use async_std::task;
use std::time::Duration;
use toy_rpc::macros::export_impl;

pub struct Echo { }

#[export_impl]
impl Echo {
    pub async fn not_exported(&self, _: ()) -> Result<(), String> {
        println!("This is not an exported method");
        Ok(())
    }

    #[export_method]
    pub async fn echo_i32(&self, req: i32) -> Result<i32, String> {
        Ok(req)
    }

    #[export_method]
    pub async fn infinite_loop(&self, _: ()) -> Result<(), String> {
        for counter in 0..500 {
            task::sleep(Duration::from_millis(500)).await;
            println!("infinite_loop counter: {}", &counter);
        }

        Ok(())
    }
}