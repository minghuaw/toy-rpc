// use serde::{Deserialize, Serialize};
use async_std::task;
use std::time::Duration;
use toy_rpc::macros::export_impl;

pub struct Echo { }

#[export_impl]
impl Echo {
    pub async fn not_exported(&self, req: ()) -> Result<(), String> {
        println!("This is not an exported method");
        Ok(())
    }

    #[export_method]
    pub async fn echo_i32(&self, req: i32) -> Result<i32, String> {
        Ok(req)
    }

    #[export_method]
    pub async fn infinite_loop(&self, req: ()) -> Result<(), String> {
        let mut counter = 0;
        loop {
            task::sleep(Duration::from_millis(500)).await;
            println!("infinite loop counter {}", &counter);
            counter += 1;
        }

        Ok(())
    }
}