use async_std::net::{TcpListener, TcpStream};
use async_std::sync::Arc;
use async_std::task;
use erased_serde as erased;
use futures::StreamExt;
use std::collections::HashMap;

use crate::codec::{DefaultCodec, ServerCodec};
use crate::error::{Error, RpcError};
use crate::message::{MessageId, RequestHeader, ResponseHeader};
use crate::service::{
    ArcAsyncServiceCall, AsyncServiceMap, HandleService, HandlerResult, HandlerResultFut,
};
use crate::transport::ws::WebSocketConn;

#[cfg(all(feature = "actix-web"))]
pub mod http_actix_web;

#[cfg(feature = "tide")]
pub mod http_tide;

#[cfg(feature = "warp")]
pub mod http_warp;

/// Default RPC path for http handler
pub const DEFAULT_RPC_PATH: &str = "_rpc_";

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
        loop {
            Self::_serve_codec_once(&mut codec, &services).await?
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    /// Serves using the specified codec only once
    async fn _serve_codec_once<C>(
        codec: &mut C,
        services: &Arc<AsyncServiceMap>,
    ) -> Result<(), Error>
    where
        C: ServerCodec + Send + Sync,
    {
        if let Some(header) = codec.read_request_header().await {
            log::debug!("Request header: {:?}", &header);

            // destructure header
            let RequestHeader { id, service_method } = header?;
            // let service_method = &service_method[..];
            let pos = service_method
                .rfind('.')
                .ok_or(Error::RpcError(RpcError::MethodNotFound))?;
            let service_name = &service_method[..pos];
            let method_name = service_method[pos + 1..].to_owned();

            log::debug!(
                "Message {}, service: {}, method: {}",
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
                log::debug!("Reading request body");

                let deserializer = codec.read_request_body().await.unwrap()?;

                log::debug!("Calling handler");

                // pass ownership to the `call`
                call(method_name, deserializer).await
            };

            // send back result
            Self::_send_response(codec, id, res).await?;
        }

        Ok(())
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
                log::debug!("Message {} Success", id.clone());

                let header = ResponseHeader {
                    id,
                    is_error: false,
                };

                log::debug!("Writing response id: {}", &id);

                _codec.write_response(header, &b).await?;
                Ok(())
            }
            Err(e) => {
                log::debug!("Message {} Error", id.clone());

                let header = ResponseHeader { id, is_error: true };
                let body = match e {
                    Error::RpcError(rpc_err) => Box::new(rpc_err),
                    _ => Box::new(RpcError::ServerError(e.to_string())),
                };

                //
                let bytes_sent = _codec.write_response(header, &body).await?;
                Ok(bytes_sent)
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
    ///     let stream = TcpStream::connect("127.0.0.1:8888").await.unwrap();
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

#[cfg(any(
    all(
        feature = "serde_bincode",
        not(feature = "serde_json"),
        not(feature = "serde_cbor"),
        not(feature = "serde_rmp"),
    ),
    all(
        feature = "serde_cbor",
        not(feature = "serde_json"),
        not(feature = "serde_bincode"),
        not(feature = "serde_rmp"),
    ),
    all(
        feature = "serde_json",
        not(feature = "serde_bincode"),
        not(feature = "serde_cbor"),
        not(feature = "serde_rmp"),
    ),
    all(
        feature = "serde_rmp",
        not(feature = "serde_cbor"),
        not(feature = "serde_json"),
        not(feature = "serde_bincode"),
    ),
    feature = "docs"
))]
/// The following impl block is controlled by feature flag. It is enabled
/// if and only if **exactly one** of the the following feature flag is turned on
/// - `serde_bincode`
/// - `serde_json`
/// - `serde_cbor`
/// - `serde_rmp`
impl Server {
    /// Accepts connections on the listener and serves requests to default
    /// server for each incoming connection
    ///
    /// This is enabled
    /// if and only if **exactly one** of the the following feature flag is turned on
    /// - `serde_bincode`
    /// - `serde_json`
    /// - `serde_cbor`
    /// - `serde_rmp`
    ///
    /// # Example
    ///
    /// ```rust
    /// use async_std::net::TcpListener;
    ///
    /// async fn main() {
    ///     // assume `ExampleService` exist
    ///     let example_service = ExampleService {};
    ///     let server = Server::builder()
    ///         .register("example", service!(example_service, ExampleService))
    ///         .build();
    ///
    ///     let listener = TcpListener::bind(addr).await.unwrap();
    ///
    ///     let handle = task::spawn(async move {
    ///         server.accept(listener).await.unwrap();
    ///     });
    ///     handle.await;
    /// }
    /// ```
    ///         
    /// See `toy-rpc/examples/server_client/` for the example
    pub async fn accept(&self, listener: TcpListener) -> Result<(), Error> {
        let mut incoming = listener.incoming();

        while let Some(conn) = incoming.next().await {
            let stream = conn?;

            // #[cfg(feature = "logging")]
            log::debug!("Accepting incoming connection from {}", stream.peer_addr()?);

            task::spawn(Self::_serve_conn(stream, self.services.clone()));
        }

        Ok(())
    }

