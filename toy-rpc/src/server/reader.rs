use brw::{Reader, Running};
use futures::sink::{Sink, SinkExt};
use std::sync::Arc;

use crate::{
    codec::CodecRead,
    error::Error,
    message::{MessageId, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM},
    pubsub::SeqId,
    service::{ArcAsyncServiceCall, AsyncServiceMap},
};

use super::broker::ServerBrokerItem;
use crate::protocol::{Header, InboundBody};

pub(crate) struct ServerReader<T> {
    reader: T,
    services: Arc<AsyncServiceMap>,
}

impl<T: CodecRead> ServerReader<T> {
    #[cfg(not(feature = "http_actix_web"))]
    pub fn new(reader: T, services: Arc<AsyncServiceMap>) -> Self {
        Self { reader, services }
    }
}

pub(crate) fn service(
    services: &Arc<AsyncServiceMap>,
    service_method: String,
) -> Result<(ArcAsyncServiceCall, String), Error> {
    // split service and method
    let args: Vec<&str> = service_method.split('.').collect();
    let (service, method) = match args[..] {
        [s, m] => (s, m),
        _ => {
            // Method not found
            return Err(Error::MethodNotFound);
        }
    };

    // look up the service
    match services.get(service) {
        Some(call) => Ok((call.clone(), method.into())),
        None => Err(Error::ServiceNotFound),
    }
}

pub(crate) fn handle_cancel(
    id: MessageId,
    mut deserializer: Box<InboundBody>,
) -> Result<(), Error> {
    let token: String = erased_serde::deserialize(&mut deserializer)?;
    if is_correct_cancellation_token(id, &token) {
        Ok(())
    } else {
        Err(Error::InvalidArgument)
    }
}

fn is_correct_cancellation_token(id: MessageId, token: &str) -> bool {
    match token.find(CANCELLATION_TOKEN_DELIM) {
        Some(ind) => {
            let base = &token[..ind];
            let id_str = &token[ind + 1..];
            let _id: MessageId = match id_str.parse() {
                Ok(num) => num,
                Err(_) => return false,
            };
            base == CANCELLATION_TOKEN && _id == id
        }
        None => false,
    }
}

#[async_trait::async_trait]
impl<T: CodecRead> Reader for ServerReader<T> {
    type BrokerItem = ServerBrokerItem;
    type Ok = ();
    type Error = Error;

    async fn op<B>(&mut self, mut broker: B) -> Running<Result<Self::Ok, Self::Error>>
    where
        B: Sink<Self::BrokerItem, Error = flume::SendError<Self::BrokerItem>> + Send + Unpin,
    {
        if let Some(header) = self.reader.read_header().await {
            let header: Header = match header {
                Ok(header) => header,
                Err(err) => return Running::Continue(Err(err.into())),
            };
            log::debug!("{:?}", &header);

            match header {
                Header::Request {
                    id,
                    service_method,
                    timeout,
                } => {
                    let deserializer = match self.reader.read_body().await {
                        Some(res) => match res {
                            Ok(de) => de,
                            Err(err) => return Running::Continue(Err(err.into())),
                        },
                        None => return Running::Stop,
                    };
                    match service(&self.services, service_method) {
                        Ok((call, method)) => {
                            let msg = ServerBrokerItem::Request {
                                call,
                                id,
                                method,
                                duration: timeout,
                                deserializer,
                            };
                            Running::Continue(broker.send(msg).await.map_err(|err| err.into()))
                        }
                        Err(err) => {
                            log::error!("{}", &err);
                            let msg = ServerBrokerItem::Response {
                                id,
                                result: Err(err),
                            };
                            Running::Continue(broker.send(msg).await.map_err(|err| err.into()))
                        }
                    }
                }
                Header::Response { id, is_ok } => {
                    let _ = match self.reader.read_body().await {
                        Some(res) => match res {
                            Ok(de) => de,
                            Err(err) => return Running::Continue(Err(err.into())),
                        },
                        None => return Running::Stop,
                    };
                    Running::Continue(Err(Error::Internal(
                        format!("Server received Response {{id: {}, is_ok: {}}}", id, is_ok).into(),
                    )))
                }
                Header::Cancel(id) => {
                    let deserializer = match self.reader.read_body().await {
                        Some(res) => match res {
                            Ok(de) => de,
                            Err(err) => return Running::Continue(Err(err.into())),
                        },
                        None => return Running::Stop,
                    };
                    match handle_cancel(id, deserializer) {
                        Ok(_) => {
                            let msg = ServerBrokerItem::Cancel(id);
                            Running::Continue(broker.send(msg).await.map_err(|err| err.into()))
                        }
                        Err(err) => {
                            let msg = ServerBrokerItem::Response {
                                id,
                                result: Err(err),
                            };
                            Running::Continue(broker.send(msg).await.map_err(|err| err.into()))
                        }
                    }
                }
                Header::Publish { id, topic } => {
                    let content = match self.reader.read_bytes().await {
                        Some(res) => match res {
                            Ok(b) => b,
                            Err(err) => return Running::Continue(Err(err.into())),
                        },
                        None => return Running::Stop,
                    };
                    Running::Continue(
                        broker
                            .send(ServerBrokerItem::Publish { id, topic, content })
                            .await
                            .map_err(|err| err.into()),
                    )
                }
                Header::Subscribe { id, topic } => {
                    let _ = self.reader.read_bytes().await;
                    Running::Continue(
                        broker
                            .send(ServerBrokerItem::Subscribe { id, topic })
                            .await
                            .map_err(|err| err.into()),
                    )
                }
                Header::Unsubscribe { id, topic } => {
                    let _ = self.reader.read_bytes().await;
                    Running::Continue(
                        broker
                            .send(ServerBrokerItem::Unsubscribe { id, topic })
                            .await
                            .map_err(|err| err.into()),
                    )
                }
                Header::Ack(id) => {
                    // There is no body frame for unsubscribe message
                    let seq_id = SeqId::new(id);
                    Running::Continue(
                        broker
                            .send(ServerBrokerItem::InboundAck { seq_id })
                            .await
                            .map_err(|err| err.into()),
                    )
                }
                Header::Produce {
                    id: _,
                    topic: _,
                    tickets: _,
                } => Running::Continue(Err(Error::Internal(
                    "Unexpected Header type (Header::Produce)".into(),
                ))),
                Header::Consume { id: _, topic: _ } => Running::Continue(Err(Error::Internal(
                    "Unexpected Header type (Header::Consume)".into(),
                ))),
                Header::Ext {
                    id: _,
                    content: _,
                    marker: _,
                } => Running::Continue(Err(Error::Internal(
                    "Unexpected Header type (Header::Ext)".into(),
                ))),
            }
        } else {
            // Stop is not needed on the server because server broker will send a stop to itself after stopping
            if let Err(err) = broker.send(ServerBrokerItem::Stopping).await {
                log::error!("{}", err)
            }

            Running::Stop
        }
    }

    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()> {
        if let Err(err) = res {
            log::error!("{}", err);
        }
        Running::Continue(())
    }
}
