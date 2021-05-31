use async_trait::async_trait;
use toy_rpc::macros::export_trait;

#[async_trait]
#[export_trait]
pub trait Arith {
    #[export_method]
    async fn add(&self, args: (i32, i32)) -> Result<i32, String>;

    #[export_method]
    async fn subtract(&self, args: (i32, i32)) -> Result<i32, String>;

    fn say_hi(&self);
}