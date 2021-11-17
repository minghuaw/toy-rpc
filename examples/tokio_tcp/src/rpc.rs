use std::time::Duration;
use toy_rpc::macros::{export_impl, export_trait};
use toy_rpc::Error;
use tokio::time;
use async_trait::async_trait;

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
    pub async fn finite_loop(&self, _: ()) -> Result<(), String> {
        for counter in 0i32..10 {
            time::sleep(Duration::from_millis(500)).await;
            println!("finite_loop counter: {}", &counter);
        }

        Ok(())
    }
}

#[async_trait]
#[export_trait(impl_for_client)] // All methods must be exported if client trait impl generation is enabled
pub trait Arith {
    #[export_method]
    async fn add(&self, args: (i32, i32)) -> Result<i32, Error>;

    #[export_method]
    async fn subtract(&self, args: (i32, i32)) -> Result<i32, Error>;

    // #[export_method]
    // async fn get_num_anyhow(&self, args:()) -> anyhow::Result<u32>;

    // #[export_method]
    // async fn get_str_anyhow(&self, args:()) -> Result<String, Error>;
}