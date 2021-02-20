use toy_rpc_macros::{export_impl};
use async_trait::async_trait;

struct Example { }

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

    // #[export_method]
    // async fn bar(&self, args: bool) -> Result<bool, String> {
    //     Ok(!args)
    // }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
