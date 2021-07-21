//! PubSub support
use serde::{de::DeserializeOwned, Serialize};

/// Trait for PubSub Topic
pub trait Topic {
    /// Message type of the topic
    type Item: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Name of the topic
    fn topic() -> String;
}

/// Type state of AckMode. No Ack will be sent back.
#[derive(Clone)]
pub struct AckModeNone { }

/// Type state of AckMode. One Ack message will be sent back automatically upon recving the Publish message
#[derive(Clone)]
pub struct AckModeAuto { }

/// Type state of AckMode. The user need to manually acknowledge delivery of a Publish message
#[derive(Clone)]
pub struct AckModeManual { }