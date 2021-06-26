use std::sync::Arc;
use futures::{sink::{Sink, SinkExt}};
use brw::{Running, Reader};

use crate::{
    codec::split::ServerCodecRead, service::AsyncServiceMap,
    message::{ExecutionMessage, ExecutionResult}, error::Error,
};

use super::{preprocess_header, preprocess_request};

pub(crate) struct ServerReader<T: ServerCodecRead>{
    reader: T,
    services: Arc<AsyncServiceMap>,
}

impl<T: ServerCodecRead> ServerReader<T> {
    pub fn new(reader: T, services: Arc<AsyncServiceMap>) -> Self {
        Self {
            reader,
            services
        }
    }
}

#[async_trait::async_trait]
impl<T: ServerCodecRead> Reader for ServerReader<T> {
    type BrokerItem = ExecutionMessage;
    type Ok = ();
    type Error = Error;

    async fn op<B>(&mut self, mut broker: B) -> Running<Result<Self::Ok, Self::Error>>
    where B: Sink<Self::BrokerItem, Error = flume::SendError<Self::BrokerItem>> + Send + Unpin {
        if let Some(header) = self.reader.read_request_header().await {
            let header = match header {
                Ok(header) => header,
                Err(err) => return Running::Continue(Err(err))
            };
            let deserializer = match self.reader.read_request_body().await {
                Some(res) => {
                    match res {
                        Ok(de) => de,
                        Err(err) => return Running::Continue(Err(err))
                    }
                },
                None => return Running::Stop
            };

            match preprocess_header(&header) {
                Ok(req_type) => {
                    match preprocess_request(&self.services, req_type, deserializer) {
                        Ok(msg) => {
                            match broker.send(msg).await {
                                Ok(_) => { },
                                Err(err) => return Running::Continue(Err(err.into()))
                            }
                        },
                        Err(err) => {
                            log::error!("{}", err);
                            match err {
                                Error::ServiceNotFound => {
                                    let result = ExecutionResult {
                                        id: header.id,
                                        result: Err(Error::ServiceNotFound)
                                    };
                                    match broker.send(
                                        ExecutionMessage::Result(result)
                                    ).await{
                                        Ok(_) => { },
                                        Err(err) => return Running::Continue(Err(err.into()))
                                    };
                                },
                                _ => { }
                            }
                        }
                    }
                },
                Err(err) => {
                    // the only error returned should be MethodNotFound,
                    // which should be sent back to client
                    let result = ExecutionResult {
                        id: header.id,
                        result: Err(err),
                    };
                    match broker.send(
                        ExecutionMessage::Result(result)
                    ).await {
                        Ok(_) => { },
                        Err(err) => return Running::Continue(Err(err.into()))
                    }
                }
            }
            Running::Continue(Ok(()))
        } else {
            if broker.send(
                ExecutionMessage::Stop
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