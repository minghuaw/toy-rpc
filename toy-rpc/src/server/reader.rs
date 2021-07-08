use std::{sync::Arc};
use futures::{sink::{Sink, SinkExt}};
use brw::{Running, Reader};

use crate::{codec::{CodecRead}, error::Error, message::{
        MessageId, CANCELLATION_TOKEN, 
        CANCELLATION_TOKEN_DELIM,
    }, service::{ArcAsyncServiceCall, AsyncServiceMap}};

use crate::protocol::{InboundBody, Header};
use super::broker::ServerBorkerItem;

pub(crate) struct ServerReader<T: CodecRead>{
    reader: T,
    services: Arc<AsyncServiceMap>,
}

impl<T: CodecRead> ServerReader<T> {
    pub fn new(reader: T, services: Arc<AsyncServiceMap>) -> Self {
        Self {
            reader,
            services
        }
    }

    async fn handle_request(
        &mut self, 
        service_method: String,
    ) -> Result<(ArcAsyncServiceCall, String), Error> {
        // split service and method
        let args: Vec<&str> = service_method.split('.').collect();
        let (service, method) = match args[..] {
            [s, m] => (s, m),
            _ => {
                // Method not found
                return Err(Error::MethodNotFound)
            }
        };

        // look up the service
        match self.services.get(service) {
            Some(call) => {
                Ok((call.clone(), method.into()))
            },
            None => {
                Err(Error::ServiceNotFound)
            }
        }
    }

    async fn handle_cancel(
        &mut self,
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
}

#[async_trait::async_trait]
impl<T: CodecRead> Reader for ServerReader<T> {
    type BrokerItem = ServerBorkerItem;
    type Ok = ();
    type Error = Error;

    async fn op<B>(&mut self, mut broker: B) -> Running<Result<Self::Ok, Self::Error>>
    where B: Sink<Self::BrokerItem, Error = flume::SendError<Self::BrokerItem>> + Send + Unpin {
        if let Some(header) = self.reader.read_header().await {
            let header: Header = match header {
                Ok(header) => header,
                Err(err) => return Running::Continue(Err(err))
            };
            let deserializer = match self.reader.read_body().await {
                Some(res) => {
                    match res {
                        Ok(de) => de,
                        Err(err) => return Running::Continue(Err(err))
                    }
                },
                None => return Running::Stop
            };

            match header {
                Header::Request{id, service_method, timeout} => {
                    match self.handle_request(service_method).await {
                        Ok((call, method)) => {
                            let msg = ServerBorkerItem::Request {
                                call,
                                id,
                                method,
                                duration: timeout,
                                deserializer
                            };
                            Running::Continue(
                                broker.send(msg).await
                                    .map_err(|err| err.into())
                            )
                        },
                        Err(err) => {
                            log::error!("{}", err);
                            let msg = ServerBorkerItem::Response{id, result: Err(err)};
                            Running::Continue(
                                broker.send(msg).await
                                    .map_err(|err| err.into())
                            )
                        }
                    }
                },
                Header::Response{id, is_ok} => {
                    log::error!("Server received Response {{id: {}, is_ok: {}}}", id, is_ok);
                    Running::Continue(Ok(()))
                },
                Header::Cancel(id) => {
                    match self.handle_cancel(id, deserializer).await {
                        Ok(_) => {
                            let msg = ServerBorkerItem::Cancel(id);
                            Running::Continue(
                                broker.send(msg).await
                                    .map_err(|err| err.into())
                            )
                        },
                        Err(err) => {
                            let msg = ServerBorkerItem::Response{id, result: Err(err)};
                            Running::Continue(
                                broker.send(msg).await
                                    .map_err(|err| err.into())
                            )
                        }
                    }
                },
                Header::Publish{id, topic} => {
                    unimplemented!()
                },
                Header::Subscribe{id, topic} => {
                    unimplemented!()
                },
                Header::Ack(id) => {
                    unimplemented!()
                },
                Header::Produce { id, topic, tickets} => {
                    unimplemented!()
                },
                Header::Consume{id, topic} => {
                    unimplemented!()
                },
                Header::Ext {id, content, marker} => {
                    unimplemented!()
                }
            }
        } else {
            if broker.send(
                ServerBorkerItem::Stop
            ).await.is_ok() { }
            Running::Stop
        }
    }

    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()> {
        if let Err(err) = res {
            log::error!("{:?}", err);
        }
        Running::Continue(())
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