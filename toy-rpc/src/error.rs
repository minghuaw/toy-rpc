//! Custom errors

use std::fmt::Debug;

use crate::message::{ErrorMessage, MessageId};

pub(crate) type IoError = std::io::Error;

/// Custom error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Errors with IO including that from the transport layer.
    /// 
    /// This includes errors from reading/writing on either the TCP transport
    /// or websocket transport. If the underlying error isn't a native 
    /// `std::io::Error`, it will be converted to `std::io::Error` with `ErrorKind::Other`
    /// 
    /// This is expected to see changes in version 0.9.
    #[error("{0:?}")]
    IoError(#[from] std::io::Error),

    /// Errors with serialization/deserialization
    #[error("{0}")]
    ParseError(Box<dyn std::error::Error + Send + Sync>),

    /// Errors with server or client
    #[error("InternalError: {0}")]
    Internal(Box<dyn std::error::Error + Send + Sync>),

    /// The supplied argument for the function is invalid
    #[error("InvalidArgument")]
    InvalidArgument,

    /// The specified service is not found on server side
    #[error("ServiceNotFound")]
    ServiceNotFound,

    /// The specified method is not found on the specified service
    #[error("MethodNotFound")]
    MethodNotFound,

    /// Execution error returned by RPC method
    #[error("{0}")]
    ExecutionError(String),

    /// Cancellation error when an RPC call is cancelled
    #[error("Request ({0}) is canceled")]
    Canceled(MessageId),

    /// Timeout error when an RPC request timesout
    ///
    /// The timeout is tracked independently on the client and the server.
    /// Thus, the situation where a response comes back to the client just
    /// at the moment of timeout could happen and may still result in a timeout
    /// error.
    #[error("Request ({0}) reached timeout")]
    Timeout(MessageId),

    /// Maximum number of retries is reached before an Ack is received
    #[error("Maximum number of retries is reached for message {0}")]
    MaxRetriesReached(MessageId),
}

impl Error {
    pub(crate) fn from_err_msg(msg: ErrorMessage) -> Self {
        match msg {
            ErrorMessage::InvalidArgument => Self::InvalidArgument,
            ErrorMessage::ServiceNotFound => Self::ServiceNotFound,
            ErrorMessage::MethodNotFound => Self::MethodNotFound,
            ErrorMessage::ExecutionError(s) => Self::ExecutionError(s),
        }
    }
}

impl<T: 'static> From<flume::SendError<T>> for Error {
    fn from(_: flume::SendError<T>) -> Self {
        Self::Internal(format!("Cannot send internal message").into())
    }
}

impl From<ErrorMessage> for Error {
    fn from(msg: ErrorMessage) -> Self {
        Self::from_err_msg(msg)
    }
}

/// Convert from serde_json::Error to the custom Error
#[cfg(feature = "serde_json")]
impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        Error::ParseError(Box::new(err))
    }
}

/// Convert from bincode::Error to the custom Error
impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::ParseError(err)
    }
}

#[cfg(feature = "serde_cbor")]
impl From<serde_cbor::Error> for Error {
    fn from(err: serde_cbor::Error) -> Self {
        Error::ParseError(Box::new(err))
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::ParseError(Box::new(err))
    }
}

#[cfg(feature = "serde_rmp")]
impl From<rmp_serde::decode::Error> for Error {
    fn from(err: rmp_serde::decode::Error) -> Self {
        Error::ParseError(Box::new(err))
    }
}

#[cfg(feature = "serde_rmp")]
impl From<rmp_serde::encode::Error> for Error {
    fn from(err: rmp_serde::encode::Error) -> Self {
        Error::ParseError(Box::new(err))
    }
}

#[cfg(feature = "tokio_runtime")]
impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Internal(Box::new(err))
    }
}

#[cfg(feature = "http_actix_web")]
impl From<Error> for actix_web::Error {
    fn from(err: crate::error::Error) -> Self {
        // wrap error with actix_web::error::InternalError for now
        // TODO: imporve error handling
        actix_web::error::InternalError::new(err, actix_web::http::StatusCode::OK).into()
    }
}

#[cfg(feature = "http_actix_web")]
impl<T> From<actix::prelude::SendError<T>> for Error {
    fn from(err: actix::prelude::SendError<T>) -> Self {
        Self::Internal(format!("Cannot send internal message {:?}", err).into())
    }
}

#[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
impl From<tungstenite::Error> for crate::error::Error {
    fn from(err: tungstenite::Error) -> Self {
        Self::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            err.to_string(),
        ))
    }
}

impl From<erased_serde::Error> for crate::error::Error {
    fn from(err: erased_serde::Error) -> Self {
        Self::ParseError(Box::new(err))
    }
}

#[cfg(feature = "tls")]
impl From<webpki::InvalidDnsNameError> for crate::error::Error {
    fn from(err: webpki::InvalidDnsNameError) -> Self {
        Self::Internal(Box::new(err))
    }
}

impl From<String> for Error {
    fn from(val: String) -> Self {
        Self::ExecutionError(val)
    }
}

impl From<&str> for Error {
    fn from(val: &str) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

impl From<bool> for Error {
    fn from(val: bool) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

impl From<u8> for Error {
    fn from(val: u8) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

impl From<u16> for Error {
    fn from(val: u16) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

impl From<u32> for Error {
    fn from(val: u32) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

impl From<u64> for Error {
    fn from(val: u64) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

impl From<i8> for Error {
    fn from(val: i8) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

impl From<i16> for Error {
    fn from(val: i16) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

impl From<i32> for Error {
    fn from(val: i32) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

impl From<i64> for Error {
    fn from(val: i64) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for Error {
    fn from(val: anyhow::Error) -> Self {
        Self::ExecutionError(val.to_string())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "anyhow")]
    use super::*;

    #[test]
    fn test_conversion_to_anyhow() {}

    #[cfg(feature = "anyhow")]
    fn return_std_result() -> Result<(), Error> {
        let a: anyhow::Result<u32> = Ok(1);
        a?;

        let result: Result<(), Error> = Err(Error::MethodNotFound);
        result
    }

    #[cfg(feature = "anyhow")]
    fn return_anyhow_result() -> anyhow::Result<()> {
        let r = return_std_result()?;
        Ok(r)
    }
}
