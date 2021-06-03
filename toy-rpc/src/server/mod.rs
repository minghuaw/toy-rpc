//! RPC server. There is only one `Server` defined, but some methods have
//! different implementations depending on the runtime feature flag
//!

use cfg_if::cfg_if;
use erased_serde as erased;
use std::collections::HashMap;
use std::sync::Arc;

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
        use crate::service::{ArcAsyncServiceCall, HandlerResult};
        use crate::{
            codec::{
                split::{ServerCodecRead, ServerCodecWrite},
                RequestDeserializer,
            },
            message::{
                ExecutionMessage, ExecutionResult, MessageId, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM
            },
        };
        use crate::error::Error;
        
        async fn serve_codec_reader_loop(
            mut codec_reader: impl ServerCodecRead,
            services: Arc<AsyncServiceMap>,
            executor: Sender<ExecutionMessage>,
            writer: Sender<ExecutionResult>,
        ) -> Result<(), Error> {
            // Keep reading until no header can be read
            while let Some(header) = codec_reader.read_request_header().await {
                // [0] read request body
                let mut deserializer = get_request_deserializer(&mut codec_reader).await?;
                // [1] destructure header
                let RequestHeader { id, service_method } = header?;
                // [2] split service name and method name
                let (service, method) = match preprocess_service_method(id, &service_method) {
                    Ok(pair) => pair,
                    Err(err) => {
                        match err {
                            Error::Canceled(_) => {
                                // double check the message body
                                let token: String = erased_serde::deserialize(&mut deserializer)?;
                                if is_correct_cancellation_token(id, &token) {
                                    executor.send_async(ExecutionMessage::Cancel(id)).await?;
                                }
                            },
                            Error::MethodNotFound => {
                                // should not stop the reader if the service is not found
                                writer
                                    .send_async(ExecutionResult {
                                        id,
                                        result: Err(err),
                                    })
                                    .await?;
                            },
                            Error::Timeout(id) => {
                                unimplemented!()
                            }
        
                            // Note: not using `_` in case of mishanlding of new additions of Error types
                            Error::IoError(_) => {},
                            Error::ParseError(_) => {},
                            Error::Internal(_) => {},
                            Error::InvalidArgument => {},
                            Error::ServiceNotFound => {},
                            Error::ExecutionError(_) => {},
                        }
                        continue;
                    }
                };
                log::trace!(
                    "Message id: {}, service: {}, method: {}",
                    id,
                    service,
                    method
                );
                // [3] look up the service
                let call: ArcAsyncServiceCall = match services.get(service) {
                    Some(serv_call) => serv_call.clone(),
                    None => {
                        log::error!("Service not found: '{}'", service);
        
                        // should not stop the reader if the method is not found
                        writer
                            .send_async(ExecutionResult {
                                id,
                                result: Err(Error::ServiceNotFound),
                            })
                            .await?;
                        continue;
                    }
                };
                // [4] send to executor
                executor
                    .send_async(ExecutionMessage::Request {
                        call,
                        id,
                        method: method.into(),
                        deserializer,
                    })
                    .await?;
            }
        
            // Stop the executor loop when client connection is gone
            executor.send_async(ExecutionMessage::Stop).await?;
        
            Ok(())
        }
        
        async fn serve_codec_execute_call(
            id: MessageId,
            fut: impl std::future::Future<Output = HandlerResult>,
            // executor: Sender<ExecutionMessage>,
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
            // // [6] send result to response writer
            // let result = ExecutionResult { id, result };
            // match executor.send_async(ExecutionMessage::Result(result)).await {
            //     Ok(_) => {}
            //     Err(err) => log::error!("Failed to send to response writer ({})", err),
            // };
            result
        }
        
        async fn serve_codec_writer_loop(
            mut codec_writer: impl ServerCodecWrite,
            results: Receiver<ExecutionResult>,
        ) -> Result<(), Error> {
            while let Ok(msg) = results.recv_async().await {
                match serve_codec_write_once(&mut codec_writer, msg).await {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            }
            Ok(())
        }
        
        async fn serve_codec_write_once(
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
        
        fn preprocess_service_method(id: MessageId, service_method: &str) -> Result<(&str, &str), Error> {
            let pos = match service_method.rfind('.') {
                Some(p) => p,
                None => {
                    // test whether a cancellation request is received
                    if service_method == CANCELLATION_TOKEN {
                        log::error!("Received cancellation request with id: {}", id);
                        return Err(Error::Canceled(Some(id)));
                    } else {
                        log::error!("Method not supplied from request: '{}'", service_method);
                        return Err(Error::MethodNotFound);
                    }
                }
            };
            let service = &service_method[..pos];
            let method = &service_method[pos + 1..];
            Ok((service, method))
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
