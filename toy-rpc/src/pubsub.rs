//! PubSub support
use flume::{Receiver};
use futures::{Stream};
use flume::r#async::{SendSink,RecvStream};
use std::marker::PhantomData;
use std::task::Poll;
use pin_project::pin_project;
use serde::{Serialize, de::DeserializeOwned};

use crate::protocol::{InboundBody};
use crate::error::Error;

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
pub struct Subscriber<T> 
where 
    T: Topic,
{
    #[pin]
    inner: RecvStream<'static, Box<InboundBody>>,
    marker: PhantomData<T>,
}

impl<T:Topic> Subscriber<T> {
    pub(crate) fn from_recver(rx: Receiver<Box<InboundBody>>) -> Self {
        Self {
            inner: rx.into_stream(),
            marker: PhantomData
        }
    }
}

impl<T: Topic> Stream for Subscriber<T> {
    type Item = Result<T::Item, Error>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(val) => {
                match val {
                    Some(mut body) => {
                        let result = erased_serde::deserialize(&mut body)
                            .map_err(|err| err.into());
                        Poll::Ready(Some(result))
                    },
                    None => Poll::Ready(None)
                }
            }
        }
    }
}