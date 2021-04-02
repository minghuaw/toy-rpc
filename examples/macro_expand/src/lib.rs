#![allow(dead_code)]

use toy_rpc::macros::export_impl;
use toy_rpc::Server;
use async_trait::async_trait;
use std::sync::Arc;

pub struct Example { }

#[async_trait]
pub trait ExampleService {
    async fn foo(&self, args: i32) -> Result<i32, String>;
    async fn bar(&self, args: bool) -> Result<bool, String>;
}

#[async_trait]
#[export_impl]
impl ExampleService for Example {
// impl Example {
    #[export_method]
    async fn foo(&self, args: i32) -> Result<i32, String> {
        Ok(args)
    }

    #[export_method]
    async fn bar(&self, args: bool) -> Result<bool, String> {
        Ok(!args)
    }
}

fn expand_service() {
    let example = Arc::new(Example{});

    let _server = Server::builder()
        .register(example)
        .build();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
