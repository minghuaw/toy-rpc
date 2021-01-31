use super::{Server};

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
    // pub async fn serve_websocket<S, E>(&self, ws_stream: S) -> Result<(), Error> 
    // where
    //     S: Stream<Item = Result<WsMessage, E>> + Sink<WsMessage> + Send + Sync + Unpin,
    //     E: std::error::Error + 'static,
    // {
    //     let ws_stream = WebSocketConn::new(ws_stream);
    //     let codec = DefaultCodec::with_websocket(ws_stream);

    //     Self::_serve_codec(codec, self.services.clone()).await
    // }

    #[cfg(feature = "actix-web")]
    async fn _handle_http(
        state: web::Data<Server>,
        req_body: web::Bytes,
    ) -> Result<web::Bytes, actix_web::Error> {
        use futures::io::{BufReader, BufWriter};

        let input = req_body.to_vec();
        let mut output: Vec<u8> = Vec::new();

        let mut codec =
            super::DefaultCodec::with_reader_writer(BufReader::new(&*input), BufWriter::new(&mut output));
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
                    web::resource(super::DEFAULT_RPC_PATH)
                        .route(web::post().to(Server::_handle_http))
                        .route(web::method(actix_web::http::Method::CONNECT).to(|| {
                            actix_web::HttpResponse::Ok().body("CONNECT request is received")
                        })),
                ),
        );
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