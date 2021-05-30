#![allow(dead_code)]

use toy_rpc::macros::{export_impl, export_trait};
use async_trait::async_trait;

// =============================================================================
// #[export_trait]
// =============================================================================

#[async_trait]
#[export_trait]
pub trait AnotherExample {
    #[export_method]
    async fn one(&self, args: i32) -> Result<i32, String>;
    #[export_method]
    async fn two(&self, args: bool) -> Result<bool, String>;
    fn three();
}

// =============================================================================
// #[export_impl]
// =============================================================================

pub struct FooBar { }

#[async_trait]
pub trait Example {
    async fn foo(&self, args: i32) -> Result<i32, String>;
    async fn bar(&self, args: bool) -> Result<bool, String>;
    async fn not_exported(&self);
}

#[async_trait]
#[export_impl]
impl Example for FooBar {
// impl Example {
    #[export_method]
    async fn foo(&self, args: i32) -> Result<i32, String> {
        Ok(args)
    }

    #[export_method]
    async fn bar(&self, args: bool) -> Result<bool, String> {
        Ok(!args)
    }

    async fn not_exported(&self) {
        println!("this is not exported");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
