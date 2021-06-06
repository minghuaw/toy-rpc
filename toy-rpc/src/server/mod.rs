//! RPC server. There is only one `Server` defined, but some methods have
//! different implementations depending on the runtime feature flag
//!

use cfg_if::cfg_if;
use erased_serde as erased;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::message::TimeoutRequestBody;
use crate::service::{
    build_service, AsyncServiceMap, HandleService,
    HandlerResultFut, Service,
};
use crate::util::RegisterService;

#[cfg(all(feature = "http_actix_web"))]
#[cfg_attr(doc, doc(cfg(feature = "http_actix_web")))]
pub mod http_actix_web;

#[cfg(feature = "http_tide")]
#[cfg_attr(doc, doc(cfg(feature = "http_tide")))]
pub mod http_tide;

#[cfg(all(feature = "http_warp"))]
#[cfg_attr(doc, doc(cfg(feature = "http_warp")))]
pub mod http_warp;

#[cfg(any(
    feature = "docs", doc,
    feature = "async_std_runtime"
))]
pub mod async_std;

#[cfg(any(
    feature = "docs", doc,
    feature = "tokio_runtime"
))]
pub mod tokio;

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
    if #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))] {
        use flume::{Receiver, Sender};
        use std::io::ErrorKind;
        
        use crate::message::{ErrorMessage, RequestHeader, ResponseHeader};
        use crate::service::{HandlerResult};
        use crate::{
            codec::{
                split::{ServerCodecRead, ServerCodecWrite},
                RequestDeserializer,
            },
            message::{
                ExecutionMessage, ExecutionResult, MessageId, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM,
                RequestType, TIMEOUT_TOKEN, 
            },
        };
        use crate::error::Error;
        
        async fn reader_loop(
            mut codec_reader: impl ServerCodecRead,
            services: Arc<AsyncServiceMap>,
            executor: Sender<ExecutionMessage>,
            writer: Sender<ExecutionResult>,
        ) -> Result<(), Error> {
            // Keep reading until no header can be read
            while let Some(header) = codec_reader.read_request_header().await {
                let header = header?;
                // [0] read request body
                // This should be performed before processing because even if an
                // invalid request body should be consumed from the IO
                let deserializer = get_request_deserializer(&mut codec_reader).await?;

                match preprocess_header(&header) {
                    Ok(req_type) => {
                        match preprocess_request(&services, req_type, deserializer) {
                            Ok(msg) => { 
                                executor.send_async(msg).await?;
                            },
                            Err(err) => {
                                log::error!("{:?}", err);
                                match err {
                                    Error::ServiceNotFound => {
                                        writer.send_async(ExecutionResult {
                                            id: header.id,
                                            result: Err(Error::ServiceNotFound)
                                        }).await?;
                                    },
                                    _ => { }
                                }
                            }
                        }
                    },
                    Err(err) => {
                        // the only error returned is MethodNotFound,
                        // which should be sent back to client
                        writer.send_async(ExecutionResult {
                            id: header.id,
                            result: Err(err),
                        }).await?;
                    }
                }
            }
        
            // Stop the executor loop when client connection is gone
            executor.send_async(ExecutionMessage::Stop).await?;
        
            Ok(())
        }
        
        async fn execute_call(
            id: MessageId,
            fut: impl std::future::Future<Output = HandlerResult>,
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
            fut: impl std::future::Future<Output = HandlerResult>
        ) -> HandlerResult {
            #[cfg(all(
                feature = "async_std_runtime",
                not(feature = "tokio_runtime")
            ))]
            match ::async_std::future::timeout(duration, async {
                execute_call(id, fut).await
            }).await {
                Ok(res) => res,
                Err(_) => Err(Error::Timeout(Some(id)))
            }

            #[cfg(all(
                feature = "tokio_runtime",
                not(feature = "async_std_runtime")
            ))]
            match ::tokio::time::timeout(duration, async {
                execute_call(id, fut).await
            }).await {
                Ok(res) => res,
                Err(_) => Err(Error::Timeout(Some(id)))
            }
        }
        
        async fn writer_loop(
            mut codec_writer: impl ServerCodecWrite,
            results: Receiver<ExecutionResult>,
        ) -> Result<(), Error> {
            while let Ok(msg) = results.recv_async().await {
                writer_once(&mut codec_writer, msg).await
                    .unwrap_or_else(|e| log::error!("{:?}", e));
            }
            Ok(())
        }
        
        async fn writer_once(
            writer: &mut impl ServerCodecWrite,
            result: ExecutionResult,
        ) -> Result<(), Error> {
            let ExecutionResult { id, result } = result;
        
            match result {
                Ok(b) => {
                    log::trace!("Message {} Success", &id);
                    let header = ResponseHeader {
                        id,
                        is_error: false,
                    };
                    writer.write_response(header, &b).await?;
                }
                Err(err) => {
                    log::trace!("Message {} Error", &id);
                    let header = ResponseHeader { id, is_error: true };
                    let msg = ErrorMessage::from_err(err)?;
                    writer.write_response(header, &msg).await?;
                }
            };
            Ok(())
        }
        
        async fn get_request_deserializer(
            codec_reader: &mut impl ServerCodecRead,
        ) -> Result<RequestDeserializer, Error> {
            match codec_reader.read_request_body().await {
                Some(r) => r,
                None => {
                    let err = Error::IoError(std::io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "Failed to read message body",
                    ));
                    log::error!("{}", &err);
                    return Err(err);
                }
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


/// Server builder
pub struct ServerBuilder {
    services: AsyncServiceMap,
}

impl ServerBuilder {
    /// Creates a new `ServerBuilder`
    pub fn new() -> Self {
        ServerBuilder {
            services: HashMap::new(),
        }
    }

    /// Registers a new service to the `Server` with the default name.
    ///
    /// Internally the `Service` object will be built using the supplied `service`
    /// , which is the state of the `Service` object
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use async_std::net::TcpListener;
    /// use toy_rpc_macros::{export_impl};
    /// use toy_rpc::server::Server;
    ///
    /// struct EchoService { }
    ///
    /// #[export_impl]
    /// impl EchoService {
    ///     #[export_method]
    ///     async fn echo(&self, req: String) -> Result<String, String> {
    ///         Ok(req)
    ///     }
    /// }
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080";
    ///     
    ///     let echo_service = Arc::new(EchoService { });
    ///     let server = Server::builder()
    ///         .register(echo_service)
    ///         .build();
    ///     
    ///     let listener = TcpListener::bind(addr).await.unwrap();
    ///
    ///     let handle = task::spawn(async move {
    ///         server.accept(listener).await.unwrap();
    ///     });
    ///
    ///     handle.await;
    /// }
    /// ```
    pub fn register<S>(self, service: Arc<S>) -> Self
    where
        S: RegisterService + Send + Sync + 'static,
    {
        self.register_with_name(S::default_name(), service)
    }

    /// Register a a service with a name. This allows registering multiple instances
    /// of the same type on the server.
    ///
    /// Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use async_std::net::TcpListener;
    /// use toy_rpc::macros::export_impl;
    /// use toy_rpc::service::Service;
    /// use toy_rpc::Server;
    /// pub struct Foo { }
    ///
    /// #[export_impl]
    /// impl Foo {
    ///     #[export_method]
    ///     pub async fn increment(&self, arg: i32) -> Result<i32, String> {
    ///         Ok(arg + 1)
    ///     }
    /// }
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     let foo1 = Arc::new(Foo { });
    ///     let foo2 = Arc::new(Foo { });
    ///
    ///     // construct server
    ///     let server = Server::builder()
    ///         .register(foo1) // this will register `foo1` with the default name `Foo`
    ///         .register_with_name("Foo2", foo2) // this will register `foo2` with the name `Foo2`
    ///         .build();
    ///
    ///     let addr = "127.0.0.1:8080";
    ///     let listener = TcpListener::bind(addr).await.unwrap();
    ///
    ///     let handle = task::spawn(async move {
    ///         server.accept(listener).await.unwrap();
    ///     });
    ///
    ///     handle.await;
    /// }
    /// ```
    pub fn register_with_name<S>(self, name: &'static str, service: Arc<S>) -> Self
    where
        S: RegisterService + Send + Sync + 'static,
    {
        let service = build_service(service, S::handlers());
        self.register_service(name, service)
    }

    /// Register a `Service` instance. This allows registering multiple instances
    /// of the same type on the server.
    pub fn register_service<S>(self, name: &'static str, service: Service<S>) -> Self
    where
        S: Send + Sync + 'static,
    {
        let call = move |method_name: String,
                         _deserializer: Box<(dyn erased::Deserializer<'static> + Send)>|
              -> HandlerResultFut { service.call(&method_name, _deserializer) };

        log::debug!("Registering service: {}", name);
        let mut builder = self;
        builder.services.insert(name, Arc::new(call));
        builder
    }

    /// Creates an RPC `Server`
    pub fn build(self) -> Server {
        Server {
            services: Arc::new(self.services),
        }
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
