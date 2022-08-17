use super::broker::ClientBrokerItem;
use crate::protocol::{Header, InboundBody};
use crate::pubsub::SeqId;
use crate::{codec::CodecRead, Error};

pub(crate) struct ClientReader<R> {
    pub reader: R,
}

impl<R: CodecRead> ClientReader<R> {
    pub(crate) async fn op(
        &mut self,
    ) -> Option<Result<ClientBrokerItem, Error>> {
        let header = self.reader.read_header().await?;
        let header: Header = match header {
            Ok(header) => header,
            Err(err) => return Some(Err(err.into()))
        };

        match header {
            Header::Response { id, is_ok } => {
                // Ack will not come with a body
                let deserializer: Box<InboundBody> = match self.reader.read_body().await {
                    Some(res) => match res {
                        Ok(de) => de,
                        Err(err) => return Some(Err(err.into())),
                    },
                    None => return None
                };
                let result = match is_ok {
                    true => Ok(deserializer),
                    false => Err(deserializer),
                };

                Some(Ok(ClientBrokerItem::Response { id, result }))
            }
            Header::Publish { id, topic } => {
                let result = self.reader.read_body().await?;
                let deserializer: Box<InboundBody> = match result {
                    Ok(de) => de,
                    Err(err) => return Some(Err(err.into())),
                };
                Some(Ok(ClientBrokerItem::Subscription {
                    id: SeqId::new(id),
                    topic,
                    item: deserializer,
                }))
            }
            Header::Ack(id) => {
                let seq_id = SeqId::new(id);
                Some(Ok(ClientBrokerItem::InboundAck(seq_id)))
            }
            _ => Some(Err(Error::Internal("Unexpected Header type".into()))),
        }
    }
}
