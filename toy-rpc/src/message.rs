//! Custom definition of rpc request and response headers

use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU16;

use crate::{codec::RequestDeserializer, error::Error, service::{ArcAsyncServiceCall, HandlerResult}};

pub(crate) const CANCELLATION_TOKEN: &str = "RPC_TASK_CANCELLATION";

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

pub(crate) type RequestBody = Box<dyn erased_serde::Serialize + Send + Sync>;

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

pub(crate) type ResponseBody = Box<dyn erased_serde::Deserializer<'static> + Send>;

/// The serialized representation of the response body
pub(crate) type ResponseResult = Result<ResponseBody, ResponseBody>;

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
            e @ Error::Canceled(_) => Err(e)
        }
    }
}

pub(crate) enum ExecutionMessage {
    Request {
        call: ArcAsyncServiceCall,
        id: MessageId,
        method: String,
        deserializer: RequestDeserializer
    },
    Cancel(MessageId)
}

/// The 
// pub(crate) struct ExecutionInfo {
//     pub call: ArcAsyncServiceCall,
//     pub id: MessageId,
//     pub method: String,
//     pub deserializer: RequestDeserializer,
// }

pub(crate) struct ExecutionResult {
    pub id: MessageId,
    pub result: HandlerResult
}

// pub(crate) enum ClientReaderControl {
//     Stop,
//     NewMessage(oneshot::Sender<ResponseResult>)
// }