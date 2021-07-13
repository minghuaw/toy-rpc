use cfg_if::cfg_if;
use std::{sync::atomic::Ordering, time::Duration};
use flume::Sender;
use futures::{channel::oneshot};

cfg_if!{
    if #[cfg(any(
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
    ))] {
        use std::{sync::Arc, collections::HashMap};
        use brw::{Context, Running};
        use futures::{Sink, SinkExt};
        
        use super::{writer::ClientWriterItem};
    }
}

use crate::{Error, message::{AtomicMessageId, MessageId}, protocol::{InboundBody, OutboundBody}};


#[cfg_attr(all(not(feature = "tokio_runtime"), not(feature = "async_std_runtime")), allow(dead_code))]
pub enum ClientBrokerItem {
    Request{
        // id: MessageId,
        service_method: String,
        duration: Duration,
        body: Box<OutboundBody>, 
        resp_tx: oneshot::Sender<Result<Result<Box<InboundBody>, Box<InboundBody>>, Error>>,
    },
    Response{
        id: MessageId,
        result: Result<Box<InboundBody>, Box<InboundBody>>
    },
    Cancel(MessageId),
    /// New publication to the server
    Publish{
        // id: MessageId,
        topic: String,
        body: Box<OutboundBody>
    },
    Subscribe {
        // id: MessageId,
        topic: String,

        // message is deserialized as it is read on the subscriber
        item_sink: Sender<Box<InboundBody>>, 
    },
    NewLocalSubscriber {
        topic: String,
        new_item_sink: Sender<Box<InboundBody>>,
    },
    Unsubscribe {
        // id: MessageId,
        topic: String,
    },
    /// Subscription from the server
    Subscription {
        id: MessageId,
        topic: String,
        item: Box<InboundBody>,
    },
    /// Stops the broker
    Stop,
}

#[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
use ::tokio::{task::{self}};
#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use ::async_std::task::{self};

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
pub struct ClientBroker {
    pub count: Arc<AtomicMessageId>,
    pub pending: HashMap<MessageId, oneshot::Sender<Result<Result<Box<InboundBody>, Box<InboundBody>>, Error>>>,
    pub next_timeout: Option<Duration>,
    pub subscriptions: HashMap<String, Sender<Box<InboundBody>>>,
}

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
#[async_trait::async_trait]
impl brw::Broker for ClientBroker {
    type Item = ClientBrokerItem;
    type WriterItem = ClientWriterItem;
    type Ok = ();
    type Error = Error;

    async fn op<W>(&mut self, _: &Arc<Context<Self::Item>>, item: Self::Item, mut writer: W) -> Running<Result<Self::Ok, Self::Error>>
    where W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin {
        let res = match item {
            ClientBrokerItem::Request{
                // id,
                service_method,
                duration,
                body,
                resp_tx,
            } => {
                // fetch_add returns the previous value
                let id = self.count.fetch_add(1, Ordering::Relaxed);
                let (tx, rx) = oneshot::channel();
                let fut = async move {
                    // takes care of receiving/cancel  error
                    match rx.await {
                        Ok(res) => res,
                        Err(_) => Err(Error::Canceled(Some(id)))
                    }
                };
                let res = writer.send(
                    ClientWriterItem::Request(id, service_method, duration, body)
                ).await;

                task::spawn(async move {
                    #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                    let res = ::tokio::time::timeout(duration, fut).await;
                    #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                    let res = ::async_std::future::timeout(duration, fut).await;

                    let res = match res {
                        Ok(res) => res,
                        Err(_) => {
                            resp_tx.send(Err(Error::Timeout(Some(id))))
                                .unwrap_or_else(|_| log::error!("Error sending over response sender"));
                            return
                        }
                    };
                    resp_tx.send(res)
                        .unwrap_or_else(|_| log::error!("Error sending over response sender"));
                });

                self.pending.insert(id, tx);
                res.map_err(|err| err.into())
            },
            ClientBrokerItem::Response{id , result} => {
                if let Some(tx) = self.pending.remove(&id) {
                    tx.send(Ok(result)) 
                        .map_err(|_| {
                            Error::Internal("InternalError: client failed to send response over channel".into())
                        })
                } else {
                    Err(Error::Internal("Done channel not found".into()))
                        
                }
            },
            ClientBrokerItem::Publish{topic, body} => {
                let id = self.count.fetch_add(1, Ordering::Relaxed);
                // TODO: QoS check? at least once?
                let res = writer
                    .send(ClientWriterItem::Publish(id, topic, body))
                    .await
                    .map_err(|err| err.into());

                // TODO: Spawn a timed task to check Ack?
                // task::spawn(async move {

                // });                  
                res
            },
            ClientBrokerItem::Subscribe{topic, item_sink} => {
                let id = self.count.fetch_add(1, Ordering::Relaxed);
                // NOTE: Only one local subscriber is allowed
                self.subscriptions.insert(topic.clone(), item_sink);

                let res = writer
                    .send(ClientWriterItem::Subscribe(id, topic))
                    .await
                    .map_err(|err| err.into());
                // TODO: Spawn a timed task to check Ack?
                res
            },
            ClientBrokerItem::NewLocalSubscriber{topic, new_item_sink} => {
                self.subscriptions.insert(topic, new_item_sink);
                Ok(())
            }
            ClientBrokerItem::Unsubscribe{topic } => {
                let id = self.count.fetch_add(1, Ordering::Relaxed);
                // NOTE: the sender should be dropped on the Client side
                let res = writer
                    .send(ClientWriterItem::Unsubscribe(id, topic))
                    .await
                    .map_err(|err| err.into());
                // TODO: Spawn  timed task to check Ack?
                res
            },
            ClientBrokerItem::Subscription {id, topic, item } => {
                log::info!("Received subscription message {{id: {}, topic: {}}}", id, &topic);
                if let Some(sub) = self.subscriptions.get(&topic) {
                    match sub.try_send(item) {
                        Ok(_) => { Ok(()) },
                        Err(err) => {
                            match err {
                                flume::TrySendError::Disconnected(_) => {
                                    self.subscriptions.remove(&topic);
                                    Err(Error::Internal(
                                        "Subscription recver is Disconnected".into()
                                    ))
                                }
                                _ => { Ok(()) }
                            }
                        }
                    }
                } else {
                    Err(Error::Internal(
                        "Topic is not found locally".into()
                    ))
                }
            },
            ClientBrokerItem::Cancel(id) => {
                if let Some(tx) = self.pending.remove(&id) {
                    tx.send(Err(Error::Canceled(Some(id))))
                        .unwrap_or_else(|_| log::error!("Error sending over response sender"));
                }
                writer.send(ClientWriterItem::Cancel(id)).await
                    .map_err(|err| err.into())
            },
            ClientBrokerItem::Stop => {
                println!("Stop");
                if let Err(err) = writer.send(ClientWriterItem::Stop).await {
                    log::error!("{:?}", err);
                }
                return Running::Stop
            }
        };

        Running::Continue(res)
    }
}