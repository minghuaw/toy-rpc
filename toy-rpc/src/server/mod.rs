//! RPC server. There is only one `Server` defined, but some methods have
//! different implementations depending on the runtime feature flag
//!

use cfg_if::cfg_if;
use std::sync::Arc;

use crate::{service::AsyncServiceMap};

mod integration;

mod broker;
mod reader;
mod writer;

#[cfg(any(
    feature = "docs",
    doc,
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"),)
))]
mod async_std;

#[cfg(any(
    feature = "docs",
    doc,
    all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
))]
mod tokio;

pub mod builder;
use builder::ServerBuilder;

/// RPC Server
///
/// ```
/// const DEFAULT_RPC_PATH: &str = "_rpc_";
/// ```
#[derive(Clone)]
pub struct Server {
    services: Arc<AsyncServiceMap>,
}

impl Server {
    /// Creates a `ServerBuilder`
    ///
    /// Example
    ///
    /// ```rust
    /// use toy_rpc::server::{ServerBuilder, Server};
    ///
    /// let builder: ServerBuilder = Server::builder();
    /// ```
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    ))] {
        use flume::{Sender};
        use std::future::Future;
        use std::time::Duration;
        use std::collections::HashMap;

        use crate::service::{HandlerResult};
        use crate::{
            codec::RequestDeserializer,
            message::{
                ExecutionMessage, ExecutionResult, MessageId, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM,
                RequestType, TIMEOUT_TOKEN, TimeoutRequestBody, RequestHeader
            },
        };
        use crate::error::Error;

        // Spawn tasks for the reader/broker/writer loops
        #[cfg(any(
            feature = "serde_bincode", 
            feature = "serde_json",
            feature = "serde_cbor",
            feature = "serde_rmp",
        ))]
        pub(crate) async fn serve_codec_setup(
            codec: impl crate::codec::split::SplittableServerCodec + 'static,
            services: Arc<AsyncServiceMap>
        ) -> Result<(), Error> {
            let (writer, reader) = codec.split();

            let reader = reader::ServerReader::new(reader, services);
            let writer = writer::ServerWriter::new(writer);
            let broker = broker::ServerBroker::new();

            let (mut broker_handle, _) = brw::spawn(broker, reader, writer);
            brw::util::Conclude::conclude(&mut broker_handle);
            Ok(())
        }

        /// Spawn the execution in a task and return the JoinHandle
        #[cfg(all(
            feature = "async_std_runtime",
            not(feature = "tokio_runtime")
        ))]
        fn handle_request(
            broker: Sender<ExecutionMessage>,
            durations: &mut HashMap<MessageId, Duration>,
            id: MessageId,
            fut: impl Future<Output=HandlerResult> + Send + 'static,
        ) -> ::async_std::task::JoinHandle<()> {
            match durations.remove(&id) {
                Some(duration) => {
                    ::async_std::task::spawn(async move {
                        let result = execute_timed_call(id, duration, fut).await;
                        let result = ExecutionResult { id, result };
                        broker.send_async(ExecutionMessage::Result(result)).await
                            .unwrap_or_else(|e| log::error!("{}", e));
                    })
                },
                None => {
                    ::async_std::task::spawn(async move {
                        let result = execute_call(id, fut).await;
                        let result = ExecutionResult { id, result };
                        broker.send_async(ExecutionMessage::Result(result)).await
                            .unwrap_or_else(|e| log::error!("{}", e));
                    })
                }
            }
        }

        /// Spawn the execution in a task and return the JoinHandle
        #[cfg(all(
            feature = "tokio_runtime",
            not(feature = "async_std_runtime")
        ))]
        fn handle_request(
            broker: Sender<ExecutionMessage>,
            durations: &mut HashMap<MessageId, Duration>,
            id: MessageId,
            fut: impl Future<Output=HandlerResult> + Send + 'static,
        ) -> ::tokio::task::JoinHandle<()> {
            match durations.remove(&id) {
                Some(duration) => {
                    ::tokio::task::spawn(async move {
                        let result = execute_timed_call(id, duration, fut).await;
                        let result = ExecutionResult { id, result };
                        broker.send_async(ExecutionMessage::Result(result)).await
                            .unwrap_or_else(|e| log::error!("{}", e));
                    })
                },
                None => {
                    ::tokio::task::spawn(async move {
                        let result = execute_call(id, fut).await;
                        let result = ExecutionResult { id, result };
                        broker.send_async(ExecutionMessage::Result(result)).await
                            .unwrap_or_else(|e| log::error!("{}", e));
                    })
                }
            }
        }

        async fn execute_call(
            id: MessageId,
            fut: impl Future<Output = HandlerResult>,
        ) -> HandlerResult {
            let result: HandlerResult = fut.await.map_err(|err| {
                log::error!(
                    "Error found executing request id: {}, error msg: {}",
                    &id,
                    &err
                );
                match err {
                    // if serde cannot parse request, the argument is likely mistaken
                    Error::ParseError(e) => {
                        log::error!("ParseError {:?}", e);
                        Error::InvalidArgument
                    }
                    e => e,
                }
            });
            result
        }

        async fn execute_timed_call(
            id: MessageId,
            duration: Duration,
            fut: impl Future<Output = HandlerResult>
        ) -> HandlerResult {
            #[cfg(all(
                feature = "async_std_runtime",
                not(feature = "tokio_runtime")
            ))]
            match ::async_std::future::timeout(
                duration,
                execute_call(id, fut)
            ).await {
                Ok(res) => res,
                Err(_) => Err(Error::Timeout(Some(id)))
            }

            #[cfg(all(
                feature = "tokio_runtime",
                not(feature = "async_std_runtime"),
            ))]
            match ::tokio::time::timeout(
                duration,
                execute_call(id, fut)
            ).await {
                Ok(res) => res,
                Err(_) => Err(Error::Timeout(Some(id)))
            }
        }

        fn preprocess_header(header: &RequestHeader) -> Result<RequestType, Error> {
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

        fn preprocess_request<'a> (
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
    }
}


