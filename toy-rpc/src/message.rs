//! Custom definition of rpc request and response headers

use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, sync::atomic::AtomicU16};

use crate::error::Error;

pub type MessageId = u16;
pub type AtomicMessageId = AtomicU16;

/// Returning the metadata
pub trait Metadata {
    fn get_id(&self) -> MessageId;
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RequestHeader {
    pub id: MessageId,
    pub service_method: String,
}

impl Metadata for RequestHeader {
    fn get_id(&self) -> MessageId {
        self.id
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ResponseHeader {
    pub id: MessageId,
    pub is_error: bool,
}

impl Metadata for ResponseHeader {
    fn get_id(&self) -> MessageId {
        self.id
    }
}

/// Client should be able to gracefully shutdown the connection by
/// sending some kind of closing message
#[async_trait::async_trait]
pub trait GracefulShutdown {
    async fn close(&mut self);
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ErrorMessage {
    InvalidArgument,
    ServiceNotFound,
    MethodNotFound,
    ExecutionError(String),
}

impl ErrorMessage {
    pub(crate) fn from_err(err: impl Borrow<Error>) -> Option<Self> {
        match err.borrow() {
            Error::InvalidArgument => Some(Self::InvalidArgument),
            Error::ServiceNotFound => Some(Self::ServiceNotFound),
            Error::MethodNotFound => Some(Self::MethodNotFound),
            Error::ExecutionError(s) => Some(Self::ExecutionError(s.into())),
            Error::IoError(_) => None,
            Error::ParseError(_) => None,
            Error::Internal(_) => None,
        }
    }
}
