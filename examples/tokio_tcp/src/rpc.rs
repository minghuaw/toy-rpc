use std::time::Duration;
use toy_rpc::macros::{export_impl, export_trait};
use tokio::time;
use async_trait::async_trait;

pub struct Echo { }

#[export_impl]
impl Echo {
    /// This shows a method that is not exported
    pub async fn not_exported(&self, _: ()) -> Result<(), String> {
        println!("This is not an exported method");
        Ok(())
    }

    /// This shows an exported method that returns a non-result type
    #[export_method]
    pub async fn echo_i32(&self, req: i32) -> i32 {
        req
    }

    /// This shows an exported method that returns a unit type `()`
    #[export_method]
    pub async fn finite_loop(&self, _: ()) {
        for counter in 0i32..10 {
            time::sleep(Duration::from_millis(500)).await;
            println!("finite_loop counter: {}", &counter);
        }
    }

    /// This shows an exported method that returns a Result type
    #[export_method]
    pub async fn echo_if_equal_to_one(&self, req: i32) -> Result<i32, i32> {
        match req {
            1 => Ok(req),
            _ => Err(req), // This will present on the client call as `Error::ExecutionError`
        }
    }
}

/// Here we define a trait `Arith` that comes with default implementations
/// for all of its methods
#[async_trait]
#[export_trait] 
pub trait Arith {
    /// Addition
    #[export_method]
    async fn add(&self, args: (i32, i32)) -> i32 {
        args.0 + args.1
    }

    /// Subtraction
    #[export_method]
    async fn subtract(&self, args: (i32, i32)) -> i32 {
        args.0 - args.1
    }

    /// Multiplication
    #[export_method]
    async fn multiply(&self, args: (i32, i32)) -> i32 {
        args.0 * args.1
    }

    /// Division. We cannot divide by zero
    #[export_method]
    async fn divide(&self, args: (i32, i32)) -> Result<i32, String> {
        let (numerator, denominator) = args;
        match denominator {
            0 => return Err("Divide by zero!".to_string()),
            _ => Ok( numerator / denominator )
        }
    }
}

/// Here we are going to define another trait (service) tha is almost 
/// identical to `Arith` shown above, but we are going to generate the trait
/// impl for the client using `impl_for_client` argument in our `#[export_trait]`
/// attribute. Plus, we will not going to supply a default implementation either.
#[async_trait]
/// All methods must be exported if client trait impl generation is enabled.
/// If `impl_for_client` is enabled, all methods in the trait must return 
/// a Result
#[export_trait(impl_for_client)]
pub trait Arith2 {
    #[export_method]
    async fn add(&self, args: (i32, i32)) -> anyhow::Result<i32>;

    #[export_method]
    async fn subtract(&self, args: (i32, i32)) -> anyhow::Result<i32>;

    #[export_method]
    async fn multiply(&self, args: (i32, i32)) -> anyhow::Result<i32>;

    #[export_method]
    async fn divide(&self, args: (i32, i32)) -> anyhow::Result<i32>;
}