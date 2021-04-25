use std::collections::HashMap;

use crate::service::AsyncHandler;

/// Helper trait for service registration
pub trait RegisterService {
    /// Helper function that returns a hashmap of the RPC service method handlers
    fn handlers() -> &'static HashMap<&'static str, AsyncHandler<Self>>;

    /// Helper function that returns the name of the service struct
    ///
    /// For a struct defined as `pub struct Foo { }`, the default name will be `"Foo"`.
    fn default_name() -> &'static str;
}