//! PubSub support
use std::time::Duration;

use serde::{de::DeserializeOwned, Serialize};

use crate::message::MessageId;

pub const DEFAULT_PUB_RETRY_TIMEOUT: Duration = Duration::from_secs(10);

/// Trait for PubSub Topic
pub trait Topic {
    /// Message type of the topic
    type Item: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Name of the topic
    fn topic() -> String;
}

/// PubSub Sequence ID that is tracked by the PubSub server
#[derive(Debug, Clone)]
pub struct SeqId(pub MessageId);

impl SeqId {
    /// Creates a new sequence ID
    pub fn new(val: MessageId) -> Self {
        Self(val)
    }
}

/// Type state of AckMode. No Ack will be sent back.
#[derive(Clone)]
pub struct AckModeNone {}

/// Type state of AckMode. One Ack message will be sent back automatically upon recving the Publish message
#[derive(Clone)]
pub struct AckModeAuto {}

/// Type state of AckMode. The user need to manually acknowledge delivery of a Publish message
#[derive(Clone)]
pub struct AckModeManual {}
