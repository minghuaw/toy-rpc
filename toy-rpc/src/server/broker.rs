/// Broker on the server side

use std::sync::Arc;
use std::future::Future;
use std::time::Duration;

use crate::protocol::InboundBody;
use crate::service::{ArcAsyncServiceCall, HandlerResult};

use crate::{
    message::{MessageId},
    error::Error,
};

cfg_if::cfg_if! {
    if #[cfg(not(feature = "http_actix_web"))] {
        use std::collections::HashMap;
        
        use flume::Sender;
        use brw::{Running, Broker};
        use futures::sink::{Sink, SinkExt};

        use super::ClientId;
        use super::pubsub::PubSubItem;
        use super::writer::ServerWriterItem;
    }
}

#[cfg(any(
    feature = "docs",
    all(feature = "tokio_runtime", not(feature = "async_std_runtime"), not(feature = "http_actix_web"))
))]
use ::tokio::task::JoinHandle;
#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use ::async_std::task::JoinHandle;

#[cfg(not(feature = "http_actix_web"))]
pub(crate) struct ServerBroker { 
    pub client_id: ClientId,
    pub executions: HashMap<MessageId, JoinHandle<()>>,
    pub pubsub_broker: Sender<PubSubItem>
}

#[cfg(not(feature = "http_actix_web"))]
impl ServerBroker {
    pub fn new(client_id: ClientId, pubsub_broker: Sender<PubSubItem>) -> Self {
        Self {
            client_id,
            executions: HashMap::new(),
            pubsub_broker,
        }
    }
}

#[cfg_attr(feature = "http_actix_web", derive(actix::Message))]
#[cfg_attr(feature = "http_actix_web", rtype(result = "()"))]
pub(crate) enum ServerBrokerItem {
    Request {
        call: ArcAsyncServiceCall,
        id: MessageId,
        method: String,
        duration: Duration,
        deserializer: Box<InboundBody>,
    },
    Response {
        id: MessageId,
        result: HandlerResult
    },
    Cancel(MessageId),
    // A new publish from the client publisher
    Publish {
        id: MessageId,
        topic: String,
        content: Vec<u8>,
    },
    // A new subscribe from the client subscriber
    Subscribe {
        id: MessageId,
        topic: String,
    },
    Unsubscribe {
        id: MessageId,
        topic: String,
    },
    // A publication message to the client subscriber
    Publication {
        id: MessageId,
        topic: String,
        content: Arc<Vec<u8>>, 
    },
    Stop
}

#[cfg(not(feature = "http_actix_web"))]
#[async_trait::async_trait]
impl Broker for ServerBroker {
    type Item = ServerBrokerItem;
    type WriterItem = ServerWriterItem;
    type Ok = ();
    type Error = Error;

