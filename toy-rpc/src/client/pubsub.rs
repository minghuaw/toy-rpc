//! PubSub integration for client
use std::any::TypeId;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::Poll;
use futures::{Sink};
use flume::{Sender};

use crate::{
    error::Error,
    pubsub::{Topic, Publisher, Subscriber},
    protocol::OutboundBody,
};
use super::{Client, broker::ClientBrokerItem};


impl<T> From<Sender<ClientBrokerItem>> for Publisher<T, ClientBrokerItem> 
where 
    T: Topic
{
    fn from(inner: Sender<ClientBrokerItem>) -> Self {
        Self {
            inner: inner.into_sink(),
            marker: PhantomData
        }
    }
}

impl<T> Sink<T::Item> for Publisher<T, ClientBrokerItem> 
where 
    T: Topic,
    // S: Sink<ClientBrokerItem, Error = Error>,
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
    pub fn subscriber<T: Topic + 'static>(&mut self, cap: usize) -> Result<Subscriber<T>, Error> {
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
    
        let sub = Subscriber::from_recver(rx);
        Ok(sub)
    }
    
    /// Replaces the local subscriber without sending any message to the server
    ///
    /// The previous subscriber will no longer receive any message.
    pub fn replace_local_subscriber<T: Topic + 'static>(&mut self, cap: usize) -> Result<Subscriber<T>, Error> {
        let topic = T::topic();
        match self.subscriptions.get(&topic) {
            Some(entry) => {
                match &TypeId::of::<T>() == entry {
                    true => {
                        let (tx, rx) = flume::bounded(cap);
                        if let Err(err) = self.broker.send(ClientBrokerItem::NewLocalSubscriber{topic, new_item_sink: tx}) {
                            return Err(err.into())
                        }
                        let sub = Subscriber::from_recver(rx);
                        Ok(sub)
                    },
                    false => Err(Error::Internal("TypeId mismatch".into()))
                }
            },
            None => Err(Error::Internal("There is no existing local subscriber".into()))
        }
    }
}
