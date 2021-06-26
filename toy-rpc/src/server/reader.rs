use std::sync::Arc;
use futures::{sink::{Sink, SinkExt}};
use brw::{Running, Reader};

use crate::{
    codec::{split::ServerCodecRead, RequestDeserializer}, 
    service::AsyncServiceMap,
    message::{
        ExecutionMessage, ExecutionResult, MessageId, CANCELLATION_TOKEN, 
        CANCELLATION_TOKEN_DELIM, RequestType, TIMEOUT_TOKEN, TimeoutRequestBody, RequestHeader
    }, 
    error::Error,
};

// use super::{preprocess_header, preprocess_request};

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

pub(crate) fn preprocess_header(header: &RequestHeader) -> Result<RequestType, Error> {
    match header.service_method.rfind('.') {
        Some(pos) => {
            // split service and method
            let service = header.service_method[..pos].to_string();
            let method = header.service_method[pos + 1..].to_string();
            Ok(RequestType::Request{
                id: header.id,
                service,
                method
            })
        },
        None => {
            // check for timeout request
            if header.service_method == TIMEOUT_TOKEN {
                Ok(RequestType::Timeout(header.id))
            // check for cancellation request
            } else if header.service_method == CANCELLATION_TOKEN {
                Ok(RequestType::Cancel(header.id))
            // Method is not provided
            } else {
                Err(Error::MethodNotFound)
            }
        }
    }
}

pub(crate) fn preprocess_request<'a> (
    services: &AsyncServiceMap,
    req_type: RequestType,
    mut deserializer: RequestDeserializer
) -> Result<ExecutionMessage, Error> {
    match req_type {
        RequestType::Timeout(id) => {
            let timeout_body: TimeoutRequestBody = erased_serde::deserialize(&mut deserializer)?;
            Ok(ExecutionMessage::TimeoutInfo(id, timeout_body.0))
        },
        RequestType::Cancel(id) => {
            let token: String = erased_serde::deserialize(&mut deserializer)?;
            if is_correct_cancellation_token(id, &token) {
                Ok(ExecutionMessage::Cancel(id))
            } else {
                // If the token is wrong, it should be considered as an InvalidArgument
                Err(Error::InvalidArgument)
            }
        },
        RequestType::Request{
            id,
            service,
            method
        } => {
            log::trace!("Message id: {}, service: {}, method: {}", id, service, method);

            // look up the service
            match services.get(&service[..]) {
                Some(call) => {
                    // send to executor
                    Ok(ExecutionMessage::Request {
                        call: call.clone(),
                        id: id,
                        method: method,
                        deserializer
                    })
                },
                None => {
                    log::error!("Service not found: {}", service);
                    Err(Error::ServiceNotFound)
                }
            }
        }
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