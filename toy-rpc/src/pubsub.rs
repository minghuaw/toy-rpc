//! PubSub support
use serde::{de::DeserializeOwned, Serialize};

/// Trait for PubSub Topic
pub trait Topic {
    /// Message type of the topic
    type Item: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Name of the topic
    fn topic() -> String;
}
