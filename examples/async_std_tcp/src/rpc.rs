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
    pub async fn echo_i32(&self, req: i32) -> i32 {
        req
    }

    #[export_method]
    pub async fn finite_loop(&self, _: ()) -> Result<(), String> {
        for counter in 0..500 {
            async_std::task::sleep(Duration::from_millis(500)).await;
            println!("finite_loop counter: {}", &counter);
        }

        Ok(())
    }
}