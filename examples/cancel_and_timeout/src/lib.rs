use cfg_if::cfg_if;
use std::time::Duration;

pub mod rpc;

cfg_if! {
    if #[cfg(feature = "async_std_runtime")] {
        use async_std::task;
        pub async fn sleep(dur: Duration) {
            task::sleep(dur).await;
        }
    } else {
        use tokio::time;
        pub async fn sleep(dur: Duration) {
            time::sleep(dur).await;
        }
    }
}