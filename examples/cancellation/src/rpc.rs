// use serde::{Deserialize, Serialize};

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
        loop {

        }

        Ok(())
    }
}