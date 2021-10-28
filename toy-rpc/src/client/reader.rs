use async_trait::async_trait;
use brw::Running;
use futures::Sink;
use futures::SinkExt;

use super::broker::ClientBrokerItem;
use crate::error::CodecError;
use crate::protocol::{Header, InboundBody};
use crate::pubsub::SeqId;
use crate::{codec::CodecRead, Error};

pub(crate) struct ClientReader<R> {
    pub reader: R,
}

#[async_trait]
impl<R: CodecRead> brw::Reader for ClientReader<R> {
    type BrokerItem = ClientBrokerItem;
    type Ok = ();
    type Error = Error;

    async fn op<B>(&mut self, mut broker: B) -> Running<Result<Self::Ok, Self::Error>>
    where
        B: Sink<Self::BrokerItem, Error = flume::SendError<Self::BrokerItem>> + Send + Unpin,
    {
        if let Some(header) = self.reader.read_header().await {
            let header: Header = match header {
                Ok(header) => header,
                Err(err) => {
                    match err {
                        CodecError::IoError(e) => {
                            // pass back IoError
                            let _ = broker.send(ClientBrokerItem::Stop(Some(e))).await;
                            return Running::Stop;
                        }
                        _ => return Running::Continue(Err(err.into())),
                    }
                }
            };
            log::debug!("{:?}", &header);

            match header {
                Header::Response { id, is_ok } => {
                    // Ack will not come with a body
                    let deserializer: Box<InboundBody> = match self.reader.read_body().await {
                        Some(res) => match res {
                            Ok(de) => de,
                            Err(err) => return Running::Continue(Err(err.into())),
                        },
                        None => return Running::Stop,
                    };
                    let result = match is_ok {
                        true => Ok(deserializer),
                        false => Err(deserializer),
                    };

                    if let Err(err) = broker.send(ClientBrokerItem::Response { id, result }).await {
                        return Running::Continue(Err(err.into()));
                    }
                    Running::Continue(Ok(()))
                }
                Header::Publish { id, topic } => {
                    let deserializer: Box<InboundBody> = match self.reader.read_body().await {
                        Some(res) => match res {
                            Ok(de) => de,
                            Err(err) => return Running::Continue(Err(err.into())),
                        },
                        None => return Running::Stop,
                    };
                    Running::Continue(
                        broker
                            .send(ClientBrokerItem::Subscription {
                                id: SeqId::new(id),
                                topic,
                                item: deserializer,
                            })
                            .await
                            .map_err(|err| err.into()),
                    )
                }
                Header::Ack(id) => {
                    let seq_id = SeqId::new(id);
                    Running::Continue(
                        broker
                            .send(ClientBrokerItem::InboundAck(seq_id))
                            .await
                            .map_err(|err| err.into()),
                    )
                }
                _ => Running::Continue(Err(Error::Internal("Unexpected Header type".into()))),
            }
        } else {
            // A WebSocket Close frame will return None to the reader
            if broker.send(ClientBrokerItem::Stop(None)).await.is_ok() {}
            Running::Stop
        }
    }
}
