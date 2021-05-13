// use serde::{Deserialize, Serialize};

use toy_rpc::macros::export_impl;

pub struct Echo { }

#[export_impl]
impl Echo {
    #[export_method]
    pub async fn echo_i32(&self, req: i32) -> Result<i32, String> {
        Ok(req)
    }
}