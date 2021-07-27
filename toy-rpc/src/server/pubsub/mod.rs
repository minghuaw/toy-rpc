//! PubSub impl on the server side

use flume::{Receiver, Sender};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::marker::PhantomData;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "http_actix_web")]
use actix::Recipient;

use crate::message::{AtomicMessageId, MessageId};
use crate::pubsub::{AckModeAuto, AckModeNone, SeqId};

use super::{broker::ServerBrokerItem, ClientId};

#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use async_std::task;
#[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
use tokio::task;

pub(crate) enum PubSubResponder {
    #[cfg(not(feature = "http_actix_web"))]
    Sender(Sender<ServerBrokerItem>),
    #[cfg(feature = "http_actix_web")]
    Recipient(Recipient<ServerBrokerItem>),
}

pub(crate) enum PubSubItem {
    /// A new publish request from publisher
    Publish {
        client_id: ClientId,
        msg_id: MessageId,
        topic: String,
        content: Arc<Vec<u8>>,
    },
    PublishRetry {
        client_ids: BTreeSet<ClientId>,
        seq_id: SeqId,
        topic: String,
        content: Arc<Vec<u8>>,
    },
    RemovePendingAcks {
        seq_id: SeqId,
    },
    Subscribe {
        client_id: ClientId,
        topic: String,
        sender: PubSubResponder,
    },
    Unsubscribe {
        client_id: ClientId,
        topic: String,
    },
    Ack {
        seq_id: SeqId,
        client_id: ClientId,
    },
    Stop,
}

pub(crate) struct PubSubBroker<AckMode> {
    seq_counter: AtomicMessageId,
    pubsub_tx: Sender<PubSubItem>,
    listener: Receiver<PubSubItem>,
    subscriptions: HashMap<String, BTreeMap<ClientId, PubSubResponder>>,
    pending_acks: BTreeMap<SeqId, Sender<ClientId>>,
    pub_retry_timeout: Duration,
    ack_mode: PhantomData<AckMode>
}

impl<AckMode: Send + 'static> PubSubBroker<AckMode> {
    pub fn new(retry_timeout: Duration) -> (Self, Sender<PubSubItem>) {
        let (pubsub_tx, listener) = flume::unbounded();
        (
            Self {
                seq_counter: AtomicMessageId::new(0),
                pubsub_tx: pubsub_tx.clone(),
                listener,
                subscriptions: HashMap::new(),
                pending_acks: BTreeMap::new(),
                pub_retry_timeout: retry_timeout,
                ack_mode: PhantomData
            },
            pubsub_tx
        )
    }

    fn get_seq_id(&mut self, client_id: &ClientId, msg_id: &MessageId) -> SeqId {
        let seq_id = self.seq_counter.fetch_add(1, Ordering::Relaxed);
        let seq_id = SeqId::new(seq_id);
        log::info!("{:?} assigned to Publish message {} from client {}", &seq_id, msg_id, client_id);
        seq_id
    }

    pub fn handle_publish_inner(
        &mut self, 
        seq_id: SeqId, 
        topic: &String,
        content: Arc<Vec<u8>>
    ) {
        if let Some(entry) = self.subscriptions.get_mut(topic) {
            entry.retain(|_, responder| {
                let msg = ServerBrokerItem::Publication{
                    seq_id: seq_id.clone(),
                    topic: topic.clone(),
                    content: content.clone()
                };

                send_broker_item_publication(responder, msg)
            })
        }
    }

    

    async fn handle_publish_retry(
        &mut self,
        client_ids: BTreeSet<ClientId>,
        seq_id: SeqId,
        topic: String,
        content: Arc<Vec<u8>>,
    ) {
        log::debug!("Retry publish");
        if let Some(entry) = self.subscriptions.get_mut(&topic) {
            
            for client_id in client_ids {
                let msg = ServerBrokerItem::Publication {
                    seq_id: seq_id.clone(),
                    topic: topic.clone(),
                    content: content.clone()
                };
                if let Some(responder) = entry.get_mut(&client_id) {
                    send_broker_item_publication(responder, msg);
                }
            }
        }
    }

    pub fn handle_subscribe(
        &mut self,
        client_id: ClientId,
        topic: String,
        sender: PubSubResponder
    ) {
        match self.subscriptions.get_mut(&topic) {
            Some(entry) => {
                entry.insert(client_id, sender);
            }
            None => {
                let mut entry = BTreeMap::new();
                entry.insert(client_id, sender);
                self.subscriptions.insert(topic, entry);
            }
        }
    }

    pub fn handle_unsubscribe(&mut self, client_id: ClientId, topic: String) {
        if let Some(entry) = self.subscriptions.get_mut(&topic) {
            entry.remove(&client_id);
        }
    }

    pub fn handle_ack(&mut self, seq_id: SeqId, client_id: ClientId) {
        log::debug!("Received Ack for seq_id: {:?} from client {:?}", &seq_id, &client_id);

    }   
}

impl PubSubBroker<AckModeNone> {
    pub fn handle_publish(
        &mut self,
        client_id: ClientId,
        msg_id: MessageId,
        topic: String,
        content: Arc<Vec<u8>>
    ) {
        let seq_id = self.get_seq_id(&client_id, &msg_id);
        self.handle_publish_inner(seq_id, &topic, content)
    }
}

