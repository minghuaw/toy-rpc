//! Custom definition of rpc request and response headers

use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU16;

use crate::{codec::RequestDeserializer, error::Error, service::{ArcAsyncServiceCall, HandlerResult}};

/// Type of message id is u16
pub type MessageId = u16;

/// Atomic type of MessageId
pub type AtomicMessageId = AtomicU16;

/// Returning the metadata
pub trait Metadata {
    fn get_id(&self) -> MessageId;
}

/// Header of a request
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

/// Header of a response
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

/// The Error message that will be sent over for a error response
#[derive(Serialize, Deserialize)]
pub(crate) enum ErrorMessage {
    InvalidArgument,
    ServiceNotFound,
    MethodNotFound,
    ExecutionError(String),
}

impl ErrorMessage {
    pub(crate) fn from_err(err: Error) -> Result<Self, Error> {
        match err {
            Error::InvalidArgument => Ok(Self::InvalidArgument),
            Error::ServiceNotFound => Ok(Self::ServiceNotFound),
            Error::MethodNotFound => Ok(Self::MethodNotFound),
            Error::ExecutionError(s) => Ok(Self::ExecutionError(s.into())),
            e @ Error::IoError(_) => Err(e),
            e @ Error::ParseError(_) => Err(e),
            e @ Error::Internal(_) => Err(e),
        }
    }
}

pub(crate) struct ExecutionMessage {
    pub call: ArcAsyncServiceCall,
    pub id: MessageId,
    // pub service: String,
    pub method: String,
    pub deserializer: RequestDeserializer,
}

pub(crate) struct ResultMessage {
    pub id: MessageId,
    pub result: HandlerResult
}
