//! RPC server. There is only one `Server` defined, but some methods have
//! different implementations depending on the runtime feature flag
//!

use cfg_if::cfg_if;
use erased_serde as erased;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::sync::Arc;

use crate::codec::ServerCodec;
use crate::error::Error;
use crate::message::{ErrorMessage, MessageId, RequestHeader, ResponseHeader};
use crate::service::{
    build_service, ArcAsyncServiceCall, AsyncServiceMap, HandleService, HandlerResult,
    HandlerResultFut, Service
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

cfg_if! {
    if #[cfg(any(feature = "docs", doc))] {
        #[doc(cfg(any(
            feature = "async_std_runtime",
            feature = "http_tide",
        )))]
        pub mod async_std;

        #[doc(cfg(any(
            feature = "tokio_runtime",
            feature = "http_warp",
            feature = "http_actix_web"
        )))]
        pub mod tokio;
    } else if #[cfg(any(
        feature = "async_std_runtime",
        feature = "http_tide",
    ))] {
        pub mod async_std;
    } else if #[cfg(any(
        feature = "tokio_runtime",
        feature = "http_warp",
        feature = "http_actix_web"
    ))] {
        pub mod tokio;
    }
}

/// Default RPC path for http handler
pub const DEFAULT_RPC_PATH: &str = "_rpc_";

#[derive(Debug)]
pub enum ConnectionStatus {
    KeepReading,
    Stop,
}

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

    /// Serves using a specified codec
    async fn serve_codec_loop<C>(mut codec: C, services: Arc<AsyncServiceMap>) -> Result<(), Error>
    where
        C: ServerCodec + Send + Sync,
    {
        log::debug!("Start serving codec");
        loop {
            match Self::serve_codec_once(&mut codec, &services).await {
                Ok(stat) => match stat {
                    ConnectionStatus::KeepReading => {}
                    ConnectionStatus::Stop => {
                        log::debug!("Stop serving codec");
                        return Ok(());
                    }
                },
                Err(err) => {
                    log::error!("Error encountered serving codec: '{}'", err);
                }
            }
        }
    }

    /// Serves using the specified codec only once
    async fn serve_codec_once<C>(
        codec: &mut C,
        services: &Arc<AsyncServiceMap>,
    ) -> Result<ConnectionStatus, Error>
    where
        C: ServerCodec + Send + Sync,
    {
        match codec.read_request_header().await {
            Some(header) => {
                // [0] read request body
                let deserializer = match codec.read_request_body().await {
                    Some(r) => r?,
                    None => {
                        let err = Error::IoError(std::io::Error::new(
                            ErrorKind::UnexpectedEof,
                            "Failed to read message body",
                        ));
                        log::error!("{}", &err);
                        return Err(err);
                    }
                };

                // [1] destructure header
                let RequestHeader { id, service_method } = header?;

                // [2] split service name and method name
                // return early send back Error::MethodNotFound if no "." is found
                let pos = match service_method.rfind('.') {
                    Some(p) => p,
                    None => {
                        Self::send_response(codec, id, Err(Error::MethodNotFound)).await?;
                        log::error!("Method not supplied from request: '{}'", service_method);
                        return Ok(ConnectionStatus::KeepReading);
                    }
                };
                let service = &service_method[..pos];
                let method = &service_method[pos + 1..];
                log::trace!(
                    "Message id: {}, service: {}, method: {}",
                    id,
                    service,
                    method
                );

                // [3] look up the service
                // return early and send back Error::ServiceNotFound if key is not found
                let call: ArcAsyncServiceCall = match services.get(service) {
                    Some(serv_call) => serv_call.clone(),
                    None => {
                        Self::send_response(codec, id, Err(Error::ServiceNotFound)).await?;
                        log::error!("Service not found: '{}'", service);
                        return Ok(ConnectionStatus::KeepReading);
                    }
                };

                // [4] execute the call
                let res: HandlerResult = {
                    // pass ownership to the `call`
                    call(method.into(), deserializer).await.map_err(|err| {
                        log::error!(
                            "Error found calling service: '{}', method: '{}', error: '{}'",
                            service,
                            method,
                            err
                        );
                        match err {
                            // if serde cannot parse request, the argument is likely mistaken
                            Error::ParseError(e) => {
                                log::error!("ParseError {:?}", e);
                                Error::InvalidArgument
                            }
                            e @ _ => e,
                        }
                    })
                };

                // [5] send back result
                Self::send_response(codec, id, res).await?;
                Ok(ConnectionStatus::KeepReading)
            }
            None => Ok(ConnectionStatus::Stop),
        }
    }

    /// Sends back the response with the specified codec
    async fn send_response<C>(
        _codec: &mut C,
        id: MessageId,
        res: HandlerResult,
    ) -> Result<(), Error>
    where
        C: ServerCodec + Send + Sync,
    {
        match res {
            Ok(b) => {
                log::trace!("Message {} Success", id.clone());
                let header = ResponseHeader {
                    id,
                    is_error: false,
                };
                _codec.write_response(header, &b).await?;
                Ok(())
            }
            Err(err) => {
                log::trace!("Message {} Error", id.clone());
                let header = ResponseHeader { id, is_error: true };
                let msg = match ErrorMessage::from_err(err) {
                    Ok(m) => m,
                    Err(e) => {
                        log::error!("Cannot send back IoError or ParseError: {:?}", e);
                        return Err(e);
                    }
                };

                _codec.write_response(header, &msg).await?;
                Ok(())
            }
        }
    }

    /// This is like serve_conn except that it uses a specified codec
    ///
    /// Example
    ///
    /// ```rust
    /// use async_std::net::TcpStream;
    /// // the default `Codec` will be used as an example
    /// use toy_rpc::codec::Codec;
    ///
    /// #[async_std::main]
    /// async fn main() {
    ///     // assume the following connection can be established
    ///     let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    ///     let codec = Codec::new(stream);
    ///     
    ///     let server = Server::builder()
    ///         .register(example_service)
    ///         .build();
    ///     // assume `ExampleService` exist
    ///     let handle = task::spawn(async move {
    ///         server.serve_codec(codec).await.unwrap();
    ///     })    
    ///     handle.await;
    /// }
    /// ```
    pub async fn serve_codec<C>(&self, codec: C) -> Result<(), Error>
    where
        C: ServerCodec + Send + Sync,
    {
        Self::serve_codec_loop(codec, self.services.clone()).await
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
        S: RegisterService + Send + Sync + 'static
    {
        let service = build_service(service, S::handlers());
        self.register_service(name, service)
    }
    
    /// Register a `Service` instance. This allows registering multiple instances
    /// of the same type on the server. 
    pub fn register_service<S>(self, name: &'static str, service: Service<S>) -> Self 
    where 
        S: Send + Sync + 'static
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
