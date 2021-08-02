//! This module implements integration with `tide`.
use cfg_if::cfg_if;
use tide_websockets as tide_ws;

use crate::server::Server;
use crate::transport::ws::WebSocketConn;

cfg_if! {
    if #[cfg(any(
        any(feature = "docs", doc),
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
    ))] {
        use std::sync::atomic::Ordering;

        use crate::codec::DefaultCodec;
        use crate::DEFAULT_RPC_PATH;
        use crate::pubsub::{AckModeNone, AckModeAuto};

        macro_rules! impl_tide_integarion_for_ack_modes {
            ($($ack_mode:ty),*) => {
                $(
                    /// The following impl block is controlled by feature flag. It is enabled
                    /// if and only if **exactly one** of the the following feature flag is turned on
                    /// - `serde_bincode`
                    /// - `serde_json`
                    /// - `serde_cbor`
                    /// - `serde_rmp`
                    impl Server<$ack_mode> {
                        #[cfg(any(feature = "http_tide", feature = "docs"))]
                        #[cfg_attr(feature = "docs", doc(cfg(feature = "http_tide")))]
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
                        ///
                        /// ```
                        /// let foo_service = Arc::new(FooService { });
                        /// let server = Server::builder()
                        ///     .register(foo_service)
                        ///     .build();
                        /// let mut app = tide::new();
                        ///
                        /// // If a network path were to be supplied,
                        /// // the network path must end with a slash "/"
                        /// app.at("/rpc/").nest(server.into_endpoint());
                        /// app.listen("127.0.0.1:8080").await?;
                        /// ```
                        ///
                        pub fn into_endpoint(self) -> tide::Server<Server<$ack_mode>> {
                            let mut app = tide::Server::with_state(self);
                            // let mut app = tide::Server::new();
                            app.at(DEFAULT_RPC_PATH)
                                // .connect(|_| async move { Ok("CONNECT request is received") })
                                .get(tide_ws::WebSocket::new(
                                    |req: tide::Request<Server<$ack_mode>>, ws_stream| async move {
                                        let ws_stream = WebSocketConn::new_without_sink(ws_stream);
                                        let codec = DefaultCodec::with_tide_websocket(ws_stream);
                                        let services = req.state().services.clone();
                                        let client_id = req.state().client_counter.fetch_add(1, Ordering::Relaxed);
                                        let pubsub_broker = req.state().pubsub_tx.clone();
            
                                        let fut = Self::start_broker_reader_writer(codec, services, client_id, pubsub_broker);
                                        log::trace!("Client disconnected.");
                                        fut.await?;
                                        Ok(())
                                    },
                                ));
            
                            app
                        }
            
                        #[cfg(any(
                            all(
                                feature = "http_tide",
                                not(feature = "http_actix_web"),
                                not(feature = "http_warp"),
                                not(feature = "http_axum"),
                            ),
                            feature = "docs"
                        ))]
                        #[cfg_attr(
                            feature = "docs",
                            doc(cfg(all(
                                feature = "http_tide",
                                not(feature = "http_actix_web"),
                                not(feature = "http_warp"),
                                not(feature = "http_axum"),
                            )))
                        )]
                        /// A conevience function that calls the corresponding http handling
                        /// function depending on the enabled feature flag
                        ///
                        /// | feature flag | function name  |
                        /// | ------------ |---|
                        /// | `http_tide`| [`into_endpoint`](#method.into_endpoint) |
                        /// | `http_actix_web` | [`scope_config`](#method.scope_config) |
                        /// | `http_warp` | [`into_boxed_filter`](#method.into_boxed_filter) |
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
                        /// ```
                        /// let foo_service = Arc::new(FooService { });
                        /// let server = Server::builder()
                        ///     .register(foo_service)
                        ///     .build();
                        /// let mut app = tide::new();
                        ///
                        /// // If a network path were to be supplied,
                        /// // the network path must end with a slash "/"
                        /// app.at("/rpc/").nest(server.handle_http());
                        /// app.listen("127.0.0.1:8080").await?;
                        /// ```
                        pub fn handle_http(self) -> tide::Server<Server<$ack_mode>> {
                            self.into_endpoint()
                        }
                    }
                )*
            }
        }
        
        impl_tide_integarion_for_ack_modes!(AckModeNone, AckModeAuto);
    }
}