impl PubSubBroker<AckModeAuto> {
    fn spawn_timed_task_waiting_for_acks(
        &mut self,
        topic: String,
        seq_id: SeqId,
        content: Arc<Vec<u8>>
    ) {
        #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
        use tokio::time::timeout;        
        #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
        use async_std::future::timeout;

        let (tx, rx) = flume::unbounded::<ClientId>();
        let duration = self.pub_retry_timeout;
        self.pending_acks.insert(seq_id.clone(), tx);
        if let Some(entry) = self.subscriptions.get(&topic) {
            let set: BTreeSet<ClientId> = entry.keys().map(|val| *val).collect();
            let len = set.len();
            let pubsub_tx = self.pubsub_tx.clone();
            
            task::spawn(async move {
                let mut set = set;
                let fut = async {
                    let _set = &mut set;
                    for _ in 0..len {
                        if let Ok(client_id) = rx.recv_async().await {
                            if _set.remove(&client_id) == false {
                                log::error!("Client ID {} is not found in subscription ack set", client_id);
                            }
                        }
                    }
                };

                let msg = match timeout(duration, fut).await {
                    Ok(_) => PubSubItem::RemovePendingAcks { seq_id },
                    Err(err) => {
                        log::error!("{:?}", err);
                        PubSubItem::PublishRetry {
                            client_ids: set, 
                            seq_id, 
                            topic: topic, 
                            content
                        }
                    }
                };
                pubsub_tx.send_async(msg)
                        .await
                        .unwrap_or_else(|err| log::error!("{:?}", err))
            });
        }
    }

    pub fn handle_publish(
        &mut self,
        client_id: ClientId,
        msg_id: MessageId,
        topic: String,
        content: Arc<Vec<u8>>
    ) {
        let seq_id = self.get_seq_id(&client_id, &msg_id);
        self.handle_publish_inner(seq_id.clone(), &topic, content.clone());
        self.spawn_timed_task_waiting_for_acks(topic, seq_id, content)
    }
}

macro_rules! impl_pubsub_broker_for_ack_modes {
    ($($ack_mode:ty),*) => {
        $(
            impl PubSubBroker<$ack_mode> {
                /// Spawn PubSubBroker loop in a task
                #[cfg(any(
                    all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
                    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
                ))]
                pub fn spawn(self) {
                    #[cfg(not(feature = "http_actix_web"))]
                    task::spawn(self.pubsub_loop());
                    #[cfg(all(feature = "http_actix_web"))]
                    actix::spawn(self.pubsub_loop());
                }

                pub async fn pubsub_loop(mut self) {
                    while let Ok(item) = self.listener.recv_async().await {
                        match item {
                            PubSubItem::Publish {
                                client_id,
                                msg_id,
                                topic,
                                content,
                            } => {
                                self.handle_publish(client_id, msg_id, topic, content)
                            },
                            PubSubItem::PublishRetry {
                                client_ids,
                                seq_id,
                                topic,
                                content
                            } => {
                                self.handle_publish_retry(client_ids, seq_id, topic, content).await
                            },
                            PubSubItem::RemovePendingAcks{ seq_id } => {
                                self.pending_acks.remove(&seq_id);
                            },
                            PubSubItem::Subscribe {
                                client_id,
                                topic,
                                sender,
                            } => {
                                self.handle_subscribe(client_id, topic, sender)
                            },
                            PubSubItem::Unsubscribe { client_id, topic } => {
                                self.handle_unsubscribe(client_id, topic)
                            },
                            PubSubItem::Ack{seq_id, client_id} => {
                                self.handle_ack(seq_id, client_id)
                            },
                            PubSubItem::Stop => return,
                        }
                    }
                }
            }
        )*
    };
}

impl_pubsub_broker_for_ack_modes!(AckModeNone, AckModeAuto);

fn send_broker_item_publication(
    responder: &mut PubSubResponder,
    msg: ServerBrokerItem
) -> bool {
    match responder {
        #[cfg(not(feature = "http_actix_web"))]
        PubSubResponder::Sender(tx) => {
            if let Err(err) = tx.try_send(msg) {
                if let flume::TrySendError::Disconnected(_) = err {
                    log::error!("Client is disconnected, removing from subscriptions");
                    return false
                }
            }
        },
        #[cfg(feature = "http_actix_web")]
        PubSubResponder::Recipient(tx) => {
            if let Err(err) = tx.try_send(msg) {
                if let actix::prelude::SendError::Closed(_) = err {
                    log::error!("Client is disconnected, removing from subscriptions");
                    return false
                }
            }
        }
    }
    true
}

/* -------------------------------------------------------------------------- */
/*                                 Public API                                 */
/* -------------------------------------------------------------------------- */

cfg_if::cfg_if! {
    if #[cfg(any(
        any(feature = "docs", doc),
        all(
            feature = "serde_bincode",
            not(feature = "serde_json"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_cbor",
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_json",
            not(feature = "serde_bincode"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_rmp",
            not(feature = "serde_cbor"),
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
        ),
    ))] {
    pub mod publisher;
    pub mod subscriber;
    }
}
