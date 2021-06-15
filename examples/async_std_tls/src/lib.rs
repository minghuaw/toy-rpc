pub const ADDR: &str = "127.0.0.1:23333";

pub mod rpc {
    use toy_rpc::macros::export_impl;
    pub struct Echo { }
    
    #[export_impl]
    impl Echo {
        #[export_method]
        pub async fn echo_i32(&self, args: i32) -> Result<i32, String> {
            Ok(args)
        }
    }
}