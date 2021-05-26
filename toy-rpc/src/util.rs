use async_trait::async_trait;
// use cfg_if::cfg_if;
use std::collections::HashMap;

use crate::service::AsyncHandler;

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
    async fn close(&mut self);
}

pub(crate) trait Terminate {
    fn terminate(self);
}