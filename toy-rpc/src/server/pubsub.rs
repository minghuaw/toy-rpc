use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use flume::{Sender, Receiver};

use crate::message::MessageId;

use super::{broker::ServerBrokerItem, ClientId};

enum PubSubItem {
    Publish {
        msg_id: MessageId,
        topic: String,
        content: Arc<Vec<u8>>,
    },
    Subscribe {
        client_id: ClientId,
        topic: String,
        sink: Sender<ServerBrokerItem>
    }
}

struct PubSubBroker {
    listener: Receiver<PubSubItem>,
    subscriptions: HashMap<String, BTreeMap<ClientId, Sender<ServerBrokerItem>>>
}

// impl PubSubBroker {
//     pub async fn pubsub_loop(&mut self) {
//         while let Ok(item) = self.listener.recv_async().await {
//             match item {
//                 PubSubItem::Publish{topic, }
//             }
//         }
//     }
// }