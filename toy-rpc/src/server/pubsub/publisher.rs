//! Publisher on the server side

use std::{marker::PhantomData, pin::Pin, sync::{Arc, atomic::Ordering}, task::Poll};

use flume::{Sender, r#async::SendSink};
use futures::Sink;
use pin_project::pin_project;

use crate::{
    error::Error, 
    codec::{Marshal, Reserved, DefaultCodec}, 
    message::AtomicMessageId, 
    pubsub::{AckModeAuto, AckModeNone, Topic}, 
    server::{Server, RESERVED_CLIENT_ID}
};

use super::PubSubItem;

/// Publisher on the server side
#[pin_project]
pub struct Publisher<T: Topic, C: Marshal> {
    #[pin]
    inner: SendSink<'static, PubSubItem>,
    counter: AtomicMessageId,
    marker: PhantomData<T>,
    codec: PhantomData<C>,
}

impl<T: Topic, C: Marshal> From<Sender<PubSubItem>> for Publisher<T, C> {
    fn from(inner: Sender<PubSubItem>) -> Self {
        Self {
            inner: inner.into_sink(),
            counter: AtomicMessageId::new(0),
            marker: PhantomData,
            codec: PhantomData,
        }
    }
}

impl<T: Topic, C: Marshal> Sink<T::Item> for Publisher<T, C> {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_ready(cx).map_err(|err| err.into())
    }

    fn start_send(self: Pin<&mut Self>, item: T::Item) -> Result<(), Self::Error> {
        let this = self.project();
        let topic = T::topic();
        let msg_id = this.counter.fetch_add(1, Ordering::Relaxed);
        let body = C::marshal(&item)?;
        let content = Arc::new(body);
        let item = PubSubItem::Publish {
            client_id: RESERVED_CLIENT_ID,
            msg_id,
            topic,
            content,
        };
        this.inner.start_send(item).map_err(|err| err.into())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx).map_err(|err| err.into())
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_close(cx).map_err(|err| err.into())
    }
}

type PhantomCodec = DefaultCodec<Reserved, Reserved, Reserved>;

macro_rules! impl_server_pubsub_for_ack_modes {
    ($($ack_mode:ty),*) => {
        $(
            impl Server<$ack_mode> {
                /// Creates a new publihser on a topic
                pub fn publisher<T: Topic>(&self) -> Publisher<T, PhantomCodec> {
                    let tx = self.pubsub_tx.clone();
                    Publisher::from(tx)
                }
            }
        )*
    }
}

impl_server_pubsub_for_ack_modes!(AckModeNone, AckModeAuto);