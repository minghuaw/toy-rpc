//! Subscriber on the server side

use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use flume::{r#async::RecvStream, Receiver, Sender};
use futures::Stream;

use crate::{
    codec::{DefaultCodec, Reserved, Unmarshal},
    error::Error,
    pubsub::{AckModeAuto, AckModeNone, Topic},
    server::{broker::ServerBrokerItem, Server, RESERVED_CLIENT_ID},
};

use super::{PubSubItem, PubSubResponder};

/// Subscriber on the client side
#[pin_project::pin_project(PinnedDrop)]
pub struct Subscriber<T: Topic, C: Unmarshal, AckMode> {
    #[pin]
    inner: RecvStream<'static, ServerBrokerItem>,
    pubsub_tx: Sender<PubSubItem>,
    topic: String,
    marker: PhantomData<T>,
    codec: PhantomData<C>,
    ack_mode: PhantomData<AckMode>,
}

impl<T: Topic, C: Unmarshal, AckMode> Subscriber<T, C, AckMode> {
    fn new(inner: Receiver<ServerBrokerItem>, pubsub_tx: Sender<PubSubItem>) -> Self {
        Self {
            inner: inner.into_stream(),
            pubsub_tx,
            topic: T::topic(),
            marker: PhantomData,
            codec: PhantomData,
            ack_mode: PhantomData,
        }
    }
}

#[pin_project::pinned_drop]
impl<T: Topic, C: Unmarshal, AckMode> PinnedDrop for Subscriber<T, C, AckMode> {
    fn drop(self: Pin<&mut Self>) {
        let this = self.get_mut();

        // Unsubscribe if the sender half is not dropped yet
        if !this.inner.is_disconnected() {
            if let Err(_) = this.pubsub_tx.send(PubSubItem::Unsubscribe {
                client_id: RESERVED_CLIENT_ID,
                topic: T::topic(),
            }) {
                log::error!("Failed to send unsubscribe when dropping server side ")
            }
        }
    }
}

impl<T: Topic, C: Unmarshal> Stream for Subscriber<T, C, AckModeNone> {
    type Item = Result<T::Item, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(opt) => match opt {
                Some(item) => match item {
                    ServerBrokerItem::Publication {
                        seq_id: _,
                        topic: _,
                        content,
                    } => {
                        let result = C::unmarshal(&content);
                        Poll::Ready(Some(result.map_err(Into::into)))
                    }
                    _ => {
                        let result = Err(Error::Internal("Invalid PubSub item".into()));
                        Poll::Ready(Some(result))
                    }
                },
                None => Poll::Ready(None),
            },
        }
    }
}

impl<T: Topic, C: Unmarshal> Stream for Subscriber<T, C, AckModeAuto> {
    type Item = Result<T::Item, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(opt) => match opt {
                Some(item) => match item {
                    ServerBrokerItem::Publication {
                        seq_id,
                        topic: _,
                        content,
                    } => {
                        // Send back Ack first
                        log::debug!("Auto Ack");
                        if let Err(err) = this
                            .pubsub_tx
                            .send(PubSubItem::Ack {
                                seq_id,
                                client_id: RESERVED_CLIENT_ID,
                            })
                            .map_err(|err| err.into())
                        {
                            return Poll::Ready(Some(Err(err)));
                        }
                        let result = C::unmarshal(&content);
                        Poll::Ready(Some(result.map_err(Into::into)))
                    }
                    _ => {
                        let result = Err(Error::Internal("Invalid PubSub item".into()));
                        Poll::Ready(Some(result))
                    }
                },
                None => Poll::Ready(None),
            },
        }
    }
}

type PhantomCodec = DefaultCodec<Reserved, Reserved, Reserved>;

macro_rules! impl_server_pubsub_for_ack_modes {
    ($($ack_mode:ty),*) => {
        $(
            impl Server<$ack_mode> {
                /// Creates a new subscriber on a topic
                ///
                /// Only one subscriber per topic can exist at the same time on the server.
                /// Creating a new subscriber will drop the sender of the old subscriber.
                #[cfg(not(feature = "http_actix_web"))]
                #[cfg_attr(feature = "docs", doc(cfg(not(feature = "http_actix_web"))))]
                pub fn subscriber<T: Topic>(&self, cap: usize) -> Result<Subscriber<T, PhantomCodec, $ack_mode>, Error> {
                    let (sender, rx) = flume::bounded(cap);
                    let client_id = RESERVED_CLIENT_ID;
                    let topic = T::topic();
                    let sender = PubSubResponder::Sender(sender);
                    self.pubsub_tx.send(PubSubItem::Subscribe{client_id, topic, sender})?;
                    Ok(Subscriber::new(rx, self.pubsub_tx.clone()))
                }
            }
        )*
    }
}

impl_server_pubsub_for_ack_modes!(AckModeNone, AckModeAuto);
