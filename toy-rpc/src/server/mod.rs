//! RPC server. There is only one `Server` defined, but some methods have
//! different implementations depending on the runtime feature flag
//!

use cfg_if::cfg_if;
use erased_serde as erased;
use std::collections::HashMap;
use std::sync::Arc;

use crate::codec::ServerCodec;
use crate::error::{Error, RpcError};
use crate::message::{MessageId, RequestHeader, ResponseHeader};
use crate::service::{
    ArcAsyncServiceCall, AsyncServiceMap, HandleService, HandlerResult, HandlerResultFut,
    build_service,
};
use crate::util::{RegisterService};

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
    async fn _serve_codec<C>(mut codec: C, services: Arc<AsyncServiceMap>) -> Result<(), Error>
    where
        C: ServerCodec + Send + Sync,
    {
        log::debug!("Start serving codec");
        loop {
            match Self::_serve_codec_once(&mut codec, &services).await {
                Ok(stat) => match stat {
                    ConnectionStatus::KeepReading => {}
                    ConnectionStatus::Stop => return Ok(()),
                },
                Err(e) => {
                    log::error!("Error encountered serving codec \n{}", e);
                    return Err(e);
                }
            }
        }
    }

    /// Serves using the specified codec only once
    async fn _serve_codec_once<C>(
        codec: &mut C,
        services: &Arc<AsyncServiceMap>,
    ) -> Result<ConnectionStatus, Error>
    where
        C: ServerCodec + Send + Sync,
    {
        match codec.read_request_header().await {
            Some(header) => {
                // destructure header
                let RequestHeader { id, service_method } = header?;
                // let service_method = &service_method[..];
                let pos = service_method
                    .rfind('.')
                    .ok_or(Error::RpcError(RpcError::MethodNotFound))?;
                let service_name = &service_method[..pos];
                let method_name = service_method[pos + 1..].to_owned();

                log::trace!(
                    "Message id: {}, service: {}, method: {}",
                    id,
                    service_name,
                    method_name
                );

                // look up the service
                // TODO; consider adding a new error type
                let call: ArcAsyncServiceCall = services
                    .get(service_name)
                    .ok_or(Error::RpcError(RpcError::MethodNotFound))?
                    .clone();

                // read body
                let res = {
                    let deserializer = codec.read_request_body().await.unwrap()?;
                    // pass ownership to the `call`
                    call(method_name, deserializer).await
                };

                // send back result
                Self::_send_response(codec, id, res).await?;

                Ok(ConnectionStatus::KeepReading)
            }
            None => Ok(ConnectionStatus::Stop),
        }
    }

    /// Sends back the response with the specified codec
    async fn _send_response<C>(
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
            Err(e) => {
                log::trace!("Message {} Error", id.clone());

                let header = ResponseHeader { id, is_error: true };
                let body = match e {
                    Error::RpcError(rpc_err) => Box::new(rpc_err),
                    _ => Box::new(RpcError::ServerError(e.to_string())),
                };

                _codec.write_response(header, &body).await?;
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
    ///         .register("example", service!(example_service, ExampleService))
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
        Self::_serve_codec(codec, self.services.clone()).await
    }
}

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

    /// Registers a new service to the `Server`
    ///
    /// # Example
    ///
    /// ```
    /// use async_std::net::TcpListener;
    /// use async_std::sync::Arc;
    /// use toy_rpc_macros::{export_impl, service};
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
    ///         .register("echo_service", service!(echo_service, EchoService))
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
    // pub fn register<S, T>(self, service_name: &'static str, service: S) -> Self
    // where
    //     S: HandleService<T> + Send + Sync + 'static,
    //     T: Send + Sync + 'static,
    // {
    //     let call = move |method_name: String,
    //                      _deserializer: Box<(dyn erased::Deserializer<'static> + Send)>|
    //           -> HandlerResultFut { service.call(&method_name, _deserializer) };

    //     let mut ret = self;
    //     ret.services.insert(service_name, Arc::new(call));
    //     ret
    // }

    pub fn register<S>(self, service: Arc<S>) -> Self 
    where 
        S: RegisterService + Send + Sync + 'static,
    {
        let name = S::default_name();
        self.register_with_name(service, name)
    }

    pub fn register_with_name<S>(self, service: Arc<S>, name: &'static str) -> Self 
    where 
        S: RegisterService + Send + Sync + 'static,
    {
        let service = build_service(service, S::handlers());
        let call = move |method_name: String,
                         _deserializer: Box<(dyn erased::Deserializer<'static> + Send)>|
              -> HandlerResultFut { service.call(&method_name, _deserializer) };

        let mut ret = self;
        ret.services.insert(name, Arc::new(call));
        ret
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
