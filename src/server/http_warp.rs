/// This module implements integration with `warp`.

use cfg_if::cfg_if;
use std::sync::Arc;

use super::Server;

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
        use crate::codec::DefaultCodec;
        use warp::{Filter, Reply, filters::BoxedFilter};

        /// The following impl block is controlled by feature flag. It is enabled
        /// if and only if **exactly one** of the the following feature flag is turned on
        /// - `serde_bincode`
        /// - `serde_json`
        /// - `serde_cbor`
        /// - `serde_rmp`
        impl Server {
            /// WebSocket handler for integration with `warp`
            fn warp_websocket_handler(state: Arc<Self>, ws: warp::ws::Ws) -> impl warp::Reply {
                ws.on_upgrade(|websocket| async move {
                    let codec = DefaultCodec::with_warp_websocket(websocket);
                    let services = state.services.clone();

                    let fut = Self::_serve_codec(codec, services);
                    match fut.await {
                        Ok(_) => (),
                        Err(e) => log::debug!("{}", e),
                    };
                })
            }

            /// Returns the `DEFAULT_RPC_PATH`
            fn handler_path() -> &'static str {
                super::DEFAULT_RPC_PATH
            }

            /// Consumes `Server` and returns a `warp::filters::BoxedFilter`
            /// which can be chained with `warp` filters
            pub fn boxed_filter(self) -> BoxedFilter<(impl Reply,)> {
                let state = Arc::new(self);
                let state = warp::any().map(move || state.clone());

                let rpc_route = warp::path(Server::handler_path())
                    .and(state)
                    .and(warp::ws())
                    .map(Server::warp_websocket_handler)
                    .boxed();

                rpc_route
            }

            #[cfg(any(
                all(
                    feature = "http_warp",
                    not(feature = "http_actix_web"),
                    not(feature = "http_tide"),
                ),
                feature = "docs"
            ))]
            #[cfg_attr(
                feature = "docs",
                doc(cfg(all(
                    feature = "http_warp",
                    not(feature = "http_actix_web"),
                    not(feature = "http_tide"),
                )))
            )]
            /// A conevience function that calls the corresponding http handling
            /// function depending on the enabled feature flag
            ///
            /// | feature flag | function name  |
            /// | ------------ |---|
            /// | `http_tide`| [`into_endpoint`](#method.into_endpoint) |
            /// | `http_actix_web` | [`scope_config`](#method.scope_config) |
            /// | `http_warp` | [`boxed_filter`](#method.boxed_filter) |
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            pub fn handle_http(self) -> BoxedFilter<(impl Reply,)> {
                self.boxed_filter()
            }
        }
    }
}
