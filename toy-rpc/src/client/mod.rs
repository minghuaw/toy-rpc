//! There are two `Client` struct defined, one for `feature = "async_std_runtime"`
//! and the other for `feature = "tokio_runtime"`.
//!
//! When either `feature = "async_std_runtime"` or `feature = "http_tide"` is true
//! and none of `feature = "tokio_runtime"`, `feature = "http_actix_web"`, or
//! `feature = "http_warp"` is toggled, [`client::async_std::Client`](async_std/struct.Client.html)
//! will be re-exported as `client::Client`.
//!
//! When one of `feature = "tokio_runtime"`, `feature = "http_actix_web"`, or
//! `feature = "http_warp"` is true and none of `feature = "async_std_runtime"`
//! or `feature = "http_tide"` is [`client::tokio::Client`](tokio/struct.Client.html)
//! will be re-exported as `client::Client`.

use cfg_if::cfg_if;
use erased_serde as erased;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::Ordering;

use crate::codec::ClientCodec;
use crate::error::Error;
use crate::message::{AtomicMessageId, MessageId, RequestHeader, ResponseHeader};

cfg_if! {
    if #[cfg(any(
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
        )
    ))] {
        use crate::codec::DefaultCodec;
    }
}

#[cfg(any(feature = "async_std_runtime", feature = "http_tide"))]
pub mod async_std;

#[cfg(any(
    feature = "tokio_runtime",
    feature = "http_warp",
    feature = "http_actix_web",
))]
pub mod tokio;

cfg_if! {
    if #[cfg(any(
        feature = "async_std_runtime",
        feature = "http_tide"
    ))] {
        pub use crate::client::async_std::Client;
    } else if #[cfg(any(
        feature = "tokio_runtime",
        feature = "http_warp",
        feature = "http_actix_web",
    ))] {
        pub use crate::client::tokio::Client;
    }
}

/// Type state for creating `Client`
pub struct NotConnected {}
/// Type state for creating `Client`
pub struct Connected {}

type ResponseBody = Box<dyn erased::Deserializer<'static> + Send>;


