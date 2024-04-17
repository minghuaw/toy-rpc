//! Integration with `axum` using WebSocket

use std::sync::atomic::Ordering;

use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    routing::get,
    response::IntoResponse,
    Router,
    Extension,
};

use crate::{
    codec::DefaultCodec,
    pubsub::{AckModeAuto, AckModeNone},
    server::Server,
    DEFAULT_RPC_PATH,
};

macro_rules! impl_http_axum_for_ack_modes {
    ($($ack_mode:ty),*) => {
        $(
            impl Server<$ack_mode> {
                /// axum websocket handler
                pub async fn handle_axum_websocket(
                    ws: WebSocket,
                    state: Server<$ack_mode>
                ) {
                    let codec = DefaultCodec::with_axum_websocket(ws);
                    let services = state.services.clone();
                    let client_id = state.client_counter.fetch_add(1, Ordering::Relaxed);
                    let pubsub_broker = state.pubsub_tx.clone();

                    let fut = Self::start_broker_reader_writer(codec, services, client_id, pubsub_broker);
                    fut.await.unwrap_or_else(|e| log::error!("{}", e));
                }

                async fn on_websocket_upgrade(
                    ws: WebSocketUpgrade,
                    Extension(state): Extension<Server<$ack_mode>>,
                ) -> impl IntoResponse {
                    ws.on_upgrade(|websocket| Self::handle_axum_websocket(websocket, state))
                }

                /// Consumes `Server` and returns something that can nested in axum as a service
                pub fn into_route(self) -> Router
                {
                    Router::new()
                        .route(
                            &format!("/{}", DEFAULT_RPC_PATH),
                            get(Self::on_websocket_upgrade)
                        )
                        .layer(Extension(self.clone()))
                }

                #[cfg(any(
                    all(
                        feature = "http_axum",
                        not(feature = "http_actix_web"),
                        not(feature = "http_tide"),
                        not(feature = "http_warp")
                    ),
                    feature = "docs"
                ))]
                #[cfg_attr(
                    feature = "docs",
                    doc(cfg(all(
                        feature = "axum",
                        not(feature = "http_actix_web"),
                        not(feature = "http_tide"),
                        not(feature = "http_warp")
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
                /// | `http_axum` | [`into_route`](#method.into_route) |
                pub fn handle_http(self) -> Router
                {
                    self.into_route()
                }
            }

        )*
    };
}

impl_http_axum_for_ack_modes!(AckModeNone);

#[cfg(not(feature = "docs"))]
impl_http_axum_for_ack_modes!(AckModeAuto);
