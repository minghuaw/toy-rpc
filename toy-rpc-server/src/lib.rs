#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! # An async RPC crate that mimics the `golang`'s `net/rpc` package and supports both `async-std` and `tokio`.
//!
//! <div align="center">
//! <!-- Crates version -->
//! <a href="https://crates.io/crates/toy-rpc">
//! <img src="https://img.shields.io/crates/v/toy-rpc.svg?style=flat"
//! alt="Crates.io version" />
//! </a>
//! <!-- docs.rs docs -->
//! <a href="https://docs.rs/toy-rpc">
//! <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat"
//! alt="docs.rs docs" />
//! </a>
//! <!-- Downloads -->
//! <a href="https://crates.io/crates/toy-rpc">
//! <img src="https://img.shields.io/crates/d/toy-rpc.svg?style=flat"
//! alt="Download" />
//! </a>
//! <a href="https://github.com/rust-secure-code/safety-dance/">
//! <img src="https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat"
//! alt="Unsafe Rust forbidden" />
//! </a>
//! </div>
//!
//! `toy-rpc` aims to be an easy-to-use async RPC tool that is inspired by golang's `net/rpc`'s API.
//! It supports both `async_std` and `tokio` runtimes over either TCP or TLS. Integration with
//! common HTTP server frameworks such as `actix_web`, `warp` and `tide` are provided.
//!
//! The overall usage and API should feel similar to that of the golang's `net/rpc` package. Some of the names are changed
//! to make them sound more "rusty". Because rust does not come with runtime reflection, attribute macros `#[export_impl]`
//! and `#[export_trait]` / `#[export_trait_impl]`, and attribute `#[export_method]` are used to mark functions "exported" in golang's
//! `net/rpc` perspective.
//!
//! More detailed usage can be found in the book and documentation.
//!
//! - [Book](https://minghuaw.github.io/toy-rpc/01_introduction.html)
//! - [Quickstart](https://minghuaw.github.io/toy-rpc/02_quickstart.html)
//! - [Documentation](https://docs.rs/toy-rpc/0.7.0-alpha.0/toy_rpc/)
//! - [GitHub repository](https://github.com/minghuaw/toy-rpc)
//! - [crate.io](https://crates.io/crates/toy-rpc)
//!
//! This crate uses `#![forbid(unsafe_code)]` to ensure no usage of `unsafe` in the crate.
//!
//! # Feature flags
//!
//! The feature flags can be put into three categories.
//!
//! Choice of runtime and HTTP framework integration
//!
//! - `async_std_runtime`: supports usage with `async-std`
//! - `tokio_runtime`: supports usage with `tokio`
//! - `http_tide`: enables `tide` integration on the server side. This also enables `async_std_runtime`
//! - `http_actix_web`: enables `actix-web` integration on the server side. This also enables `tokio_runtime`
//! - `http_warp`: enables integration with `warp` on the server side. This also enables `tokio_runtime`
//!
//! Choice of RPC server or client (both can be enabled at the same time)
//!
//! - `server`: enables RPC server
//! - `client`: enables RPC client
//!
//! Choice of serialization/deserialzation (only one should be enabled at a time)
//!
//! - `serde_bincode`: (default) the default codec will use `bincode`
//!     for serialization/deserialization
//! - `serde_json`: the default codec will use `serde_json`
//!     for `json` serialization/deserialization
//! - `serde_cbor`: the default codec will use `serde_cbor`
//!     for serialization/deserialization
//! - `serde_rmp`: the default codec will use `rmp-serde`
//!     for serialization/deserialization
//!
//! TLS support
//!
//! - `tls`: enables TLS support
//! 
//! Other trivial feature flags are listed below, and they are likely of no actual usage for you.
//! - `docs`
//! - `std`: `serde/std`. There is no actual usage right now.
//!
//! By default, only `serde_bincode` feature is enabled.
//! You must enable at least one runtime feature flag and the `server` and/or `client` to have something usable.
//!
//! ## Default features
//!
//! ```toml
//! default = ["serde_bincode"]
//! ```
//!
//! # Integration
//!
//! HTTP integration is provided for `actix-web`, `tide`, and `warp`. More details can be found
//! in the [Book/Integrations](https://minghuaw.github.io/toy-rpc/05_integration.html) and in
//! [examples](https://github.com/minghuaw/toy-rpc/tree/main/examples).
//!
//! # Quickstart Example
//!
//! A quickstart example with `tokio` runtime is provided in the [Book/Quickstart](https://minghuaw.github.io/toy-rpc/02_quickstart.html).
//!

use cfg_if::cfg_if;
use std::sync::Arc;

use toy_rpc_core::service::AsyncServiceMap;

#[cfg(all(feature = "http_actix_web"))]
#[cfg_attr(doc, doc(cfg(feature = "http_actix_web")))]
mod http_actix_web;

#[cfg(feature = "http_tide")]
#[cfg_attr(doc, doc(cfg(feature = "http_tide")))]
mod http_tide;

