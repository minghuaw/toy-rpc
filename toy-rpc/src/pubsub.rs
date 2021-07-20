//! PubSub support
use serde::{de::DeserializeOwned, Serialize};

/// Trait for PubSub Topic
pub trait Topic {
    /// Message type of the topic
    type Item: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Name of the topic
    fn topic() -> String;
}

/// Type state for AckMode. No Ack will be sent back upon recving a new Publish item
pub struct AckModeNone { }

/// Type state for AckMode. One Ack message will be sent back automatically upon recving a new Publish item
pub struct AckModeAuto { }

/// Type state for AckMode. This is only allowed on Client. 
/// The client will need to manually Ack the delivery of a new Publish item
pub struct AckModeManual { }