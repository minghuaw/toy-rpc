//! Broker on the server side

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use crate::protocol::InboundBody;
use crate::pubsub::SeqId;
use crate::service::{ArcAsyncServiceCall, ServiceCallResult};

use crate::{error::Error, message::MessageId};

cfg_if::cfg_if! {
    if #[cfg(not(feature = "http_actix_web"))] {
        use std::collections::HashMap;
        use async_trait::async_trait;
        use flume::Sender;
        
        use crate::util::Broker;
        use crate::server::pubsub::PubSubResponder;

        use super::ClientId;
        use super::pubsub::PubSubItem;
        use super::writer::ServerWriterItem;
    }
}

#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use ::async_std::task::JoinHandle;
#[cfg(any(
    feature = "docs",
    all(
        feature = "tokio_runtime",
        not(feature = "async_std_runtime"),
        not(feature = "http_actix_web")
    )
))]
use ::tokio::task::JoinHandle;

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
        result: ServiceCallResult,
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
        seq_id: SeqId,
        topic: String,
        content: Arc<Vec<u8>>,
    },
    // The server broker should only receive Ack from the client
    InboundAck {
        seq_id: SeqId,
    },
    Stopping,
    Stop,
}

#[cfg(not(feature = "http_actix_web"))]
pub(crate) enum BrokerState {
    Started,
    Ending,
    Ended
}

#[cfg(not(feature = "http_actix_web"))]
pub(crate) struct ServerBroker {
    pub client_id: ClientId,
    pub executions: HashMap<MessageId, JoinHandle<()>>,
    pub pubsub_broker: Sender<PubSubItem>,
    pub state: BrokerState,
}

#[cfg(not(feature = "http_actix_web"))]
impl ServerBroker {
    pub fn new(client_id: ClientId, pubsub_broker: Sender<PubSubItem>) -> Self {
        Self {
            client_id,
            executions: HashMap::new(),
            pubsub_broker,
            state: BrokerState::Started,
        }
    }

    fn handle_request<'a>(
        &'a mut self,
        tx: &Sender<ServerBrokerItem>,
        call: ArcAsyncServiceCall,
        id: MessageId,
        method: String,
        duration: Duration,
        deserializer: Box<InboundBody>,
    ) -> Result<(), Error> {
        let fut = call(method, deserializer);
        let handle = spawn_timed_request_execution(tx, duration, id, fut);
        self.executions.insert(id, handle);
        Ok(())
    }

    async fn handle_response(
        &mut self,
        id: MessageId,
        result: ServiceCallResult,
    ) -> ServerWriterItem {
        self.executions.remove(&id);
        ServerWriterItem::Response { id, result }
    }

    async fn handle_cancel(&mut self, id: MessageId) -> Result<(), Error> {
        if let Some(handle) = self.executions.remove(&id) {
            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
            handle.abort();
            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
            handle.cancel().await;
        }
        Ok(())
    }

    async fn handle_publish_inner(
        &mut self,
        id: MessageId,
        topic: String,
        content: Vec<u8>,
    ) -> Result<(), Error> {
        let content = Arc::new(content);
        let msg = PubSubItem::Publish {
            client_id: self.client_id,
            msg_id: id,
            topic,
            content,
        };
        self.pubsub_broker
            .send_async(msg)
            .await
            .map_err(|err| err.into())
    }

    async fn handle_subscribe<'a>(
        &'a mut self,
        tx: &Sender<ServerBrokerItem>,
        id: MessageId,
        topic: String,
    ) -> Result<(), Error> {
        log::debug!("Message ID: {}, Subscribe to topic: {}", &id, &topic);
        let sender = PubSubResponder::Sender(tx.clone());
        let msg = PubSubItem::Subscribe {
            client_id: self.client_id,
            topic,
            sender,
        };

        self.pubsub_broker
            .send_async(msg)
            .await
            .map_err(|err| err.into())
    }

    async fn handle_unsubscribe(&mut self, id: MessageId, topic: String) -> Result<(), Error> {
        log::debug!("Message ID: {}, Unsubscribe from topic: {}", &id, &topic);
        let msg = PubSubItem::Unsubscribe {
            client_id: self.client_id,
            topic,
        };

        self.pubsub_broker
            .send_async(msg)
            .await
            .map_err(|err| err.into())
    }

    async fn handle_publication(
        &mut self,
        seq_id: SeqId,
        topic: String,
        content: Arc<Vec<u8>>,
    ) -> ServerWriterItem {
        // Publication is the PubSub message from server to client
        ServerWriterItem::Publication {
            seq_id,
            topic,
            content,
        }
    }

    async fn handle_inbound_ack(&mut self, seq_id: SeqId) -> Result<(), Error> {
        let item = PubSubItem::Ack {
            seq_id,
            client_id: self.client_id,
        };
        self.pubsub_broker
            .send_async(item)
            .await
            .map_err(|err| err.into())
    }
}