#[cfg(all(feature = "http_warp"))]
#[cfg_attr(doc, doc(cfg(feature = "http_warp")))]
mod http_warp;

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
        use brw::{Broker, Reader, Running, Writer};
        use futures::{
            sink::{Sink, SinkExt}
        };

        use toy_rpc_core::{
            service::{HandlerResult},
            codec::{
                split::{ServerCodecRead, ServerCodecWrite},
                RequestDeserializer,
            },
            message::{
                ErrorMessage, RequestHeader, ResponseHeader,
                ExecutionMessage, ExecutionResult, MessageId, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM,
                RequestType, TIMEOUT_TOKEN, TimeoutRequestBody
            },
            error::Error,
        };

        // Spawn tasks for the reader/broker/writer loops
        #[cfg(any(
            feature = "serde_bincode", 
            feature = "serde_json",
            feature = "serde_cbor",
            feature = "serde_rmp",
        ))]
        pub(crate) async fn serve_codec_setup(
            codec: impl toy_rpc_core::codec::split::SplittableServerCodec + 'static,
            services: Arc<AsyncServiceMap>
        ) -> Result<(), Error> {
            let (writer, reader) = codec.split();

            let reader = ServerReader { reader, services };
            let writer = ServerWriter { writer };
            let broker = ServerBroker {
                executions: HashMap::new(),
                durations: HashMap::new()   
            };

            log::debug!("Spawning brw");
            let (mut broker_handle, _) = brw::spawn(broker, reader, writer);
            // broker_handle.conclude();
            brw::util::Conclude::conclude(&mut broker_handle);
            Ok(())
        }


        struct ServerReader<T: ServerCodecRead>{
            reader: T,
            services: Arc<AsyncServiceMap>,
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

        struct ServerWriter<W> {
            writer: W,
        }
        impl<W: ServerCodecWrite> ServerWriter<W> {
            async fn write_one_message(&mut self, result: ExecutionResult) -> Result<(), Error> {
                let ExecutionResult { id, result } = result;

                match result {
                    Ok(body) => {
                        log::trace!("Message {} Success", &id);
                        let header = ResponseHeader {
                            id,
                            is_error: false,
                        };
                        self.writer.write_response(header, &body).await?;
                    }
                    Err(err) => {
                        log::trace!("Message {} Error", &id);
                        let header = ResponseHeader { id, is_error: true };
                        let msg = ErrorMessage::from_err(err)?;
                        self.writer.write_response(header, &msg).await?;
                    }
                };
                Ok(())
            }
        }

        #[async_trait::async_trait]
        impl<W: ServerCodecWrite> Writer for ServerWriter<W> {
            type Item = ExecutionResult;
            type Ok = ();
            type Error = Error;

            async fn op(&mut self, item: Self::Item) -> Running<Result<Self::Ok, Self::Error>> {
                let res = self.write_one_message(item).await;
                Running::Continue(res)
            }

            async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()> {
                if let Err(err) = res {
                    log::error!("{:?}", err);
                }
                Running::Continue(())
            }
        }

        #[cfg(any(
            feature = "docs",
            all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
        ))]
        use ::tokio::task::JoinHandle;
        #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
        use ::async_std::task::JoinHandle;

        struct ServerBroker { 
            executions: HashMap<MessageId, JoinHandle<()>>,
            durations: HashMap<MessageId, Duration>
        }

        #[async_trait::async_trait]
        impl Broker for ServerBroker {
            type Item = ExecutionMessage;
            type WriterItem = ExecutionResult;
            type Ok = ();
            type Error = Error;

            async fn op<W>(&mut self, ctx: & Arc<brw::Context<Self::Item>>, item: Self::Item, mut writer: W) -> Running<Result<Self::Ok, Self::Error>>
            where W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin {
                match item {
                    ExecutionMessage::TimeoutInfo(id, duration) => {
                        self.durations.insert(id, duration);
                        Running::Continue(Ok(()))
                    },
                    ExecutionMessage::Request{
                        call,
                        id,
                        method,
                        deserializer
                    } => {
                        let fut = call(method, deserializer);
                        let _broker = ctx.broker.clone();
                        let handle = handle_request(_broker, &mut self.durations, id, fut);
                        self.executions.insert(id, handle);
                        Running::Continue(Ok(()))
                    },
                    ExecutionMessage::Result(result) => {
                        self.executions.remove(&result.id);
                        let res: Result<(), Error> = writer.send(result).await
                            .map_err(|err| err.into());
                        Running::Continue(res)
                    },
                    ExecutionMessage::Cancel(id) => {
                        if let Some(handle) = self.executions.remove(&id) {
                            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                            handle.abort();
                            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                            handle.cancel().await;
                        }

                        Running::Continue(Ok(()))
                    },
                    ExecutionMessage::Stop => {
                        for (_, handle) in self.executions.drain() {
                            log::debug!("Stopping execution as client is disconnected");
                            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                            handle.abort();
                            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                            handle.cancel().await;
                        }
                        log::debug!("Client connection is closed");
                        Running::Stop
                    }
                }
            }
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


