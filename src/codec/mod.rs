use async_trait::async_trait;
use erased_serde as erased;
use futures::{
    io::{
        AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
        ReadHalf, WriteHalf,
    },
    stream::{SplitSink, SplitStream},
};
// use surf::http::content;
// use futures::channel::mpsc::{Receiver, Sender};
use futures::{Sink, Stream};
use std::marker::PhantomData;
use tide_websockets as tide_ws;
use tungstenite::Message as WsMessage;

use crate::error::Error;
use crate::message::{MessageId, Metadata, RequestHeader, ResponseHeader};
use crate::transport::{PayloadWrite, PayloadRead};
use crate::transport::ws::{
    CanSink, CannotSink, SinkHalf, StreamHalf, WebSocketConn,
};

#[cfg(all(
    feature = "serde_bincode",
    not(feature = "serde_json"),
    not(feature = "serde_cbor"),
    not(feature = "serde_rmp"),
))]
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(
        feature = "serde_bincode",
        not(feature = "serde_json"),
        not(feature = "serde_cbor"),
        not(feature = "serde_rmp"),
    )))
)]
pub mod bincode;

#[cfg(all(
    feature = "serde_cbor",
    not(feature = "serde_json"),
    not(feature = "serde_bincode"),
    not(feature = "serde_rmp"),
))]
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(
        feature = "serde_cbor",
        not(feature = "serde_json"),
        not(feature = "serde_bincode"),
        not(feature = "serde_rmp"),
    )))
)]
pub mod cbor;

#[cfg(all(
    feature = "serde_json",
    not(feature = "serde_bincode"),
    not(feature = "serde_cbor"),
    not(feature = "serde_rmp"),
))]
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(
        feature = "serde_json",
        not(feature = "serde_bincode"),
        not(feature = "serde_cbor"),
        not(feature = "serde_rmp"),
    )))
)]
pub mod json;

#[cfg(all(
    feature = "serde_rmp",
    not(feature = "serde_cbor"),
    not(feature = "serde_json"),
    not(feature = "serde_bincode"),
))]
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(
        feature = "serde_rmp",
        not(feature = "serde_cbor"),
        not(feature = "serde_json"),
        not(feature = "serde_bincode"),
    )))
)]
pub mod rmp;

/// type state for AsyncRead and AsyncWrite connections
pub(crate) struct ConnTypeReadWrite {}

/// type state for PayloadRead and PayloadWrite connections
pub(crate) struct ConnTypePayload {}

/// Default codec
pub struct Codec<R, W, C> {
    pub reader: R,
    pub writer: W,
    conn_type: PhantomData<C>,
}

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
pub use Codec as DefaultCodec;

impl<R, W> Codec<R, W, ConnTypeReadWrite>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin,
{
    pub fn with_reader_writer(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            conn_type: PhantomData,
        }
    }
}

impl<T> Codec<BufReader<ReadHalf<T>>, BufWriter<WriteHalf<T>>, ConnTypeReadWrite>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin,
{
    pub fn new(stream: T) -> Self {
        let (reader, writer) = stream.split();
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);

        Self::with_reader_writer(reader, writer)
    }
}

// websocket integration for async_tungstenite, tokio_tungstenite, and warp
impl<S, E>
    Codec<StreamHalf<SplitStream<S>>, SinkHalf<SplitSink<S, WsMessage>, CanSink>, ConnTypePayload>
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

// websocket integration with `tide`
impl
    Codec<
        StreamHalf<tide_ws::WebSocketConnection>,
        SinkHalf<tide_ws::WebSocketConnection, CannotSink>,
        ConnTypePayload,
    >
{
    pub fn with_tide_websocket(ws: WebSocketConn<tide_ws::WebSocketConnection, CannotSink>) -> Self {
        let (writer, reader) = WebSocketConn::<tide_ws::WebSocketConnection, CannotSink>::split(ws);

        Self {
            reader,
            writer,
            conn_type: PhantomData,
        }
    }
}

#[async_trait]
pub trait ServerCodec: Send + Sync {
    async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>>;
    async fn read_request_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>>;

    async fn write_response(
        &mut self,
        header: ResponseHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error>;
}

#[async_trait]
pub trait ClientCodec: Send + Sync {
    async fn read_response_header(&mut self) -> Option<Result<ResponseHeader, Error>>;
    async fn read_response_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>>;

    async fn write_request(
        &mut self,
        header: RequestHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error>;
}

#[async_trait]
pub trait CodecRead: Unmarshal {
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
    where
        H: serde::de::DeserializeOwned;

    async fn read_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>>;
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

pub trait Marshal {
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error>;
}

pub trait Unmarshal {
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error>;
}

#[async_trait]
impl<T> ServerCodec for T
where
    T: CodecRead + CodecWrite + Send + Sync,
{
    async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>> {
        self.read_header().await
    }

    async fn read_request_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
        self.read_body().await
    }

    async fn write_response(
        &mut self,
        header: ResponseHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error> {
        let id = header.get_id();

        log::trace!("Writing response id: {}", &id);

        self.write_header(header).await?;
        self.write_body(&id, body).await?;

        Ok(())
    }
}

#[async_trait]
impl<T> ClientCodec for T
where
    T: CodecRead + CodecWrite + Send + Sync,
{
    async fn read_response_header(&mut self) -> Option<Result<ResponseHeader, Error>> {
        self.read_header().await
    }

    async fn read_response_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
        self.read_body().await
    }

    async fn write_request(
        &mut self,
        header: RequestHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error> {
        let id = header.get_id();

        log::trace!("Writing request id: {}", &id);

        self.write_header(header).await?;
        self.write_body(&id, body).await?;

        Ok(())
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
