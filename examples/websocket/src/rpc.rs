// use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use std::time::Duration;
// use toy_rpc::macros::{export_impl, export_trait};
use toy_rpc;

use super::sleep;

pub struct Echo { }

#[cfg(any(
    feature = "async_std_runtime",
    feature = "tokio_runtime"
))]
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
    pub async fn finite_loop(&self, _: ()) -> Result<(), String> {
        for counter in 0i32..500 {
            sleep(Duration::from_millis(500)).await;
            println!("finite_loop counter: {}", &counter);
        }

        Ok(())
    }

    #[export_method]
    pub async fn infinite_loop(&self, _: ()) -> Result<(), String> {
        let mut counter = 0i32;
        loop {
            sleep(Duration::from_millis(500)).await;
            println!("infinite_loop counter: {}", &counter);
            counter += 1;
        }
    }
}

#[async_trait]
#[export_trait]
pub trait Arith {
    #[export_method]
    async fn add(&self, args: (i32, i32)) -> Result<i32, String>;

    #[export_method]
    async fn subtract(&self, args: (i32, i32)) -> Result<i32, String>;
}