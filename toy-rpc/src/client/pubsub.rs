//! PubSub integration for client
use std::any::TypeId;
use std::pin::Pin;
use std::task::Poll;
use futures::{Sink, Stream};

use crate::{error::Error, protocol::{InboundBody, OutboundBody}, pubsub::{Topic, Publisher, Subscriber}};
use super::{Client, broker::ClientBrokerItem};

impl<T> Sink<T::Item> for Publisher<T, ClientBrokerItem> 
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
        let this = self.project();
        let topic = T::topic();
        let body = Box::new(item) as Box<OutboundBody>;
        let item = ClientBrokerItem::Publish{topic, body};
        this.inner.start_send(item)
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

impl<T: Topic> Stream for Subscriber<T, Box<InboundBody>> {
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

impl Client {
    /// Creates a new publisher on a topic. 
    ///
    /// Multiple local publishers on the same topic are allowed. 
    pub fn publisher<T>(&self) -> Publisher<T, ClientBrokerItem> 
    where 
        T: Topic,
    {
        let tx = self.broker.clone();
        Publisher::from(tx)
    }
    
    /// Creates a new subscriber on a topic
    ///
    pub fn subscriber<T: Topic + 'static>(&mut self, cap: usize) -> Result<Subscriber<T, Box<InboundBody>>, Error> {
        let (tx, rx) = flume::bounded(cap);
        let topic = T::topic();
    
        // Check if there is an existing subscriber
        if self.subscriptions.contains_key(&topic) {
            return Err(Error::Internal("Only one local subscriber per topic is allowed".into()))
        }
        self.subscriptions.insert(topic.clone(), TypeId::of::<T>());
        
        // Create new subscription
        if let Err(err) = self.broker.send(ClientBrokerItem::Subscribe{topic, item_sink: tx}) {
            return Err(err.into())
        };
    
        let sub = Subscriber::from(rx);
        Ok(sub)
    }
    
    /// Replaces the local subscriber without sending any message to the server
    ///
    /// The previous subscriber will no longer receive any message.
    pub fn replace_local_subscriber<T: Topic + 'static>(&mut self, cap: usize) -> Result<Subscriber<T, Box<InboundBody>>, Error> {
        let topic = T::topic();
        match self.subscriptions.get(&topic) {
            Some(entry) => {
                match &TypeId::of::<T>() == entry {
                    true => {
                        let (tx, rx) = flume::bounded(cap);
                        if let Err(err) = self.broker.send(ClientBrokerItem::NewLocalSubscriber{topic, new_item_sink: tx}) {
                            return Err(err.into())
                        }
                        let sub = Subscriber::from(rx);
                        Ok(sub)
                    },
                    false => Err(Error::Internal("TypeId mismatch".into()))
                }
            },
            None => Err(Error::Internal("There is no existing local subscriber".into()))
        }
    }
}
