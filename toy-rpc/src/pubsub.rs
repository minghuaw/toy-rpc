//! PubSub support
use flume::r#async::{SendSink,RecvStream};
use std::marker::PhantomData;
use pin_project::pin_project;
use serde::{Serialize, de::DeserializeOwned};

/// Trait for PubSub Topic
pub trait Topic {
    /// Message type of the topic
    type Item: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Name of the topic
    fn topic() -> String;
}

/// Publisher of topic T
#[pin_project]
pub struct Publisher<T, I> 
where 
    T: Topic,
    I: 'static
{
    #[pin]
    pub(crate) inner: SendSink<'static, I>,
    pub(crate) marker: PhantomData<T>
}

/// Subscriber of topic T
#[pin_project]
pub struct Subscriber<T, I> 
where 
    T: Topic,
    I: 'static,
{
    #[pin]
    pub(crate) inner: RecvStream<'static, I>,
    pub(crate) marker: PhantomData<T>,
}
