use cfg_if::cfg_if;
use toy_rpc::Server;
use cancel_and_timeout::rpc::{Echo};

cfg_if! {
    if #[cfg(feature = "async_std_runtime")] {
        use async_std::net::TcpListener;
        use std::sync::{Arc};
        use async_std::task;
        
        #[async_std::main]
        async fn main() {
            env_logger::init();
        
            let addr = "127.0.0.1:23333";
            let echo_service = Arc::new(
                Echo { }
            );
        
            let server = Server::builder()
                .register(echo_service)
                .build();
        
            let listener = TcpListener::bind(addr).await.unwrap();
        
            log::info!("Starting server at {}", &addr);
        
            let handle = task::spawn(async move {
                server.accept(listener).await.unwrap();
            });
            handle.await;
        }
    } else if #[cfg(feature = "tokio_runtime")] {
        use tokio::net::TcpListener;
        use std::sync::{Arc};
        use tokio::task;
        
        #[tokio::main]
        async fn main() {
            env_logger::init();
        
            let addr = "127.0.0.1:23333";
            let echo_service = Arc::new(
                Echo { }
            );
        
            let server = Server::builder()
                .register(echo_service)
                .build();
        
            let listener = TcpListener::bind(addr).await.unwrap();
        
            log::info!("Starting server at {}", &addr);
        
            let handle = task::spawn(async move {
                server.accept(listener).await.unwrap();
            });
            handle.await.unwrap();
        }
    }
}


