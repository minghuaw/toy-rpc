//! PubSub support

use serde::{Serialize, de::DeserializeOwned};

/// Trait for a topic
pub trait Topic {
    /// Message type of the topic
    type Item: Serialize + DeserializeOwned;

    /// Name of the topic
    fn topic(&self) -> String;
}