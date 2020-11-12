use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FooRequest {
    pub a: u32,
    pub b: u32,
}

#[derive(Debug, Serialize, Deserialize)]
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
