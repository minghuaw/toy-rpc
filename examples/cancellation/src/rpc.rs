// use serde::{Deserialize, Serialize};
use std::time::Duration;
use toy_rpc::macros::export_impl;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "async_std_runtime")] {
        use async_std::task;
        async fn sleep(dur: Duration) {
            task::sleep(dur).await;
        }
    } else {
        use tokio::time;
        async fn sleep(dur: Duration) {
            time::sleep(dur).await;
        }
    }
}

pub struct Echo { }

#[cfg(any(
    feature = "async_std_runtime",
    feature = "tokio_runtime"
))]
#[export_impl]
impl Echo {
    pub async fn not_exported(&self, _: ()) -> Result<(), String> {
        println!("This is not an exported method");
        Ok(())
    }

    #[export_method]
    pub async fn echo_i32(&self, req: i32) -> Result<i32, String> {
        Ok(req)
    }

    #[export_method]
    pub async fn finite_loop(&self, _: ()) -> Result<(), String> {
        for counter in 0..500 {
            sleep(Duration::from_millis(500)).await;
            println!("infinite_loop counter: {}", &counter);
        }

        Ok(())
    }

    // #[export_method]
    // pub async fn long_sleep(&self, _: ()) -> Result<(), String> {
    //     println!("Start sleeping");
    //     sleep(Duration::from_secs(10)).await;
    //     println!("Sleeping ended");
    //     Ok(())
    // }
}