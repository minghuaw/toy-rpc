use serde::{Serialize, Deserialize};
use async_trait::async_trait;

#[derive(Debug, Serialize, Deserialize)]
pub struct FooRequest {
    pub a: u32,
    pub b: u32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FooResponse {
    pub a: u32,
    pub b: u32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BarRequest {
    pub content: String,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct BarResponse {
    pub content: String,
    pub is_modified: bool
}

#[async_trait]
pub trait Rpc {
    async fn echo(&self, req: FooRequest) -> Result<FooResponse, String>;
    async fn increment_a(&self, req: FooRequest) -> Result<FooResponse, String>;
    async fn increment_b(&self, req: FooRequest) -> Result<FooResponse, String>;
    async fn get_counter(&self, _: ()) -> Result<u32, String>;
}



