//! Custom definition of rpc request and response headers
use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU16;

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


/// The Error message that will be sent over for a error response
#[derive(Serialize, Deserialize)]
pub(crate) enum ErrorMessage {
    InvalidArgument,
    ServiceNotFound,
    MethodNotFound,
    ExecutionError(String),
}

#[cfg(feature = "client")]
pub(crate) type ClientRequestBody = Box<dyn erased_serde::Serialize + Send + Sync>;
#[cfg(feature = "client")]
pub(crate) type ClientResponseBody = Box<dyn erased_serde::Deserializer<'static> + Send>;
/// The serialized representation of the response body
#[cfg(feature = "client")]
pub(crate) type ClientResponseResult = Result<ClientResponseBody, ClientResponseBody>;

cfg_if! {
    if #[cfg(any(
        feature = "async_std_runtime",
        feature = "tokio_runtime"
    ))] {       
        pub(crate) const CANCELLATION_TOKEN: &str = "RPC_TASK_CANCELLATION";
        pub(crate) const CANCELLATION_TOKEN_DELIM: &str = ".";
        
        #[cfg(feature = "server")]
        use crate::{
            error::Error,
            codec::RequestDeserializer,
            service::{ArcAsyncServiceCall, HandlerResult},
        };
        
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
                }
            }
        }
        
        #[cfg(feature = "server")]
        #[cfg_attr(feature = "http_actix_web", derive(actix::Message))]
        #[cfg_attr(feature = "http_actix_web", rtype(result = "()"))]
        pub(crate) enum ExecutionMessage {
            Request {
                call: ArcAsyncServiceCall,
                id: MessageId,
                method: String,
                deserializer: RequestDeserializer,
            },
            Result(ExecutionResult),
            Cancel(MessageId),
            Stop,
        }
        
        #[cfg(feature = "server")]
        #[cfg_attr(feature = "http_actix_web", derive(actix::Message))]
        #[cfg_attr(feature = "http_actix_web", rtype(result = "()"))]
        pub(crate) struct ExecutionResult {
            pub id: MessageId,
            pub result: HandlerResult,
        }
    }
}


