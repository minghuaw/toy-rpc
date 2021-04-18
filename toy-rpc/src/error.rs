//! Custom errors
use std::io::ErrorKind;

use crate::message::ErrorMessage;

/// Custom error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Errors with IO including that from the transport layer
    #[error("{0:?}")]
    IoError(#[from] std::io::Error),

    /// Errors with serialization/deserialization
    #[error("{0}")]
    ParseError(Box<dyn std::error::Error + Send + Sync>),

    /// Errors with server or client
    #[error("{0}")]
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

// impl std::error::Error for Error {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             Error::IoError(ref e) => Some(e),
//             Error::ParseError(e) => Some(&**e),
//             Error::InvalidArgument => None,
//             Error::ServiceNotFound => None,
//             Error::MethodNotFound => None,
//             Error::ExecutionError(_) => None,
//         }
//     }
// }

// impl std::fmt::Display for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

//     }
// }

// #[derive(Debug)]
// pub enum Error {
//     IoError(std::io::Error),
//     ParseError(Box<dyn std::error::Error + Send + Sync>),
//     TransportError(String),
//     RpcError(RpcError),
// }

// impl std::error::Error for Error {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             Error::IoError(ref e) => Some(e),
//             Error::ParseError(source) => Some(&**source),
//             Error::TransportError { .. } => None,
//             // Error::NoneError => None,
//             Error::RpcError(ref e) => Some(e),
//         }
//     }
// }

// impl std::fmt::Display for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Error::IoError(ref err) => err.fmt(f),
//             Error::ParseError(source) => std::fmt::Display::fmt(&*source, f),
//             Error::TransportError(msg) => write!(f, "Transport Error Message: {}", msg),
//             // Error::NoneError => write!(f, "None error"),
//             Error::RpcError(ref err) => std::fmt::Display::fmt(err, f),
//         }
//     }
// }

// impl From<std::io::Error> for Error {
//     fn from(err: std::io::Error) -> Self {
//         Error::IoError(err)
//     }
// }

// #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
// pub enum RpcError {
//     ParseError,
//     InvalidRequest,
//     MethodNotFound,
//     InvalidParams,
//     InternalError,
//     ServerError(String),
// }

// impl std::error::Error for RpcError {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             RpcError::ParseError => None,
//             RpcError::InvalidRequest => None,
//             RpcError::MethodNotFound => None,
//             RpcError::InvalidParams => None,
//             RpcError::InternalError => None,
//             RpcError::ServerError(_) => None,
//         }
//     }
// }

// impl std::fmt::Display for RpcError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             RpcError::ParseError => write!(f, "Parse error"),
//             RpcError::InvalidRequest => write!(f, "Invalid request"),
//             RpcError::MethodNotFound => write!(f, "Method not found"),
//             RpcError::InvalidParams => write!(f, "Invalid parameters"),
//             RpcError::InternalError => write!(f, "Internal error"),
//             RpcError::ServerError(ref s) => write!(f, "Server error {}", s),
//         }
//     }
// }

// impl From<RpcError> for Box<dyn std::error::Error + Send> {
//     fn from(err: RpcError) -> Self {
//         Box::new(err)
//     }
// }

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

#[cfg(feature = "http_actix_web")]
impl From<Error> for actix_web::Error {
    fn from(err: crate::error::Error) -> Self {
        // wrap error with actix_web::error::InternalError for now
        // TODO: imporve error handling
        actix_web::error::InternalError::new(err, actix_web::http::StatusCode::OK).into()
    }
}

impl From<tungstenite::Error> for crate::error::Error {
    fn from(err: tungstenite::Error) -> Self {
        Self::IoError(std::io::Error::new(ErrorKind::InvalidData, err.to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn err_to_string() {
        let e = Error::IoError(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected eof",
        ));

        println!("{}", e.to_string());
    }
}
