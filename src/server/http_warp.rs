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

        /// The following impl block is controlled by feature flag. It is enabled
        /// if and only if **exactly one** of the the following feature flag is turned on
        /// - `serde_bincode`
        /// - `serde_json`
        /// - `serde_cbor`
        /// - `serde_rmp`
        impl Server {
            pub fn warp_websocket_handler(state: Arc<Self>, ws: warp::ws::Ws) -> impl warp::Reply {
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

            pub fn handler_path() -> &'static str {
                super::DEFAULT_RPC_PATH
            }
        }
    }
}
