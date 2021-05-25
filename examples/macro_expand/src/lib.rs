#![allow(dead_code)]

use toy_rpc::macros::export_impl;
use async_trait::async_trait;
use std::sync::Arc;

pub struct FooBarService { }

#[async_trait]
pub trait ExampleService {
    async fn foo(&self, args: i32) -> Result<i32, String>;
    async fn bar(&self, args: bool) -> Result<bool, String>;
}

#[async_trait]
#[export_impl]
impl ExampleService for FooBarService {
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
    let example = Arc::new(FooBarService{});

}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
