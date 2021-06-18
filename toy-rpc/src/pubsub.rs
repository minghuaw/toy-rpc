//! Trait definition for PubSub support

use serde::{Serialize, de::DeserializeOwned};

/// Topic trait must be implemented to use with publisher/subscriber.
/// TODO: A convenience derive macro / proc macro should be available
pub trait Topic {
    /// The type of the message used with this topic
    type Item: Serialize + DeserializeOwned;

    /// Obtains the name of the topic
    fn topic(&self) -> String;
}