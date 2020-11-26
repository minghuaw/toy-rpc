use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),

    ParseError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    // ParseError{ source: &'static (dyn std::error::Error + Send) },
    TransportError {
        msg: String,
    },

    NoneError,

    RpcError(RpcError),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IoError(ref e) => Some(e),
            Error::ParseError { source } => Some(&**source),
            Error::TransportError { .. } => None,
            Error::NoneError => None,
            Error::RpcError(ref e) => Some(e),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IoError(ref err) => err.fmt(f),
            Error::ParseError { source } => std::fmt::Display::fmt(&*source, f),
            Error::TransportError { msg } => write!(f, "Transport Error Message: {}", msg),
            Error::NoneError => write!(f, "None error"),
            Error::RpcError(ref err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RpcError {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError(String),
}

impl std::error::Error for RpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RpcError::ParseError => None,
            RpcError::InvalidRequest => None,
            RpcError::MethodNotFound => None,
            RpcError::InvalidParams => None,
            RpcError::InternalError => None,
            RpcError::ServerError(_) => None,
        }
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::ParseError => write!(f, "Parse error"),
            RpcError::InvalidRequest => write!(f, "Invalid request"),
            RpcError::MethodNotFound => write!(f, "Method not found"),
            RpcError::InvalidParams => write!(f, "Invalid parameters"),
            RpcError::InternalError => write!(f, "Internal error"),
            RpcError::ServerError(ref s) => write!(f, "Server error {}", s),
        }
    }
}

impl From<RpcError> for Box<dyn std::error::Error + Send> {
    fn from(err: RpcError) -> Self {
        Box::new(err)
    }
}

/// Convert from serde_json::Error to the custom Error
#[cfg(feature = "serde_json")]
impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        Error::ParseError {
            source: Box::new(err),
        }
    }
}

/// Convert from bincode::Error to the custom Error
#[cfg(feature = "serde_bincode")]
impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::ParseError { source: err }
    }
}

#[cfg(feature = "serde_cbor")]
impl From<serde_cbor::Error> for Error {
    fn from(err: serde_cbor::Error) -> Self {
        Error::ParseError {
            source: Box::new(err),
        }
    }
}

#[cfg(feature = "surf")]
impl From<surf::http::url::ParseError> for Error {
    fn from(err: surf::http::url::ParseError) -> Self {
        Error::ParseError {
            source: Box::new(err),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn err_to_string() {
        let e = Error::NoneError;

        println!("{}", e.to_string());
    }
}
