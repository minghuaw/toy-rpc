#![allow(dead_code)]

use toy_rpc::macros::{export_impl, export_trait, Topic};
use toy_rpc::Error;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
/* -------------------------------------------------------------------------- */
/*                                #[derive(Topic)]                            */
/* -------------------------------------------------------------------------- */
#[derive(Serialize, Deserialize, Topic)]
#[topic(rename="YourTopic")]
pub struct MyTopic(u32);


// =============================================================================
// #[export_trait]
// =============================================================================

#[async_trait]
#[export_trait(impl_for_client)]
pub trait AnotherExample {
    #[export_method]
    async fn one(&self, args: i32) -> Result<i32, Error>;
    #[export_method]
    async fn two(&self, req: bool) -> Result<bool, Error>;
}

// =============================================================================
// #[async_trait]
// #[export_impl]
// =============================================================================

pub struct Foo { }

#[async_trait]
pub trait Example {
    async fn foo(&self, args: i32) -> Result<i32, String>;
    async fn not_exported(&self);
}

#[async_trait]
#[export_impl]
impl Example for Foo {
// impl Example {
    #[export_method]
    async fn foo(&self, args: i32) -> Result<i32, String> {
        Ok(args)
    }

    async fn not_exported(&self) {
        println!("this is not exported");
    }
}

// // =============================================================================
// // #[export_impl]
// // =============================================================================

// pub struct Bar { }

// #[export_impl]
// impl Bar {
//     #[export_method]
//     async fn bar(&self, args: i32) -> Result<i32, String> {
//         Ok(args)
//     }
// }