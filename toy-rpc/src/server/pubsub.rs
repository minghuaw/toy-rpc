use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use flume::{Sender, Receiver};

use crate::message::MessageId;

use super::{broker::ServerBrokerItem, ClientId};

pub(crate) enum PubSubItem {
    Publish {
        msg_id: MessageId,
        topic: String,
        content: Arc<Vec<u8>>,
    },
    Subscribe {
        client_id: ClientId,
        topic: String,
        sender: Sender<ServerBrokerItem>
    },
    Unsubscribe {
        client_id: ClientId,
        topic: String,
    },
    Stop
}

struct PubSubBroker {
    listener: Receiver<PubSubItem>,
    subscriptions: HashMap<String, BTreeMap<ClientId, Sender<ServerBrokerItem>>>
}

impl PubSubBroker {
    pub async fn pubsub_loop(&mut self) {
        while let Ok(item) = self.listener.recv_async().await {
            match item {
                PubSubItem::Publish {msg_id, topic, content} => {
                    if let Some(entry) = self.subscriptions.get(&topic) {
                        for sender in entry.values() {
                            let msg = ServerBrokerItem::Publication{
                                id: msg_id, 
                                topic: topic.clone(), 
                                content: content.clone()
                            };
                            sender.send_async(msg).await; // TODO: handle error
                        }
                    }
                },
                PubSubItem::Subscribe {client_id, topic, sender} => {
                    match self.subscriptions.get_mut(&topic) {
                        Some(entry) => {
                            entry.insert(client_id, sender);
                        },
                        None => {
                            let mut entry = BTreeMap::new();
                            entry.insert(client_id, sender);
                            self.subscriptions.insert(topic, entry);
                        }
                    }
                },
                PubSubItem::Unsubscribe {client_id, topic} => {
                    match self.subscriptions.get_mut(&topic) {
                        Some(entry) => {
                            entry.remove(&client_id);
                        },
                        None => { }
                    }
                },
                PubSubItem::Stop => {
                    return
                }
            }
        }
    }
}