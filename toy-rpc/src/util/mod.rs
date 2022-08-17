//! Utility traits and functions.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::service::AsyncHandler;

#[cfg(any(feature = "client", feature = "server"))]
use flume::Sender;
#[cfg(any(feature = "client", feature = "server"))]
use crate::error::Error;

#[cfg(any(feature = "client", feature = "server"))]
pub(crate) mod engine;

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
pub(crate) trait Conclude {
    fn conclude(&mut self);
}

#[cfg(feature = "async_std_runtime")]
impl Conclude for async_std::task::JoinHandle<Result<(), Error>> {
    fn conclude(&mut self) {
        async_std::task::block_on(self).unwrap_or_else(|err| log::error!("{}", err));
    }
}

#[cfg(feature = "tokio_runtime")]
impl Conclude for tokio::task::JoinHandle<Result<(), Error>> {
    fn conclude(&mut self) {
        match tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(self)) {
            Ok(res) => res.unwrap_or_else(|_| {}),
            Err(err) => log::error!("{}", err),
        }
    }
}

/// This trait simply cancel/abort the task during execution
#[async_trait]
pub(crate) trait Terminate {
    async fn terminate(self);
}

#[cfg(feature = "async_std_runtime")]
#[async_trait]
impl<T: Send> Terminate for async_std::task::JoinHandle<T> {
    async fn terminate(self) {
        self.cancel().await;
    }
}

#[cfg(feature = "tokio_runtime")]
#[async_trait]
impl<T: Send> Terminate for tokio::task::JoinHandle<T> {
    async fn terminate(self) {
        self.abort();
    }
}

#[cfg(any(feature = "client", feature = "server"))]
#[async_trait]
pub(crate) trait Broker {
    type Item;
    type WriterItem;

    async fn op(
        &mut self,
        item: Self::Item,
        tx: &Sender<Self::Item>,
    ) -> Result<Option<Self::WriterItem>, Error>;
}

#[cfg(any(feature = "client", feature = "server"))]
#[async_trait]
pub(crate) trait Reader {
    type BrokerItem;

    async fn handle_error(&mut self, error: Error) -> Result<Running, Error> {
        Err(error)
    }

    async fn op(
        &mut self,
    ) -> Option<Result<Self::BrokerItem, Error>>;
}

#[cfg(any(feature = "client", feature = "server"))]
#[async_trait]
pub(crate) trait Writer {
    type Item;

    async fn handle_error(&mut self, error: Error) -> Result<Running, Error> {
        Err(error)
    }

    async fn op(
        &mut self,
        item: Self::Item,
    ) -> Result<Running, Error>;
}

#[cfg(any(feature = "client", feature = "server"))]
pub(crate) enum Running {
    Continue,
    Stop,
}