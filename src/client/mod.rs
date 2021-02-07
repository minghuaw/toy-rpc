use erased_serde as erased;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::codec::{ClientCodec};
use crate::error::Error;
use crate::transport::ws::WebSocketConn;
use crate::message::{AtomicMessageId, MessageId, RequestHeader, ResponseHeader};

use crate::server::DEFAULT_RPC_PATH;

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
use crate::codec::DefaultCodec;

#[cfg(any(
    all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
    all(feature = "http_tide", not(feature="http_actix_web"), not(feature = "http_warp"))
))]
mod async_std;

#[cfg(any(
    all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
    all(feature = "http_tide", not(feature="http_actix_web"), not(feature = "http_warp"))
))]
use crate::client::async_std::{Mutex, oneshot};
// use ::async_std::sync::Mutex;

// #[cfg(any(
//     all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
//     all(feature = "tide", not(feature="actix_web"), not(feature = "warp"))
// ))]
// use futures::channel::oneshot;

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(
        any(feature = "http_warp", feature = "http_actix_web"),
        not(feature = "http_tide")
    )
))]
mod tokio;


#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(
        any(feature = "http_warp", feature = "http_actix_web"),
        not(feature = "http_tide")
    )
))]
use crate::client::tokio::{oneshot, Mutex};

/// Type state for creating `Client`
pub struct NotConnected {}
/// Type state for creating `Client`
pub struct Connected {}

type Codec = Arc<Mutex<Box<dyn ClientCodec>>>;
type ResponseBody = Box<dyn erased::Deserializer<'static> + Send>;
type ResponseMap = HashMap<u16, oneshot::Sender<Result<ResponseBody, ResponseBody>>>;

// RPC Client
pub struct Client<Mode> {
    count: AtomicMessageId,
    inner_codec: Codec,
    pending: Arc<Mutex<ResponseMap>>,

    mode: PhantomData<Mode>,
}
