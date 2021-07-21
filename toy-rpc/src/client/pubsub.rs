//! PubSub impl on the client side

use flume::r#async::{RecvStream, SendSink};
use flume::{Receiver, Sender};
use futures::{Sink, Stream};
use pin_project::pin_project;
use std::any::TypeId;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::{broker::ClientBrokerItem, Client};
use crate::pubsub::{AckModeAuto, AckModeNone};
use crate::{
    error::Error,
    protocol::{InboundBody, OutboundBody},
    pubsub::Topic,
};

/// Publisher of topic T on the client side
#[pin_project]
pub struct Publisher<T: Topic> {
    #[pin]
    inner: SendSink<'static, ClientBrokerItem>,
    marker: PhantomData<T>,
}

impl<T: Topic> From<Sender<ClientBrokerItem>> for Publisher<T> {
    fn from(inner: Sender<ClientBrokerItem>) -> Self {
        Self {
            inner: inner.into_sink(),
            marker: PhantomData,
        }
    }
}

impl<T: Topic> Sink<T::Item> for Publisher<T> {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_ready(cx).map_err(|err| err.into())
    }

    fn start_send(self: Pin<&mut Self>, item: T::Item) -> Result<(), Self::Error> {
        let this = self.project();
        let topic = T::topic();
        let body = Box::new(item) as Box<OutboundBody>;
        let item = ClientBrokerItem::Publish { topic, body };
        this.inner.start_send(item).map_err(|err| err.into())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx).map_err(|err| err.into())
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_close(cx).map_err(|err| err.into())
    }
}

/// Subscriber of topic T on the client side
#[pin_project]
pub struct Subscriber<T: Topic, AckMode> {
    #[pin]
    inner: RecvStream<'static, Box<InboundBody>>,
    marker: PhantomData<T>,
    ack_mode: PhantomData<AckMode>
}

impl<T: Topic, AckMode> From<Receiver<Box<InboundBody>>> for Subscriber<T, AckMode> {
    fn from(rx: Receiver<Box<InboundBody>>) -> Self {
        Self {
            inner: rx.into_stream(),
            marker: PhantomData,
            ack_mode: PhantomData,
        }
    }
}

impl<T: Topic> Stream for Subscriber<T, AckModeNone> {
    type Item = Result<T::Item, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(val) => match val {
                Some(mut body) => {
                    let result = erased_serde::deserialize(&mut body).map_err(|err| err.into());
                    Poll::Ready(Some(result))
                }
                None => Poll::Ready(None),
            },
        }
    }
}

impl<T: Topic> Stream for Subscriber<T, AckModeAuto> {
    type Item = Result<T::Item, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(val) => match val {
                Some(mut body) => {
                    let result = erased_serde::deserialize(&mut body).map_err(|err| err.into());
                    Poll::Ready(Some(result))
                }
                None => Poll::Ready(None),
            },
        }
    }
}

impl<AckMode> Client<AckMode> {
    /// Creates a new publisher on a topic.
    ///
    /// Multiple local publishers on the same topic are allowed.
    pub fn publisher<T: Topic>(&self) -> Publisher<T> {
        let tx = self.broker.clone();
        Publisher::from(tx)
    }
}

impl Client<AckModeNone> {
    /// Creates a new subscriber on a topic
    ///
    pub fn subscriber<T: Topic + 'static>(&mut self, cap: usize) -> Result<Subscriber<T, AckModeNone>, Error> {
        let (tx, rx) = flume::bounded(cap);
        let topic = T::topic();

        // Check if there is an existing subscriber
        if self.subscriptions.contains_key(&topic) {
            return Err(Error::Internal(
                "Only one local subscriber per topic is allowed".into(),
            ));
        }
        self.subscriptions.insert(topic.clone(), TypeId::of::<T>());

        // Create new subscription
        if let Err(err) = self.broker.send(ClientBrokerItem::Subscribe {
            topic,
            item_sink: tx,
        }) {
            return Err(err.into());
        };

        let sub = Subscriber::from(rx);
        Ok(sub)
    }

    /// Replaces the local subscriber without sending any message to the server
    ///
    /// The previous subscriber will no longer receive any message.
    pub fn replace_local_subscriber<T: Topic + 'static>(
        &mut self,
        cap: usize,
    ) -> Result<Subscriber<T, AckModeNone>, Error> {
        let topic = T::topic();
        match self.subscriptions.get(&topic) {
            Some(entry) => match &TypeId::of::<T>() == entry {
                true => {
                    let (tx, rx) = flume::bounded(cap);
                    if let Err(err) = self.broker.send(ClientBrokerItem::NewLocalSubscriber {
                        topic,
                        new_item_sink: tx,
                    }) {
                        return Err(err.into());
                    }
                    let sub = Subscriber::from(rx);
                    Ok(sub)
                }
                false => Err(Error::Internal("TypeId mismatch".into())),
            },
            None => Err(Error::Internal(
                "There is no existing local subscriber".into(),
            )),
        }
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe<T: Topic + 'static>(&mut self) -> Result<(), Error> {
        let topic = T::topic();
        if let Some(type_id) = self.subscriptions.get(&topic) {
            if type_id == &TypeId::of::<T>() {
                self.subscriptions.remove(&topic);
                self.broker
                    .send_async(ClientBrokerItem::Unsubscribe { topic })
                    .await?;
                return Ok(());
            }
        }
        Err(Error::Internal(
            format!("Not registered to topic: {}", topic).into(),
        ))
    }
}
