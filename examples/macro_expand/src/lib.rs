#![allow(dead_code)]

use toy_rpc::macros::{export_impl, export_trait, export_trait_impl, Topic};
use toy_rpc::Error;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

pub struct Foo { }
pub struct Foo2 { }

/* -------------------------------------------------------------------------- */
/*                                #[derive(Topic)]                            */
/* -------------------------------------------------------------------------- */
// #[derive(Serialize, Deserialize, Topic)]
// #[topic(rename="YourTopic", item="u32")]
// pub struct MyTopic(u32);


// =============================================================================
// #[export_trait]
// =============================================================================

// use anyhow::Result;
// use toy_rpc::Result;

#[async_trait]
#[export_trait(impl_for_client)]
pub trait AnotherExample {
    #[export_method]
    async fn one(&self, args: i32) -> Result<i32, anyhow::Error>;
    #[export_method]
    async fn two(&self, req: bool) -> i32;
    #[export_method]
    async fn three(&self, args: ()) -> Result<i32, anyhow::Error>;
}

#[async_trait]
#[export_trait_impl]
impl AnotherExample for Foo2 {
    async fn one(&self, args: i32) -> Result<i32, anyhow::Error> {
        Ok(1)
    }

    async fn two(&self, req: bool) -> i32 {
        2
    }

    async fn three(&self, req: ()) -> Result<i32, anyhow::Error> {
        Ok(3)
    }
}

// =============================================================================
// #[async_trait]
// #[export_impl]
// =============================================================================

// #[async_trait]
// pub trait Example {
//     async fn foo(&self, args: i32) -> Result<i32, String>;
//     async fn not_exported(&self);
// }

// #[export_impl]
// #[async_trait]
// impl Example for Foo {
//     #[export_method]
//     async fn foo(&self, args: i32) -> Result<i32, String> {
//         Ok(args)
//     }

//     async fn not_exported(&self) {
//         println!("this is not exported");
//     }
// }

// // =============================================================================
// // #[export_impl]
// // =============================================================================

// use anyhow::Result;

// pub struct Bar { }

// #[export_impl]
// impl Bar {
//     #[export_method]
//     async fn bar(&self, args: i32) -> Result<i32> {
//         Ok(args)
//     }

//     #[export_method]
//     async fn get_string(&self, args: ()) -> String {
//         "hello".into()
//     }
// }