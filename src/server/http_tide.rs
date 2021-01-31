use tide_websockets as tide_ws;

use super::{Server};
use crate::{transport::ws::{WebSocketConn}};

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

        let mut app = tide::Server::with_state(self);
        // let mut app = tide::Server::new();
        app.at(super::DEFAULT_RPC_PATH)
            // .connect(|_| async move { Ok("CONNECT request is received") })
            .get(tide_ws::WebSocket::new(|req: tide::Request<Server>, ws_stream| async move {
                let ws_stream = WebSocketConn::new_without_sink(ws_stream);
                let codec = super::DefaultCodec::with_tide_websocket(ws_stream);
                let services = req.state().services.clone();

                let fut = Self::_serve_codec(codec, services);

                #[cfg(feature = "logging")]
                log::info!("Client disconnected from {}", _peer_addr);

                fut.await?;
                Ok(())
            }));

        app
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
}