#[cfg(not(feature = "http_actix_web"))]
impl ServerBroker {
    // Publish is the PubSub message from client to server
    async fn handle_publish(
        &mut self,
        id: MessageId,
        topic: String,
        content: Vec<u8>,
    ) -> Result<(), Error> {
        self.handle_publish_inner(id, topic, content).await
    }
}

#[cfg(not(feature = "http_actix_web"))]
#[async_trait]
impl Broker for ServerBroker {
    type Item = ServerBrokerItem;
    type WriterItem = ServerWriterItem;

    async fn op(
        &mut self,
        item: ServerBrokerItem,
        tx: &Sender<ServerBrokerItem>,
    ) -> Result<Option<ServerWriterItem>, Error> {
        match item {
            ServerBrokerItem::Request {
                call,
                id,
                method,
                duration,
                deserializer,
            } => self.handle_request(tx, call, id, method, duration, deserializer).map(|_| None),
            ServerBrokerItem::Response { id, result } => {
                Ok(Some(self.handle_response(id, result).await))
            }
            ServerBrokerItem::Cancel(id) => self.handle_cancel(id).await.map(|_| None),
            ServerBrokerItem::Publish { id, topic, content } => {
                self.handle_publish(id, topic, content).await.map(|_| None)
            }
            ServerBrokerItem::Subscribe { id, topic } => {
                self.handle_subscribe(tx, id, topic).await.map(|_| None)
            }
            ServerBrokerItem::Unsubscribe { id, topic } => self.handle_unsubscribe(id, topic).await.map(|_| None),
            ServerBrokerItem::Publication {
                seq_id,
                topic,
                content,
            } => {
                let item = self.handle_publication(seq_id, topic, content)
                    .await;
                Ok(Some(item))
            }
            ServerBrokerItem::InboundAck { seq_id } => self.handle_inbound_ack(seq_id).await.map(|_| None),
            ServerBrokerItem::Stopping => {
                for (_, handle) in self.executions.drain() {
                    log::debug!("Stopping execution as client is disconnected");
                    #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                    handle.abort();
                    #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                    handle.cancel().await;
                }

                self.state = BrokerState::Ending;

                tx
                    .send_async(ServerBrokerItem::Stop)
                    .await?;

                Ok(Some(ServerWriterItem::Stopping))

            }
            ServerBrokerItem::Stop => {
                self.state = BrokerState::Ended;
                Ok(Some(ServerWriterItem::Stop))
            }
        }
    }
}

/// Spawn the execution in a async_std task and return the JoinHandle
#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
fn spawn_timed_request_execution(
    tx: &Sender<ServerBrokerItem>,
    duration: Duration,
    id: MessageId,
    fut: impl Future<Output = ServiceCallResult> + Send + 'static,
) -> ::async_std::task::JoinHandle<()> {
    let tx = tx.clone();
    ::async_std::task::spawn(async move {
        let result = execute_timed_call(id, duration, fut).await;
        tx
            .send_async(ServerBrokerItem::Response { id, result })
            .await
            .unwrap_or_else(|e| log::error!("{}", e));
    })
}

/// Spawn the execution in a tokio task and return the JoinHandle
#[cfg(all(
    feature = "tokio_runtime",
    not(feature = "async_std_runtime"),
    not(feature = "http_actix_web")
))]
fn spawn_timed_request_execution(
    tx: &Sender<ServerBrokerItem>,
    duration: Duration,
    id: MessageId,
    fut: impl Future<Output = ServiceCallResult> + Send + 'static,
) -> ::tokio::task::JoinHandle<()> {
    let tx = tx.clone();
    ::tokio::task::spawn(async move {
        let result = execute_timed_call(id, duration, fut).await;
        tx
            .send_async(ServerBrokerItem::Response { id, result })
            .await
            .unwrap_or_else(|e| log::error!("{}", e));
    })
}

pub(crate) async fn execute_call(
    id: MessageId,
    fut: impl Future<Output = ServiceCallResult>,
) -> ServiceCallResult {
    let result: ServiceCallResult = fut.await.map_err(|err| {
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
    fut: impl Future<Output = ServiceCallResult>,
) -> ServiceCallResult {
    #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
    match ::async_std::future::timeout(duration, execute_call(id, fut)).await {
        Ok(res) => res,
        Err(err) => {
            log::error!("Request {} reached timeout (err: {})", id, err);
            Err(Error::Timeout(id))
        }
    }

    #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime"),))]
    match ::tokio::time::timeout(duration, execute_call(id, fut)).await {
        Ok(res) => res,
        Err(err) => {
            log::error!("Request {} reached timeout (err: {})", id, err);
            Err(Error::Timeout(id))
        }
    }
}