    pub async fn accept_websocket(&self, listener: TcpListener) -> Result<(), Error> {
        let mut incoming = listener.incoming();

        while let Some(conn) = incoming.next().await {
            let stream = conn?;
            // #[cfg(feature = "logging")]
            log::debug!("Accepting incoming connection from {}", stream.peer_addr()?);

            let ws_stream = async_tungstenite::accept_async(stream).await?;

            task::spawn(Self::_serve_websocket(ws_stream, self.services.clone()));
        }

        unimplemented!()
    }

    /// Serves a single connection
    async fn _serve_conn(stream: TcpStream, services: Arc<AsyncServiceMap>) -> Result<(), Error> {
        // let _stream = stream;
        let _peer_addr = stream.peer_addr()?;

        // using feature flag controlled default codec
        let codec = DefaultCodec::new(stream);

        // let fut = task::spawn_blocking(|| Self::_serve_codec(codec, services)).await;
        let fut = Self::_serve_codec(codec, services);

        #[cfg(feature = "logging")]
        log::info!("Client disconnected from {}", _peer_addr);

        fut.await
    }

    /// Serves a single connection using the default codec
    ///
    /// This is enabled
    /// if and only if **exactly one** of the the following feature flag is turned on
    /// - `serde_bincode`
    /// - `serde_json`
    /// - `serde_cbor`
    /// - `serde_rmp`
    ///
    /// Example
    ///
    /// ```rust
    /// use async_std::net::TcpStream;
    ///
    /// async fn main() {
    ///     // assume `ExampleService` exist
    ///     let example_service = ExampleService {};
    ///     let server = Server::builder()
    ///         .register("example", service!(example_service, ExampleService))
    ///         .build();
    ///
    ///     let conn = TcpStream::connect(addr).await.unwrap();
    ///
    ///     let handle = task::spawn(async move {
    ///         server.serve_conn(conn).await.unwrap();
    ///     });
    ///     handle.await;
    /// }
    /// ```
    pub async fn serve_conn(&self, stream: TcpStream) -> Result<(), Error> {
        Self::_serve_conn(stream, self.services.clone()).await
    }

    async fn _serve_websocket(
        ws_stream: async_tungstenite::WebSocketStream<TcpStream>,
        services: Arc<AsyncServiceMap>,
    ) -> Result<(), Error> {
        let ws_stream = WebSocketConn::new(ws_stream);
        let codec = DefaultCodec::with_websocket(ws_stream);

        let fut = Self::_serve_codec(codec, services);

        #[cfg(feature = "logging")]
        log::info!("Client disconnected");

        fut.await
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
    ///     let addr = "127.0.0.1:8888";
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
    pub fn register<S, T>(self, service_name: &'static str, service: S) -> Self
    where
        S: HandleService<T> + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let call = move |method_name: String,
                         _deserializer: Box<(dyn erased::Deserializer<'static> + Send)>|
              -> HandlerResultFut { service.call(&method_name, _deserializer) };

        let mut ret = self;
        ret.services.insert(service_name, Arc::new(call));
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
