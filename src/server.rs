use async_std::net::{TcpListener, TcpStream};
use async_std::sync::Arc;
use async_std::task;
use erased_serde as erased;
use futures::StreamExt;
use std::collections::HashMap;

#[cfg(features = "http_tide")]
use tide;

use crate::codec::{DefaultCodec, ServerCodec};
use crate::error::{Error, RpcError};
use crate::message::{MessageId, RequestHeader, ResponseHeader};
use crate::service::{
    ArcAsyncServiceCall, AsyncServiceMap, HandleService, HandlerResult, HandlerResultFut,
};

// #[cfg(feature = "tide")]
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
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Accepts connections on the listener and serves requests to default
    /// server for each incoming connection
    /// 
    /// # Example
    /// 
    /// See `toy-rpc/examples/server_client/` for the example
    pub async fn accept(&self, listener: TcpListener) -> Result<(), Error> {
        let mut incoming = listener.incoming();

        while let Some(conn) = incoming.next().await {
            let stream = conn?;
            log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

            task::spawn(Self::_serve_conn(stream, self.services.clone()));
        }

        Ok(())
    }

    /// Serves a single connection
    async fn _serve_conn(stream: TcpStream, services: Arc<AsyncServiceMap>) -> Result<(), Error> {
        // let _stream = stream;
        let peer_addr = stream.peer_addr()?;

        // using feature flag controlled default codec
        let codec = DefaultCodec::new(stream);

        // let fut = task::spawn_blocking(|| Self::_serve_codec(codec, services)).await;
        let fut = Self::_serve_codec(codec, services);

        let ret = fut.await;
        log::info!("Client disconnected from {}", peer_addr);
        ret
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
            // destructure header
            let RequestHeader { id, service_method } = header?;
            // let service_method = &service_method[..];
            let pos = service_method
                .rfind('.')
                .ok_or(Error::RpcError(RpcError::MethodNotFound))?;
            let service_name = &service_method[..pos];
            let method_name = service_method[pos + 1..].to_owned();

            log::info!(
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
            let bytes_sent = Self::_send_response(codec, id, res).await?;
            log::debug!("Response sent with {} bytes", bytes_sent);
        }

        Ok(())
    }

    /// Sends back the response with the specified codec
    async fn _send_response<C>(
        _codec: &mut C,
        id: MessageId,
        res: HandlerResult,
    ) -> Result<usize, Error>
    where
        C: ServerCodec + Send + Sync,
    {
        match res {
            Ok(b) => {
                log::info!("Message {} Success", id.clone());
                let header = ResponseHeader {
                    id,
                    is_error: false,
                };

                let bytes_sent = _codec.write_response(header, &b).await?;
                Ok(bytes_sent)
            }
            Err(e) => {
                log::info!("Message {} Error", id.clone());
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

    /// Serves a single connection using the default codec
    pub async fn serve_conn(&self, stream: TcpStream) -> Result<(), Error> {
        Self::_serve_conn(stream, self.services.clone()).await
    }

    /// This is like serve_conn except that it uses a specified codec
    pub async fn serve_codec<C>(&self, codec: C) -> Result<(), Error>
    where
        C: ServerCodec + Send + Sync,
    {
        Self::_serve_codec(codec, self.services.clone()).await
    }
}

// integration with http frameworks
impl Server {
    /// Creates a `tide::Endpoint` that handles http connections
    /// 
    /// The endpoint will be created with `DEFAULT_RPC_PATH` appended to the 
    /// end of the nested `tide` endpoint
    /// 
    /// # Example
    /// ```
    /// use toy_rpc::server::Server;
    /// 
    /// #[async_std::main]
    /// async fn main() -> tide::Result<()> {
    ///     let addr = "127.0.0.1:8888";
    ///     let foo_service = Arc::new(FooService { });
    /// 
    ///     let server = Server::builder()
    ///         .build();
    ///     
    ///     let mut app = tide::new();
    /// 
    ///     // If a network path were to be supplpied, 
    ///     // the network path must end with a slash "/"
    ///     app.at("/rpc/").nest(server.into_endpoint());
    /// 
    ///     app.listen(addr).await?;
    ///     Ok(())
    /// }
    /// ```
    ///  
    /// TODO: add a handler to test the connection
    #[cfg(feature = "tide")]
    pub fn into_endpoint(self) -> tide::Server<Self> {
        use futures::io::BufWriter;

        let mut app = tide::Server::with_state(self);
        app.at(DEFAULT_RPC_PATH)
            .all(|mut req: tide::Request<Server>| async move {
                let input = req.take_body().into_reader();
                let mut output: Vec<u8> = Vec::new();

                let mut codec = DefaultCodec::with_reader_writer(
                    input,
                    BufWriter::new(&mut output),
                );
                let services = req.state().services.clone();

                Self::_serve_codec_once(&mut codec, &services).await?;

                // construct tide::Response
                Ok(tide::Body::from_bytes(output))
            });

        app
    }

    // #[cfg(feature = "tide")]
    // pub fn handle_http(self) -> tide::Server<Self> {
        
    // }
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
    /// use toy_rpc_macros::{export_impl, service};
    /// use toy_rpc::server::Server;
    /// 
    /// EchoService { }
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
    ///         build();
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
