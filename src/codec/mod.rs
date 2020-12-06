use async_trait::async_trait;
use erased_serde as erased;
use futures::io::{AsyncBufRead, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use futures::channel::mpsc::{Receiver, Sender};
use futures::{SinkExt, StreamExt};
use std::marker::PhantomData;

use crate::error::Error;
use crate::message::{MessageId, Metadata, RequestHeader, ResponseHeader};

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

/// Default codec
pub struct Codec<R, W> {
    pub reader: R,
    pub writer: W,
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

impl<R, W> Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin,
{
    pub fn with_reader_writer(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

impl<T> Codec<BufReader<T>, BufWriter<T>>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + Clone,
{
    pub fn new(stream: T) -> Self {
        Self::with_reader_writer(BufReader::new(stream.clone()), BufWriter::new(stream))
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
        body: Box<(dyn erased::Serialize + Send + Sync)>,
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
        body: Box<(dyn erased::Serialize + Send + Sync)>,
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
        H: serde::Serialize + Metadata + Send + Sync + 'static;

    async fn write_body(
        &mut self,
        id: MessageId,
        body: Box<(dyn erased::Serialize + Send + Sync)>,
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
        body: Box<(dyn erased::Serialize + Send + Sync)>,
    ) -> Result<(), Error> {
        let id = header.get_id();

        log::trace!("Writing response id: {}", &id);

        self.write_header(header).await?;
        self.write_body(id, body).await?;

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
        body: Box<(dyn erased::Serialize + Send + Sync)>,
    ) -> Result<(), Error> {
        let id = header.get_id();

        log::trace!("Writing request id: {}", &id);

        self.write_header(header).await?;
        self.write_body(id, body).await?;

        Ok(())
    }
}

pub(crate) struct DeserializerOwned<D> {
    inner: D,
}

impl<D> DeserializerOwned<D> {
    pub fn new(inner: D) -> Self {
        Self { inner }
    }
}

pub struct ChannelCodec<C> {
    pub codec: PhantomData<C>,
    pub sender: Sender<(MessageId, Box<(dyn erased::Serialize + Send + Sync)>)>,
    pub recver: Receiver<Box<(dyn erased::Deserializer<'static> + Send + Sync)>>
}

#[async_trait]
impl<C: Marshal + Send + Sync> CodecWrite for ChannelCodec<C> {
    async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
    where
            H: serde::Serialize + Metadata + Send + Sync + 'static
    {
        let id = header.get_id();
        let mut sender = self.sender.clone();

        sender.send(
            (id, Box::new(header))
        ).await 
        .map_err(|e| Error::TransportError { msg: e.to_string() })
    }

    async fn write_body(&mut self, id: MessageId, body: Box<(dyn erased::Serialize + Send + Sync + 'static)>) -> Result<(), Error> {
        let id = id.clone();
        let mut sender = self.sender.clone();

        sender.send(
            (id, body)
        ).await 
        .map_err(|e| Error::TransportError { msg: e.to_string() })
    }
}

impl<C: Marshal> Marshal for ChannelCodec<C> {
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        C::marshal(val)
    }
}

impl<C: Unmarshal> Unmarshal for ChannelCodec<C> {
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        C::unmarshal(buf)
    }
}


