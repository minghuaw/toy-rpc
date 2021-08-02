//! Integration with `axum` using WebSocket

use std::sync::{atomic::Ordering};

use axum::{AddExtensionLayer, extract::Extension, prelude::{RoutingDsl}, ws::WebSocket, routing::BoxRoute};
use http_body::Body;
use bytes::Bytes;

use crate::{
    server::Server,
    DEFAULT_RPC_PATH, 
    codec::DefaultCodec,
    pubsub::{AckModeNone, AckModeAuto}
};

macro_rules! impl_http_axum_for_ack_modes {
    ($($ack_mode:ty),*) => {
        $(
            impl Server<$ack_mode> {
                /// axum websocket handler
                pub async fn handle_axum_websocket(
                    ws: WebSocket,
                    state: Extension<Server<$ack_mode>>
                ) {
                    let codec = DefaultCodec::with_axum_websocket(ws);
                    let services = state.0.services.clone();
                    let client_id = state.0.client_counter.fetch_add(1, Ordering::Relaxed);
                    let pubsub_broker = state.0.pubsub_tx.clone();
                
                    let fut = Self::start_broker_reader_writer(codec, services, client_id, pubsub_broker);
                    fut.await.unwrap_or_else(|e| log::error!("{}", e));
                }

                /// Consumes `Server` and returns something that can nested in axum as a service
                pub fn into_boxed_route<B>(self) -> BoxRoute<B> 
                where 
                    B: Body<Data = Bytes> + Send + Sync + 'static,
                    B::Error: std::error::Error + Send + Sync,
                {
                    axum::route::<_, B>(
                        &format!("/{}", DEFAULT_RPC_PATH), 
                        axum::ws::ws(Self::handle_axum_websocket)
                    )
                    .layer(AddExtensionLayer::new(self))
                    .boxed()
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
                pub fn handle_http<B>(self) -> BoxRoute<B>
                where 
                    B: Body<Data = Bytes> + Send + Sync + 'static,
                    B::Error: std::error::Error + Send + Sync,
                {
                    self.into_boxed_route::<B>()
                }
            }
            
        )*
    };
}

impl_http_axum_for_ack_modes!(AckModeNone, AckModeAuto);