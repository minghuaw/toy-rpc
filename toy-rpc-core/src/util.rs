//! Utility traits and functions.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::service::AsyncHandler;

#[cfg(any(feature = "async-std", feature = "tokio"))]
use crate::error::Error;

/// Helper trait for service registration
pub trait RegisterService {
    /// Helper function that returns a hashmap of the RPC service method handlers
    fn handlers() -> HashMap<&'static str, AsyncHandler<Self>>;

    /// Helper function that returns the name of the service struct
    ///
    /// For a struct defined as `pub struct Foo { }`, the default name will be `"Foo"`.
    fn default_name() -> &'static str;
}

/// Client should be able to gracefully shutdown the connection by
/// sending some kind of closing message
#[async_trait]
pub trait GracefulShutdown {
    /// Closes the connection to allow graceful shutdown.
    async fn close(&mut self);
}

/// .await until the end of the task in a blocking manner
pub trait Conclude {
    /// Wait until the end of the task. Calls `block_on` internally
    fn conclude(&mut self);
}

#[cfg(feature = "async-std")]
impl Conclude for async_std::task::JoinHandle<Result<(), Error>> {
    fn conclude(&mut self) {
        async_std::task::block_on(self).unwrap_or_else(|err| log::error!("{}", err));
    }
}

#[cfg(feature = "tokio")]
impl Conclude for tokio::task::JoinHandle<Result<(), Error>> {
    fn conclude(&mut self) {
        match tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(self)) {
            Ok(res) => {
                res.unwrap_or_else(|_| { })
            },
            Err(err) => log::error!("{}", err),
        }
    }
}
