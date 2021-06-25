//! `SplittibleServerCodec` and `SplittableClientCodec` are defined in this module, and they are implemented
//! for the `DefaultCodec`
//! Default codec implementations are feature gated behind the following features
//! `serde_bincode`, `serde_json`, `serde_cbor`, `serde_rmp`.

use async_trait::async_trait;
use cfg_if::cfg_if;
use erased_serde as erased;
use futures::stream::{SplitSink, SplitStream};
use futures::{Sink, Stream};
use std::marker::PhantomData;
use tungstenite::Message as WsMessage;

use crate::error::Error;
use crate::message::{MessageId, Metadata, RequestHeader, ResponseHeader};
use crate::transport::ws::{CanSink, SinkHalf, StreamHalf, WebSocketConn};
// use crate::util::GracefulShutdown;

pub mod split;

pub type RequestDeserializer = Box<dyn erased::Deserializer<'static> + Send + 'static>;

cfg_if! {
    if #[cfg(feature = "http_tide")] {
        use tide_websockets as tide_ws;
        use crate::transport::ws::CannotSink;
    }
}

cfg_if! {
    if #[cfg(feature = "async-std")] {
        #[cfg_attr(
            feature = "docs",
            doc(cfg(all(feature = "async-std", not(feature = "tokio"))))
        )]
        mod async_std;
    } else if #[cfg(feature = "tokio")] {
        #[cfg_attr(
            feature = "docs",
            doc(cfg(all(feature = "tokio", not(feature = "async-std"))))
        )]
        mod tokio;
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "docs",
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
        pub use Codec as DefaultCodec;
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "async-std",
        feature = "tokio",
        feature = "docs",
    ))] {
        #[cfg(all(
            feature = "serde_bincode",
            not(feature = "serde_json"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ))]
        #[cfg_attr(
            doc,
            doc(cfg(all(
                feature = "serde_bincode",
                not(feature = "serde_json"),
                not(feature = "serde_cbor"),
                not(feature = "serde_rmp"),
            )))
        )]
        pub mod bincode;

        #[cfg(all(
            feature = "serde_json",
            not(feature = "serde_bincode"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ))]
        #[cfg_attr(
            doc,
            doc(cfg(all(
                feature = "serde_json",
                not(feature = "serde_bincode"),
                not(feature = "serde_cbor"),
                not(feature = "serde_rmp"),
            )))
        )]
        pub mod json;

        #[cfg(all(
            feature = "serde_cbor",
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
            not(feature = "serde_rmp"),
        ))]
        #[cfg_attr(
            doc,
            doc(cfg(all(
                feature = "serde_cbor",
                not(feature = "serde_json"),
                not(feature = "serde_bincode"),
                not(feature = "serde_rmp"),
            )))
        )]
        pub mod cbor;

        #[cfg(all(
            feature = "serde_rmp",
            not(feature = "serde_cbor"),
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
        ))]
        #[cfg_attr(
            doc,
            doc(cfg(all(
                feature = "serde_rmp",
                not(feature = "serde_cbor"),
                not(feature = "serde_json"),
                not(feature = "serde_bincode"),
            )))
        )]
        pub mod rmp;
    }
}

#[cfg(any(feature = "async-std", feature = "tokio"))]
/// type state for AsyncRead and AsyncWrite connections (ie. raw TCP)
pub struct ConnTypeReadWrite {}

/// type state for PayloadRead and PayloadWrite connections (ie. WebSocket)
pub struct ConnTypePayload {}

/// Default codec. `Codec` is re-exported as `DefaultCodec` when one of these feature
/// flags is toggled (`serde_bincode`, `serde_json`, `serde_cbor`, `serde_rmp`")
#[cfg_attr(
    not(all(
        any( // there has to be a runtime
            feature = "async-std", 
            feature = "tokio",
        ),
        any( // there has to be a codec
            feature = "serde_bincode",
            feature = "serde_json",
            feature = "serde_cbor",
            feature = "serde_rmp",
        )
    )),
    allow(dead_code)
)]
pub struct Codec<R, W, C> {
    reader: R,
    writer: W,
    conn_type: PhantomData<C>,
}

