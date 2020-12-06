use async_std::net::{TcpListener, TcpStream};
use async_std::sync::Arc;
use async_std::task;
use erased_serde as erased;
use futures::StreamExt;
use std::collections::HashMap;

#[cfg(feature = "actix-web")]
use actix_web::web;

use crate::codec::{DefaultCodec, ServerCodec};
use crate::error::{Error, RpcError};
use crate::message::{MessageId, RequestHeader, ResponseHeader};
use crate::service::{
    ArcAsyncServiceCall, AsyncServiceMap, HandleService, HandlerResult, HandlerResultFut,
};

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
            // destructure header
            let RequestHeader { id, service_method } = header?;
            // let service_method = &service_method[..];
            let pos = service_method
                .rfind('.')
                .ok_or(Error::RpcError(RpcError::MethodNotFound))?;
            let service_name = &service_method[..pos];
            let method_name = service_method[pos + 1..].to_owned();

            #[cfg(feature = "logging")]
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
                #[cfg(feature = "logging")]
                log::debug!("Reading request body");

                let deserializer = codec.read_request_body().await.unwrap()?;

                #[cfg(feature = "logging")]
                log::debug!("Calling handler");

                // pass ownership to the `call`
                call(method_name, deserializer).await
            };

            // send back result
            let _bytes_sent = Self::_send_response(codec, id, res).await?;

            #[cfg(feature = "logging")]
            log::debug!("Response sent with {} bytes", _bytes_sent);
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
                #[cfg(feature = "logging")]
                log::info!("Message {} Success", id.clone());

                let header = ResponseHeader {
                    id,
                    is_error: false,
                };

                _codec.write_response(header, b).await?;
                Ok(())
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                log::info!("Message {} Error", id.clone());

                let header = ResponseHeader { id, is_error: true };
                let body = match e {
                    Error::RpcError(rpc_err) => Box::new(rpc_err),
                    _ => Box::new(RpcError::ServerError(e.to_string())),
                };

                //
                let bytes_sent = _codec.write_response(header, body).await?;
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

            #[cfg(feature = "logging")]
            log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

            task::spawn(Self::_serve_conn(stream, self.services.clone()));
        }

        Ok(())
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

    #[cfg(feature = "actix-web")]
    async fn _handle_http(
        state: web::Data<Server>,
        req_body: web::Bytes,
    ) -> Result<web::Bytes, actix_web::Error> {
        use futures::io::{BufReader, BufWriter};

        let input = req_body.to_vec();
        let mut output: Vec<u8> = Vec::new();

        let mut codec =
            DefaultCodec::with_reader_writer(BufReader::new(&*input), BufWriter::new(&mut output));
        let services = state.services.clone();

        Self::_serve_codec_once(&mut codec, &services)
            .await
            .map_err(|e| actix_web::Error::from(e))?;

        // construct response
        Ok(web::Bytes::from(output))
    }

    #[cfg(feature = "actix-web")]
    async fn _handle_connect() -> Result<String, actix_web::Error> {
        Ok("CONNECT request is received".to_string())
    }

    #[cfg(any(feature = "tide", feature = "docs"))]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "tide")))]
    /// Creates a `tide::Endpoint` that handles http connections.
    /// A convienient function `handle_http` can be used to achieve the same thing
    /// with `tide` feature turned on
    ///
    /// The endpoint will be created with `DEFAULT_RPC_PATH` appended to the
    /// end of the nested `tide` endpoint.
    ///
    /// This is enabled
    /// if and only if **exactly one** of the the following feature flag is turned on
    /// - `serde_bincode`
    /// - `serde_json`
    /// - `serde_cbor`
    /// - `serde_rmp`
    ///
    /// # Example
    /// ```
    /// use toy_rpc::server::Server;
    /// use toy_rpc::macros::{export_impl, service};
    /// use async_std::sync::Arc;
    ///
    /// struct FooService { }
    ///
    /// #[export_impl]
    /// impl FooService {
    ///     // define some "exported" functions
    /// }
    ///
    /// #[async_std::main]
    /// async fn main() -> tide::Result<()> {
    ///     let addr = "127.0.0.1:8888";
    ///     let foo_service = Arc::new(FooService { });
    ///
    ///     let server = Server::builder()
    ///         .register("foo", service!(foo_service, FooService))
    ///         .build();
    ///     
    ///     let mut app = tide::new();
    ///
    ///     // If a network path were to be supplied,
    ///     // the network path must end with a slash "/"
    ///     app.at("/rpc/").nest(server.into_endpoint());
    ///
    ///     // `handle_http` is a conenient function that calls `into_endpoint`
    ///     // with the `tide` feature turned on
    ///     //app.at("/rpc/").nest(server.handle_http());
    ///
    ///     app.listen(addr).await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    pub fn into_endpoint(self) -> tide::Server<Server> {
        use futures::io::BufWriter;
        // let server = self.build();

        let mut app = tide::Server::with_state(self);
        app.at(DEFAULT_RPC_PATH)
            .connect(|_| async move { Ok("CONNECT request is received") })
            .post(|mut req: tide::Request<Server>| async move {
                let input = req.take_body().into_reader();
                let mut output: Vec<u8> = Vec::new();

                let mut codec =
                    DefaultCodec::with_reader_writer(input, BufWriter::new(&mut output));
                let services = req.state().services.clone();

                Server::_serve_codec_once(&mut codec, &services).await?;

                // construct tide::Response
                Ok(tide::Body::from_bytes(output))
            });

        app
    }

    #[cfg(any(feature = "actix-web", feature = "docs"))]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "actix-web")))]
    /// Configuration for integration with an actix-web scope.
    /// A convenient funciont "handle_http" may be used to achieve the same thing
    /// with the `actix-web` feature turned on.
    ///
    /// The `DEFAULT_RPC_PATH` will be appended to the end of the scope's path.
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
    /// use toy_rpc::Server;
    /// use toy_rpc::macros::{export_impl, service};
    /// use actix_web::{App, HttpServer, web};
    ///
    /// struct FooService { }
    ///
    /// #[export_impl]
    /// impl FooService {
    ///     // define some "exported" functions
    /// }
    ///
    /// #[actix::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let addr = "127.0.0.1:8888";
    ///     
    ///     let foo_service = Arc::new(FooService { });
    ///
    ///     let server = Server::builder()
    ///         .register("foo_service", service!(foo_servicem FooService))
    ///         .build();
    ///
    ///     let app_data = web::Data::new(server);
    ///
    ///     HttpServer::new(
    ///         move || {
    ///             App::new()
    ///                 .service(
    ///                     web::scope("/rpc/")
    ///                         .app_data(app_data.clone())
    ///                         .configure(Server::scope_config)
    ///                         // The line above may be replaced with line below
    ///                         //.configure(Server::handle_http()) // use the convenience `handle_http`
    ///                 )
    ///         }
    ///     )
    ///     .bind(addr)?
    ///     .run()
    ///     .await
    /// }
    /// ```
    ///
    pub fn scope_config(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/")
                // .app_data(data)
                .service(
                    web::resource(DEFAULT_RPC_PATH)
                        .route(web::post().to(Server::_handle_http))
                        .route(web::method(actix_web::http::Method::CONNECT).to(|| {
                            actix_web::HttpResponse::Ok().body("CONNECT request is received")
                        })),
                ),
        );
    }

    #[cfg(any(all(feature = "tide", not(feature = "actix-web"),), feature = "docs"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(all(feature = "tide", not(feature = "actix-web"))))
    )]
    /// A conevience function that calls the corresponding http handling
    /// function depending on the enabled feature flag
    ///
    /// | feature flag | function name  |
    /// | ------------ |---|
    /// | `tide`| [`into_endpoint`](#method.into_endpoint) |
    /// | `actix-web` | [`scope_config`](#method.scope_config) |
    ///
    /// This is enabled
    /// if and only if **exactly one** of the the following feature flag is turned on
    /// - `serde_bincode`
    /// - `serde_json`
    /// - `serde_cbor`
    /// - `serde_rmp`
    ///
    /// # Example
    /// ```
    /// use toy_rpc::server::Server;
    /// use toy_rpc::macros::{export_impl, service};
    /// use async_std::sync::Arc;
    ///
    /// struct FooService { }
    ///
    /// #[export_impl]
    /// impl FooService {
    ///     // define some "exported" functions
    /// }
    ///
    /// #[async_std::main]
    /// async fn main() -> tide::Result<()> {
    ///     let addr = "127.0.0.1:8888";
    ///     let foo_service = Arc::new(FooService { });
    ///
    ///     let server = Server::builder()
    ///         .register("foo", service!(foo_service, FooService))
    ///         .build();
    ///     
    ///     let mut app = tide::new();
    ///
    ///     // If a network path were to be supplied,
    ///     // the network path must end with a slash "/"
    ///     // `handle_http` is a conenience function that calls `into_endpoint`
    ///     // with the `tide` feature turned on
    ///     app.at("/rpc/").nest(server.handle_http());
    ///
    ///     app.listen(addr).await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn handle_http(self) -> tide::Server<Server> {
        self.into_endpoint()
    }

    #[cfg(any(all(feature = "actix-web", not(feature = "tide"),), feature = "docs"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(all(feature = "actix-web", not(feature = "tide"))))
    )]
    /// A conevience function that calls the corresponding http handling
    /// function depending on the enabled feature flag
    ///
    /// | feature flag | function name  |
    /// | ------------ |---|
    /// | `tide`| [`into_endpoint`](#method.into_endpoint) |
    /// | `actix-web` | [`scope_config`](#method.scope_config) |
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
    /// use toy_rpc::Server;
    /// use toy_rpc::macros::{export_impl, service};
    /// use actix_web::{App, web};
    ///
    /// struct FooService { }
    ///
    /// #[export_impl]
    /// impl FooService {
    ///     // define some "exported" functions
    /// }
    ///
    /// #[actix::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let addr = "127.0.0.1:8888";
    ///     
    ///     let foo_service = Arc::new(FooService { });
    ///
    ///     let server = Server::builder()
    ///         .register("foo_service", service!(foo_servicem FooService))
    ///         .build();
    ///
    ///     let app_data = web::Data::new(server);
    ///
    ///     HttpServer::new(
    ///         move || {
    ///             App::new()
    ///                 .service(hello)
    ///                 .service(
    ///                     web::scope("/rpc/")
    ///                         .app_data(app_data.clone())
    ///                         .configure(Server::handle_http()) // use the convenience `handle_http`
    ///                 )
    ///         }
    ///     )
    ///     .bind(addr)?
    ///     .run()
    ///     .await
    /// }
    /// ```
    pub fn handle_http() -> fn(&mut web::ServiceConfig) {
        Self::scope_config
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
