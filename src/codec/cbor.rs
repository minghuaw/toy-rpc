//! Impplementation of `CodecRead`, `CodecWrite`, `Marshal` and `Unmarshal` traits with `serde_cbor`

use async_trait::async_trait;
use erased_serde as erased;
use futures::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};
use futures::{SinkExt, StreamExt};
use serde::de::Visitor;
use std::io::Cursor; // serde doesn't support AsyncRead

use super::{Codec, CodecRead, CodecWrite, DeserializerOwned, EraseDeserializer, Marshal, Unmarshal};
use crate::error::Error;
use crate::macros::impl_inner_deserializer;
use crate::message::{MessageId, Metadata};
use crate::transport::frame::{
    Frame, FrameRead, FrameSinkExt, FrameStreamExt, FrameWrite, PayloadType,
};

use super::{ConnTypeReadWrite, ConnTypePayload};

impl<'de, R> serde::Deserializer<'de> for DeserializerOwned<serde_cbor::Deserializer<R>>
where
    R: serde_cbor::de::Read<'de>,
{
    type Error = <&'de mut serde_cbor::Deserializer<R> as serde::Deserializer<'de>>::Error;

    // use a macro to generate the code
    impl_inner_deserializer!();
}

#[async_trait]
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
                .frame_stream()
                .next()
                .await?
                .and_then(|frame| Self::unmarshal(&frame.payload)),
        )
    }

    async fn read_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
        let reader = &mut self.reader;

        match reader.frame_stream().next().await? {
            Ok(frame) => {
                // log::debug!("frame: {:?}", frame);
                let de = Self::from_bytes(frame.payload);
                Some(Ok(de))
            }
            Err(e) => return Some(Err(e)),
        }
    }
}

#[async_trait]
impl<R, W> CodecWrite for Codec<R, W, ConnTypeReadWrite>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin,
{
    async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
    where
        H: serde::Serialize + Metadata + Send,
    {
        let writer = &mut self.writer;

        let id = header.get_id();
        let buf = Self::marshal(&header)?;
        let frame = Frame::new(id, 0, PayloadType::Header, buf);

        log::trace!("Header id: {} sent", &id);
        writer.frame_sink().send(frame).await
    }

    async fn write_body(
        &mut self,
        id: &MessageId,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error> {
        let writer = &mut self.writer;

        let buf = Self::marshal(&body)?;
        let frame = Frame::new(id.to_owned(), 1, PayloadType::Data, buf);

        log::trace!("Body id: {} sent", id);

        writer.frame_sink().send(frame).await
    }
}

impl<R, W, C> Marshal for Codec<R, W, C> {
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        serde_cbor::to_vec(val).map_err(|e| e.into())
    }
}

impl<R, W, C> Unmarshal for Codec<R, W, C> {
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        serde_cbor::from_slice(buf).map_err(|e| e.into())
    }
}

impl<R, W, C> EraseDeserializer for Codec<R, W, C> {
    fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send> {
        let de = serde_cbor::Deserializer::from_reader(Cursor::new(buf));

        let de_owned = DeserializerOwned::new(de);
        Box::new(erased::Deserializer::erase(de_owned))
    }
}