    async fn op<W>(&mut self, ctx: & Arc<brw::Context<Self::Item>>, item: Self::Item, mut writer: W) -> Running<Result<Self::Ok, Self::Error>>
    where W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin {
        match item {
            ServerBrokerItem::Request{
                call,
                id,
                method,
                duration,
                deserializer
            } => {
                let fut = call(method, deserializer);
                let _broker = ctx.broker.clone();
                let handle = handle_request(_broker, duration, id, fut);
                self.executions.insert(id, handle);
                Running::Continue(Ok(()))
            },
            ServerBrokerItem::Response{id, result} => {
                self.executions.remove(&id);
                let msg = ServerWriterItem::Response{id, result};
                let res: Result<(), Error> = writer.send(msg).await
                    .map_err(|err| err.into());
                Running::Continue(res)
            },
            ServerBrokerItem::Cancel(id) => {
                if let Some(handle) = self.executions.remove(&id) {
                    #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                    handle.abort();
                    #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                    handle.cancel().await;
                }

                Running::Continue(Ok(()))
            },
            ServerBrokerItem::Publish {id, topic, content} => {
                // Publish is the PubSub message from client to server
                let content = Arc::new(content);
                let msg = PubSubItem::Publish{
                    msg_id: id,
                    topic,
                    content
                };
                Running::Continue(
                    self.pubsub_broker.send_async(msg).await
                        .map_err(|err| err.into())
                )
            },
            ServerBrokerItem::Subscribe {id, topic} => {
                log::debug!("Message ID: {}, Subscribe to topic: {}", &id, &topic);
                let msg = PubSubItem::Subscribe {
                    client_id: self.client_id, 
                    topic, 
                    sender: ctx.broker.clone()
                };
                Running::Continue(
                    self.pubsub_broker.send_async(msg).await
                        .map_err(|err| err.into())
                )
            },
            ServerBrokerItem::Unsubscribe {id, topic} => {
                log::debug!("Message ID: {}, Unsubscribe from topic: {}", &id, &topic);
                let msg = PubSubItem::Unsubscribe{client_id: self.client_id, topic};
                Running::Continue(
                    self.pubsub_broker.send_async(msg).await
                        .map_err(|err| err.into())
                )
            },
            ServerBrokerItem::Publication {id, topic, content} => {
                // Publication is the PubSub message from server to client
                let msg = ServerWriterItem::Publication{id, topic, content};
                Running::Continue(
                    writer.send(msg).await
                        .map_err(|err| err.into())
                )
            },
            ServerBrokerItem::Stop => {
                for (_, handle) in self.executions.drain() {
                    log::debug!("Stopping execution as client is disconnected");
                    #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                    handle.abort();
                    #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                    handle.cancel().await;
                }
                log::debug!("Client connection is closed");
                Running::Stop
            }
        }
    }
}

/// Spawn the execution in a async_std task and return the JoinHandle
#[cfg(all(
    feature = "async_std_runtime",
    not(feature = "tokio_runtime")
))]
fn handle_request(
    broker: Sender<ServerBrokerItem>,
    duration: Duration,
    id: MessageId,
    fut: impl Future<Output=HandlerResult> + Send + 'static,
) -> ::async_std::task::JoinHandle<()> {
    ::async_std::task::spawn(async move {
        let result = execute_timed_call(id, duration, fut).await;
        broker.send_async(ServerBrokerItem::Response{id, result}).await
            .unwrap_or_else(|e| log::error!("{}", e));
    })
}

/// Spawn the execution in a tokio task and return the JoinHandle
#[cfg(all(
    feature = "tokio_runtime",
    not(feature = "async_std_runtime"),
    not(feature = "http_actix_web")
))]
fn handle_request(
    broker: Sender<ServerBrokerItem>,
    duration: Duration,
    id: MessageId,
    fut: impl Future<Output=HandlerResult> + Send + 'static,
) -> ::tokio::task::JoinHandle<()> {
    ::tokio::task::spawn(async move {
        let result = execute_timed_call(id, duration, fut).await;
        broker.send_async(ServerBrokerItem::Response{id, result}).await
            .unwrap_or_else(|e| log::error!("{}", e));
    })
}

pub(crate) async fn execute_call(
    id: MessageId,
    fut: impl Future<Output = HandlerResult>,
) -> HandlerResult {
    let result: HandlerResult = fut.await.map_err(|err| {
        log::error!(
            "Error found executing request id: {}, error msg: {}",
            &id,
            &err
        );
        match err {
            // if serde cannot parse request, the argument is likely mistaken
            Error::ParseError(e) => {
                log::error!("ParseError {:?}", e);
                Error::InvalidArgument
            }
            e => e,
        }
    });
    result
}

#[cfg(not(feature = "http_actix_web"))]
pub(crate) async fn execute_timed_call(
    id: MessageId,
    duration: Duration,
    fut: impl Future<Output = HandlerResult>
) -> HandlerResult {
    #[cfg(all(
        feature = "async_std_runtime",
        not(feature = "tokio_runtime")
    ))]
    match ::async_std::future::timeout(
        duration,
        execute_call(id, fut)
    ).await {
        Ok(res) => res,
        Err(_) => Err(Error::Timeout(Some(id)))
    }

    #[cfg(all(
        feature = "tokio_runtime",
        not(feature = "async_std_runtime"),
    ))]
    match ::tokio::time::timeout(
        duration,
        execute_call(id, fut)
    ).await {
        Ok(res) => res,
        Err(_) => Err(Error::Timeout(Some(id)))
    }
}