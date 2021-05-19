//! `ServerCodec` and `ClientCodec` are defined in this module, and they are implemented
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

pub(crate) type RequestDeserializer = Box<dyn erased::Deserializer<'static> + Send + 'static>;

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
            doc(any(
                all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
                all(feature = "http_tide", not(feature="http_actix_web"), not(feature = "http_warp"))
            ))
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
        feature = "http_tide",
        feature = "http_warp",
        feature = "http_actix_web",
        feature = "docs",
    ))] {
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

#[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
/// type state for AsyncRead and AsyncWrite connections (ie. raw TCP)
pub(crate) struct ConnTypeReadWrite {}

/// type state for PayloadRead and PayloadWrite connections (ie. WebSocket)
pub(crate) struct ConnTypePayload {}

/// Default codec. `Codec` is re-exported as `DefaultCodec` when one of these feature
/// flags is toggled (`serde_bincode`, `serde_json`, `serde_cbor`, `serde_rmp`")
pub struct Codec<R, W, C> {
    pub reader: R,
    pub writer: W,
    conn_type: PhantomData<C>,
}

// websocket integration for async_tungstenite, tokio_tungstenite
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
// websocket integration with `tide`
impl
    Codec<
        StreamHalf<tide_ws::WebSocketConnection, CannotSink>,
        SinkHalf<tide_ws::WebSocketConnection, CannotSink>,
        ConnTypePayload,
    >
{
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

// #[async_trait]
// pub trait ServerCodec: Send + Sync {
//     async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>>;
//     async fn read_request_body(&mut self) -> Option<Result<RequestDeserializer, Error>>;

//     // (Probably) don't need to worry about header/body interleaving
//     // because rust guarantees only one mutable reference at a time
//     async fn write_response(
//         &mut self,
//         header: ResponseHeader,
//         body: &(dyn erased::Serialize + Send + Sync),
//     ) -> Result<(), Error>;
// }

// #[async_trait]
// pub trait ClientCodec: GracefulShutdown + Send + Sync {
//     async fn read_response_header(&mut self) -> Option<Result<ResponseHeader, Error>>;
//     async fn read_response_body(&mut self) -> Option<Result<RequestDeserializer, Error>>;

//     // (Probably) don't need to worry about header/body interleaving
//     // because rust guarantees only one mutable reference at a time
//     async fn write_request(
//         &mut self,
//         header: RequestHeader,
//         body: &(dyn erased::Serialize + Send + Sync),
//     ) -> Result<(), Error>;
// }

#[async_trait]
pub trait CodecRead: Unmarshal {
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
    where
        H: serde::de::DeserializeOwned;

    async fn read_body(&mut self) -> Option<Result<RequestDeserializer, Error>>;
}

#[async_trait]
pub trait CodecWrite: Marshal {
    async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
    where
        H: serde::Serialize + Metadata + Send;

    async fn write_body(
        &mut self,
        id: &MessageId,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error>;
}

cfg_if! {
    if #[cfg(all(
        any(feature = "async_std_runtime", feature = "tokio_runtime"),
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
                feature = "serde_rmp",
                not(feature = "serde_cbor"),
                not(feature = "serde_json"),
                not(feature = "serde_bincode"),
            )
        )
    ))] {
        use crate::transport::frame::{Frame, PayloadType, FrameRead, FrameWrite};

        #[async_trait]
        // Trati implementation for binary protocols
        impl<R, W> CodecRead for Codec<R, W, ConnTypeReadWrite>
        where
            R: FrameRead + Send + Sync + Unpin,
            W: FrameWrite + Send + Sync + Unpin,
        {
            async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
            where
                H: serde::de::DeserializeOwned,
            {
                let reader = &mut self.reader;

                Some(
                    reader
                        .read_frame()
                        .await?
                        .and_then(|frame| Self::unmarshal(&frame.payload)),
                )
            }

            async fn read_body(
                &mut self,
            ) -> Option<Result<RequestDeserializer, Error>> {
                let reader = &mut self.reader;

                match reader.read_frame().await? {
                    Ok(frame) => {
                        let de = Self::from_bytes(frame.payload);
                        Some(Ok(de))
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
        }

        #[async_trait]
        // trait implementation for binary protocols
        impl<R, W> CodecWrite for Codec<R, W, ConnTypeReadWrite>
        where
            R: FrameRead + Send + Sync + Unpin,
            W: FrameWrite + Send + Sync + Unpin,
        {
            async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
            where
                H: serde::Serialize + Metadata + Send,
            {
                let writer = &mut self.writer;

                let id = header.get_id();
                let buf = Self::marshal(&header)?;
                let frame = Frame::new(id, 0, PayloadType::Header, buf);

                writer.write_frame(frame).await
            }

            async fn write_body(
                &mut self,
                id: &MessageId,
                body: &(dyn erased::Serialize + Send + Sync),
            ) -> Result<(), Error> {
                let writer = &mut self.writer;
                let buf = Self::marshal(&body)?;
                let frame = Frame::new(id.to_owned(), 1, PayloadType::Data, buf.to_owned());
                writer.write_frame(frame).await
            }
        }
    }
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
        use crate::transport::{PayloadRead, PayloadWrite};

        #[async_trait]
        impl<R, W> CodecRead for Codec<R, W, ConnTypePayload>
        where
            R: PayloadRead + Send,
            W: Send,
        {
            async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
            where
                H: serde::de::DeserializeOwned,
            {
                let reader = &mut self.reader;

                Some(
                    reader
                        .read_payload()
                        .await?
                        .and_then(|payload| Self::unmarshal(&payload)),
                )
            }

            async fn read_body(
                &mut self,
            ) -> Option<Result<RequestDeserializer, Error>> {
                let reader = &mut self.reader;

                match reader.read_payload().await? {
                    Ok(payload) => {
                        let de = Self::from_bytes(payload);
                        Some(Ok(de))
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
        }

        #[async_trait]
        impl<R, W> CodecWrite for Codec<R, W, ConnTypePayload>
        where
            R: Send,
            W: PayloadWrite + Send,
        {
            async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
            where
                H: serde::Serialize + Metadata + Send,
            {
                let writer = &mut self.writer;
                let buf = Self::marshal(&header)?;
                writer.write_payload(buf).await
            }

            async fn write_body(
                &mut self,
                _: &MessageId,
                body: &(dyn erased::Serialize + Send + Sync),
            ) -> Result<(), Error> {
                let writer = &mut self.writer;
                let buf = Self::marshal(&body)?;
                writer.write_payload(buf).await
            }
        }

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

pub trait Marshal {
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error>;
}

pub trait Unmarshal {
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error>;
}

// #[async_trait]
// impl<T> ServerCodec for T
// where
//     T: CodecRead + CodecWrite + Send + Sync,
// {
//     async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>> {
//         self.read_header().await
//     }

//     async fn read_request_body(
//         &mut self,
//     ) -> Option<Result<RequestDeserializer, Error>> {
//         self.read_body().await
//     }

//     async fn write_response(
//         &mut self,
//         header: ResponseHeader,
//         body: &(dyn erased::Serialize + Send + Sync),
//     ) -> Result<(), Error> {
//         let id = header.get_id();

//         log::trace!("Sending response id: {}", &id);

//         self.write_header(header).await?;
//         self.write_body(&id, body).await?;

//         Ok(())
//     }
// }

// #[async_trait]
// impl<T> ClientCodec for T
// where
//     T: CodecRead + CodecWrite + GracefulShutdown + Send + Sync,
// {
//     async fn read_response_header(&mut self) -> Option<Result<ResponseHeader, Error>> {
//         self.read_header().await
//     }

//     async fn read_response_body(&mut self) -> Option<Result<RequestDeserializer, Error>> {
//         self.read_body().await
//     }

//     async fn write_request(
//         &mut self,
//         header: RequestHeader,
//         body: &(dyn erased::Serialize + Send + Sync),
//     ) -> Result<(), Error> {
//         let id = header.get_id();

//         log::trace!("Sending request id: {}", &id);

//         self.write_header(header).await?;
//         self.write_body(&id, body).await?;

//         Ok(())
//     }
// }

pub trait EraseDeserializer {
    fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send>;
}