/// WebSocket integration for async_tungstenite, tokio_tungstenite
impl<S, E>
    Codec<
        StreamHalf<SplitStream<S>, CanSink>,
        SinkHalf<SplitSink<S, WsMessage>, CanSink>,
        ConnTypePayload,
    >
where
    S: Stream<Item = Result<WsMessage, E>> + Sink<WsMessage> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    /// Creates a `Codec` with a WebSocket connection.
    ///
    /// This works with the `WebSocketConn` types implemented in `async_tungstenite` and `tokio_tungstenite`
    pub fn with_websocket(ws: WebSocketConn<S, CanSink>) -> Self {
        let (writer, reader) = WebSocketConn::<S, CanSink>::split(ws);

        Self {
            reader,
            writer,
            conn_type: PhantomData,
        }
    }
}

#[cfg(all(feature = "http_tide"))]
/// WebSocket integration with `tide`
impl
    Codec<
        StreamHalf<tide_ws::WebSocketConnection, CannotSink>,
        SinkHalf<tide_ws::WebSocketConnection, CannotSink>,
        ConnTypePayload,
    >
{
    /// Creates a `Codec` with a WebSocket connection implemented in the `tide` HTTP server.
    #[cfg_attr(feature = "docs", doc(cfg(feature = "http_tide")))]
    pub fn with_tide_websocket(
        ws: WebSocketConn<tide_ws::WebSocketConnection, CannotSink>,
    ) -> Self {
        let (writer, reader) = WebSocketConn::<tide_ws::WebSocketConnection, CannotSink>::split(ws);

        Self {
            reader,
            writer,
            conn_type: PhantomData,
        }
    }
}

#[cfg(all(feature = "http_warp"))]
// warp websocket
impl<S, E> Codec<SplitStream<S>, SplitSink<S, warp::ws::Message>, ConnTypePayload>
where
    S: Stream<Item = Result<warp::ws::Message, E>> + Sink<warp::ws::Message>,
    E: std::error::Error,
{
    /// Creates a `Codec` with a WebSocket connection implemented in the `warp` HTTP server.
    #[cfg_attr(feature = "docs", doc(cfg(feature = "http_warp")))]
    pub fn with_warp_websocket(ws: S) -> Self {
        use futures::StreamExt;
        let (writer, reader) = ws.split();

        Self {
            reader,
            writer,
            conn_type: PhantomData,
        }
    }
}

/// A codec that can read the header and body of a message
#[async_trait]
pub trait CodecRead: Unmarshal {
    /// Reads the header of the message.
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
    where
        H: serde::de::DeserializeOwned;

    /// Reads the body of the message
    async fn read_body(&mut self) -> Option<Result<RequestDeserializer, Error>>;
}

/// A codec that can write the header and body of a message
#[async_trait]
pub trait CodecWrite: Marshal {
    /// Writes the header of the message
    async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
    where
        H: serde::Serialize + Metadata + Send;

    /// Writes the body of the message
    async fn write_body(
        &mut self,
        id: &MessageId,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error>;
}

cfg_if! {
    if #[cfg(all(
        any(
            feature = "async-std",
            feature = "tokio",
        ),
        any(
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
        )
    ))] {
        /// A wrapper for erased serde deserializers to allow transfer of ownership
        pub(crate) struct DeserializerOwned<D> {
            inner: D,
        }

        impl<D> DeserializerOwned<D> {
            pub fn new(inner: D) -> Self {
                Self { inner }
            }
        }
    }
}

/// This trait should be implemented by serializer (Codec) to serialize messages into bytes
pub trait Marshal {
    /// Marshals/serializes an object into `Vec<u8>`
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error>;
}

/// This trait should be implemented by deserializer (Codec) to deserialize messages from bytes
pub trait Unmarshal {
    /// Unmarshals an object from bytes
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error>;
}

/// This trait should be implemented by a codec to allow creating a `erased_serde::Deserilizer` from
/// bytes
pub trait EraseDeserializer {
    /// Creates an `erased_serde::Deserializer` from bytes
    fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send>;
}
