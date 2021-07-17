//! ErrorMessage from server to client
use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU16;

/// Type of message id is u16
pub type MessageId = u16;

/// Atomic type of MessageId
pub type AtomicMessageId = AtomicU16;

/// Returning the metadata
pub trait Metadata {
    /// Gets the id from the metadata
    fn get_id(&self) -> MessageId;
}

/// The Error message that will be sent over for a error response
#[derive(Serialize, Deserialize)]
pub(crate) enum ErrorMessage {
    InvalidArgument,
    ServiceNotFound,
    MethodNotFound,
    ExecutionError(String),
}

cfg_if! {
    if #[cfg(any(
        feature = "async_std_runtime",
        feature = "tokio_runtime"
    ))] {
        /// Token indicating a cancellation request
        #[cfg(any(feature = "server", feature = "client"))]
        pub(crate) const CANCELLATION_TOKEN: &str = "RPC_TASK_CANCELLATION";
        #[cfg(any(feature = "server", feature = "client"))]
        pub(crate) const CANCELLATION_TOKEN_DELIM: &str = ".";

        #[cfg(feature = "server")]
        use crate::{error::Error};

        #[cfg(feature = "server")]
        impl ErrorMessage {
            pub(crate) fn from_err(err: Error) -> Result<Self, Error> {
                match err {
                    Error::InvalidArgument => Ok(Self::InvalidArgument),
                    Error::ServiceNotFound => Ok(Self::ServiceNotFound),
                    Error::MethodNotFound => Ok(Self::MethodNotFound),
                    Error::ExecutionError(s) => Ok(Self::ExecutionError(s)),
                    e @ Error::IoError(_) => Err(e),
                    e @ Error::ParseError(_) => Err(e),
                    e @ Error::Internal(_) => Err(e),
                    e @ Error::Canceled(_) => Err(e),
                    e @ Error::Timeout(_) => Err(e),
                }
            }
        }
    }
}
