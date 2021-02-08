pub mod rpc {
    use serde::{Serialize, Deserialize};
    use toy_rpc::macros::export_impl;
     
    use tokio::sync::Mutex; // uncomment this for the examples that use tokio runtim
    // use async_std::sync::Mutex; // uncomment this for the examples that use async-std runtime

    pub struct ExampleService {
        pub counter: Mutex<i32>
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ExampleRequest {
        pub a: u32,
    }
     
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ExampleResponse {
        a: u32,
    }

    #[async_trait::async_trait]
    trait Rpc {
        async fn echo(&self, req: ExampleRequest) -> Result<ExampleResponse, String>;
    }

    #[async_trait::async_trait]
    #[export_impl]
    impl Rpc for ExampleService {
        #[export_method]
        async fn echo(&self, req: ExampleRequest) -> Result<ExampleResponse, String> {
            let mut counter = self.counter.lock().await;
            *counter += 1;

            let res = ExampleResponse{ a: req.a };
            Ok(res)
        }
    }
}