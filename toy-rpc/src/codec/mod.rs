//! `SplittibleCodec` is defined in this module, and they are implemented
//! for the `DefaultCodec`
//! Default codec implementations are feature gated behind the following features
//! `serde_bincode`, `serde_json`, `serde_cbor`, `serde_rmp`.

use async_trait::async_trait;
use cfg_if::cfg_if;
use erased_serde as erased;
use std::marker::PhantomData;

use crate::error::{CodecError, Error, ParseError};
use crate::message::{MessageId, Metadata};
use crate::protocol::InboundBody;

pub mod split;

cfg_if! {
    if #[cfg(feature = "http_tide")] {
        use tide_websockets as tide_ws;
        use crate::transport::ws::CannotSink;
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "async_std_runtime",
        feature = "http_tide"
    ))] {
        #[cfg_attr(
            feature = "docs",
            doc(cfg(any(
                all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
                all(feature = "http_tide", not(feature="http_actix_web"), not(feature = "http_warp"))
            )))
        )]
        mod async_std;
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
        feature = "async_std_runtime",
        feature = "tokio_runtime",
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

/// Type state for AsyncRead and AsyncWrite connections (ie. raw TCP)
#[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
pub(crate) struct ConnTypeReadWrite {}

/// Type state for PayloadRead and PayloadWrite connections (ie. WebSocket)
#[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
pub(crate) struct ConnTypePayload {}

/// Reserved type state for Reader/Writer for Codec
pub struct Reserved {}

/// Default codec. `Codec` is re-exported as `DefaultCodec` when one of these feature
/// flags is toggled (`serde_bincode`, `serde_json`, `serde_cbor`, `serde_rmp`")
#[cfg_attr(
    not(all(
        any( // there has to be a runtime
            feature = "async_std_runtime", 
            feature = "tokio_runtime",
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

cfg_if! {
    if #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))] {
        use futures::stream::{SplitSink, SplitStream};
        use futures::{Sink, Stream};
        use tungstenite::Message;

        use crate::transport::ws::{CanSink, SinkHalf, StreamHalf, WebSocketConn};

        /// WebSocket integration for async_tungstenite, tokio_tungstenite
        impl<S, E>
            Codec<
                StreamHalf<SplitStream<S>, CanSink>,
                SinkHalf<SplitSink<S, Message>, CanSink>,
                ConnTypePayload,
            >
        where
            S: Stream<Item = Result<Message, E>> + Sink<Message> + Send + Sync + Unpin,
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
impl<S, E>
    Codec<
        StreamHalf<SplitStream<S>, CanSink>,
        SinkHalf<SplitSink<S, warp::ws::Message>, CanSink>,
        ConnTypePayload,
    >
where
    S: Stream<Item = Result<warp::ws::Message, E>> + Sink<warp::ws::Message>,
    E: std::error::Error,
{
    /// Creates a `Codec` with a WebSocket connection implemented in the `warp` HTTP server.
    #[cfg_attr(feature = "docs", doc(cfg(feature = "http_warp")))]
    pub fn with_warp_websocket(ws: S) -> Self {
        use futures::StreamExt;
        let (writer, reader) = ws.split();
        let writer = SinkHalf::<_, CanSink> {
            inner: writer,
            can_sink: PhantomData,
        };
        let reader = StreamHalf::<_, CanSink> {
            inner: reader,
            can_sink: PhantomData,
        };

        Self {
            reader,
            writer,
            conn_type: PhantomData,
        }
    }
}

#[cfg(all(feature = "http_axum"))]
impl<S, E>
    Codec<
        StreamHalf<SplitStream<S>, CanSink>,
        SinkHalf<SplitSink<S, axum::extract::ws::Message>, CanSink>,
        ConnTypePayload,
    >
where
    S: Stream<Item = Result<axum::extract::ws::Message, E>> + Sink<axum::extract::ws::Message>,
    // E: std::error::Error
{
    /// Creates a codec with WebSocket wrapper type provided by `axum` HTTP framework
    #[cfg_attr(feature = "docs", doc(cfg(feature = "http_axum")))]
    pub fn with_axum_websocket(ws: S) -> Self {
        use futures::StreamExt;
        let (writer, reader) = ws.split();
        let writer = SinkHalf::<_, CanSink> {
            inner: writer,
            can_sink: PhantomData,
        };
        let reader = StreamHalf::<_, CanSink> {
            inner: reader,
            can_sink: PhantomData,
        };

        Self {
            reader,
            writer,
            conn_type: PhantomData,
        }
    }
}

/// A codec that can read the header and body of a message
#[async_trait]
pub trait CodecRead: Send + Unmarshal + EraseDeserializer {
    /// Reads the header of the message.
    async fn read_header<H>(&mut self) -> Option<Result<H, CodecError>>
    where
        H: serde::de::DeserializeOwned,
    {
        Some(
            self.read_bytes()
                .await?
                .and_then(|payload| Self::unmarshal(&payload).map_err(Into::into)),
        )
    }

    /// Reads the body of the message
    async fn read_body(&mut self) -> Option<Result<Box<InboundBody>, CodecError>> {
        match self.read_bytes().await? {
            Ok(payload) => {
                let de = Self::from_bytes(payload);
                Some(Ok(de))
            }
            Err(e) => return Some(Err(e)),
        }
    }

    /// Reads the frame body as raw bytes
    async fn read_bytes(&mut self) -> Option<Result<Vec<u8>, CodecError>>;
}

/// A codec that can write the header and body of a message
#[async_trait]
pub trait CodecWrite: Send + Marshal {
    /// Writes the header of the message
    async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
    where
        H: serde::Serialize + Metadata + Send;

    /// Writes the body of the message
    async fn write_body(
        &mut self,
        id: MessageId,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error>;

    /// Writes body as raw bytes
    async fn write_body_bytes(&mut self, id: MessageId, bytes: &[u8]) -> Result<(), Error>;
}

cfg_if! {
    if #[cfg(all(
        any(
            feature = "async_std_runtime",
            feature = "tokio_runtime",
            feature = "http_tide",
            feature = "http_warp",
            feature = "http_actix_web"
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
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, ParseError>;
}

/// This trait should be implemented by deserializer (Codec) to deserialize messages from bytes
pub trait Unmarshal {
    /// Unmarshals an object from bytes
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, ParseError>;
}

/// This trait should be implemented by a codec to allow creating a `erased_serde::Deserilizer` from
/// bytes
pub trait EraseDeserializer {
    /// Creates an `erased_serde::Deserializer` from bytes
    fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send>;
}
