//! PubSub support

use flume::{Receiver, Sender};
use serde::{Serialize, de::DeserializeOwned};
use futures::{Stream, Sink};
use flume::r#async::{SendSink,RecvStream};
use std::marker::PhantomData;
use std::task::Poll;
use std::pin::Pin;
use pin_project::pin_project;

use crate::protocol::{InboundBody, OutboundBody};
use crate::error::Error;

/// Trait for a topic
pub trait Topic {
    /// Message type of the topic
    type Item: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Name of the topic
    fn topic() -> String;
}

#[pin_project]
pub struct Publisher<T> 
where 
    T: Topic,
{
    #[pin]
    inner: SendSink<'static, Box<OutboundBody>>,
    marker: PhantomData<T>
}

impl<T: Topic> Publisher<T> {
    pub fn from_sender(tx: Sender<Box<OutboundBody>>) -> Self {
        Self {
            inner: tx.into_sink(),
            marker: PhantomData
        }
    }
}

impl<T> Sink<T::Item> for Publisher<T> 
where 
    T: Topic,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_ready(cx)
            .map_err(|err| err.into())
    }

    fn start_send(self: Pin<&mut Self>, item: T::Item) -> Result<(), Self::Error> {
        let outbound_body = Box::new(item) as Box<OutboundBody>;
        let this = self.project();
        this.inner.start_send(outbound_body)
            .map_err(|err| err.into())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx)
            .map_err(|err| err.into())
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_close(cx)
            .map_err(|err| err.into())
    }
}

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
    pub fn from_recver(rx: Receiver<Box<InboundBody>>) -> Self {
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