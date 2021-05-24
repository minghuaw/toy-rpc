use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use toy_rpc::macros::export_impl;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FooRequest {
    pub a: u32,
    pub b: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FooResponse {
    pub a: u32,
    pub b: u32,
}

#[async_trait]
pub trait Rpc {
    async fn echo(&self, req: FooRequest) -> Result<FooResponse, String>;
    async fn increment_a(&self, req: FooRequest) -> Result<FooResponse, String>;
    async fn increment_b(&self, req: FooRequest) -> Result<FooResponse, String>;
    async fn get_counter(&self, _: ()) -> Result<u32, String>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BarRequest {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BarResponse {
    pub content: String,
    pub is_modified: bool,
}

pub struct BarService {}

#[export_impl]
impl BarService {
    #[export_method]
    pub async fn echo(&self, req: BarRequest) -> Result<BarResponse, String> {
        let res = BarResponse {
            content: req.content,
            is_modified: false,
        };

        Ok(res)
    }

    #[export_method]
    pub async fn exclaim(&self, req: BarRequest) -> Result<BarResponse, String> {
        let res = BarResponse {
            content: format!("{}!", req.content),
            is_modified: true,
        };

        Ok(res)
    }
}
