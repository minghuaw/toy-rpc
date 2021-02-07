use erased_serde as erased;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use cfg_if::cfg_if;

use crate::codec::{ClientCodec};
use crate::error::Error;
use crate::message::{AtomicMessageId, MessageId, RequestHeader, ResponseHeader};

cfg_if!{
    if #[cfg(feature = "serde_bincode")] {
        use crate::codec::DefaultCodec;
    } else if #[cfg(feature = "serde_json")] {
        use crate::codec::DefaultCodec;
    } else if #[cfg(feature = "serde_cbor")] {
        use crate::codec::DefaultCodec;
    } else if #[cfg(feature = "serde_rmp")] {
        use crate::codec::DefaultCodec;
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "async_std_runtime", 
        feature = "http_tide"
    ))] {
        #[cfg_attr(
            feature = "docs",
            doc(any(
                all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
                all(feature = "http_tide", not(feature="http_actix_web"), not(feature = "http_warp"))
            ))
        )]
        mod async_std;
        use crate::client::async_std::{Mutex, oneshot};
    } else if #[cfg(any(
        feature = "tokio_runtime", 
        feature = "http_warp", 
        feature = "http_actix_web"
    ))] {
        #[cfg_attr(
            feature = "docs",
            doc(any(
                all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
                all(
                    any(feature = "http_warp", feature = "http_actix_web"),
                    not(feature = "http_tide")
                )
            ))
        )]
        mod tokio;
        use crate::client::tokio::{oneshot, Mutex};
    }
}

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
