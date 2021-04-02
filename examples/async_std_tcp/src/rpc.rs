use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use async_std::sync::Mutex;

use toy_rpc::macros::export_impl;

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

pub struct Foo {
    pub counter: Mutex<u32>,
}

#[async_trait]
#[export_impl]
impl Rpc for Foo {
    #[export_method]
    async fn echo(&self, req: FooRequest) -> Result<FooResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = FooResponse { a: req.a, b: req.b };
        Ok(res)
        // Err("echo error".into())
    }

    #[export_method]
    async fn increment_a(&self, req: FooRequest) -> Result<FooResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = FooResponse {
            a: req.a + 1,
            b: req.b,
        };
        Ok(res)
        // Err("increment_a error".into())
    }

    #[export_method]
    async fn increment_b(&self, req: FooRequest) -> Result<FooResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = FooResponse {
            a: req.a,
            b: req.b + 1,
        };

        Ok(res)
        // Err("increment_b error".into())
    }

    #[export_method]
    async fn get_counter(&self, _: ()) -> Result<u32, String> {
        let counter = self.counter.lock().await;
        let res = *counter;
        Ok(res)
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct BarRequest {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BarResponse {
    pub content: String,
    pub is_modified: bool,
}

pub struct Bar {}

#[export_impl]
impl Bar {
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

    #[export_method]
    pub async fn echo_error(&self, req: String) -> Result<(), String> {
        Err(req)
    }
}
