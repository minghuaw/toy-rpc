//! Custom definition of rpc request and response headers

use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU16;

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
