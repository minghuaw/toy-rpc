use std::sync::Arc;

use crate::{
    codec::CodecRead,
    error::Error,
    message::{MessageId, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM},
    pubsub::SeqId,
    service::{ArcAsyncServiceCall, AsyncServiceMap},
    util::Running,
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

impl<T: CodecRead> ServerReader<T> {
    pub(crate) async fn handle_error(&mut self, error: Error) -> Result<Running, Error> {
        Err(error)
    }

    pub(crate) async fn op(&mut self) -> Option<Result<ServerBrokerItem, Error>> {
        let header = self.reader.read_header().await?;
        let header: Header = match header {
            Ok(header) => header,
            Err(err) => return Some(Err(err.into())),
        };

        match header {
            Header::Request {
                id,
                service_method,
                timeout,
            } => {
                let deserializer = match self.reader.read_body().await {
                    Some(res) => match res {
                        Ok(de) => de,
                        Err(err) => return Some(Err(err.into())),
                    },
                    None => return None,
                };
                match service(&self.services, service_method) {
                    Ok((call, method)) => {
                        let item = ServerBrokerItem::Request {
                            call,
                            id,
                            method,
                            duration: timeout,
                            deserializer,
                        };
                        Some(Ok(item))
                    }
                    Err(err) => {
                        log::error!("{}", &err);
                        let item = ServerBrokerItem::Response {
                            id,
                            result: Err(err),
                        };
                        Some(Ok(item))
                    }
                }
            }
            Header::Response { id, is_ok } => {
                let _ = match self.reader.read_body().await {
                    Some(res) => match res {
                        Ok(de) => de,
                        Err(err) => return Some(Err(err.into())),
                    },
                    None => return None,
                };
                Some(Err(Error::Internal(
                    format!("Server received Response {{id: {}, is_ok: {}}}", id, is_ok).into(),
                )))
            }
            Header::Cancel(id) => {
                let deserializer = match self.reader.read_body().await {
                    Some(res) => match res {
                        Ok(de) => de,
                        Err(err) => return Some(Err(err.into())),
                    },
                    None => return None,
                };
                match handle_cancel(id, deserializer) {
                    Ok(_) => {
                        let item = ServerBrokerItem::Cancel(id);
                        Some(Ok(item))
                    }
                    Err(err) => {
                        let item = ServerBrokerItem::Response {
                            id,
                            result: Err(err),
                        };
                        Some(Ok(item))
                    }
                }
            }
            Header::Publish { id, topic } => {
                let content = match self.reader.read_bytes().await {
                    Some(res) => match res {
                        Ok(b) => b,
                        Err(err) => return Some(Err(err.into())),
                    },
                    None => return None,
                };
                let item = ServerBrokerItem::Publish { id, topic, content };
                Some(Ok(item))
            }
            Header::Subscribe { id, topic } => {
                let _ = self.reader.read_bytes().await;
                let item = ServerBrokerItem::Subscribe { id, topic };
                Some(Ok(item))
            }
            Header::Unsubscribe { id, topic } => {
                let _ = self.reader.read_bytes().await;
                let item = ServerBrokerItem::Unsubscribe { id, topic };
                Some(Ok(item))
            }
            Header::Ack(id) => {
                // There is no body frame for unsubscribe message
                let seq_id = SeqId::new(id);
                let item = ServerBrokerItem::InboundAck { seq_id };
                Some(Ok(item))
            }
            Header::Produce {
                id: _,
                topic: _,
                tickets: _,
            } => Some(Err(Error::Internal(
                "Unexpected Header type (Header::Produce)".into(),
            ))),
            Header::Consume { id: _, topic: _ } => Some(Err(Error::Internal(
                "Unexpected Header type (Header::Consume)".into(),
            ))),
            Header::Ext {
                id: _,
                content: _,
                marker: _,
            } => Some(Err(Error::Internal(
                "Unexpected Header type (Header::Ext)".into(),
            ))),
        }
    }
}
