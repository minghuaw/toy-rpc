pub mod rpc {
    use async_trait::async_trait;
    use toy_rpc::macros::{export_trait};
    use toy_rpc::Error;

    #[async_trait]
    #[export_trait(impl_for_client)]
    pub trait Arith {
        #[export_method]
        async fn add(&self, args: (i32, i32)) -> Result<i32, Error>;

        #[export_method]
        async fn subtract(&self, args: (i32, i32)) -> Result<i32, Error>;
    }
}