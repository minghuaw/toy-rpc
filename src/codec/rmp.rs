//! Impplementation of `CodecRead`, `CodecWrite`, `Marshal` and `Unmarshal` traits with `rmp-serde`

use async_trait::async_trait;
use erased_serde as erased;
use futures::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};
use futures::{StreamExt, SinkExt};
use serde::de::Visitor;
use std::io::Cursor; // serde doesn't support AsyncRead

use super::{Codec, CodecRead, CodecWrite, DeserializerOwned, Marshal, Unmarshal};
use crate::error::Error;
use crate::macros::impl_inner_deserializer;
use crate::message::{MessageId, Metadata};
use crate::transport::frame::{Frame, FrameRead, FrameStreamExt, FrameSinkExt, FrameWrite, PayloadType};

impl<'de, R> serde::Deserializer<'de>
    for DeserializerOwned<rmp_serde::Deserializer<rmp_serde::decode::ReadReader<R>>>
where
    R: std::io::Read,
{
    type Error = <&'de mut rmp_serde::Deserializer<rmp_serde::decode::ReadReader<R>> as serde::Deserializer<'de>>::Error;

    // use a macro to generate the code
    impl_inner_deserializer!();
}

#[async_trait]
impl<R, W> CodecRead for Codec<R, W>
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
                .and_then(|frame| Self::unmarshal(&frame)),
        )
    }

    async fn read_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
        let reader = &mut self.reader;

        let de = match reader.frame_stream().next().await? {
            Ok(frame) => {
                log::debug!("frame: {:?}", frame);
                rmp_serde::Deserializer::new(Cursor::new(frame))
            }
            Err(e) => return Some(Err(e)),
        };

        // wrap the deserializer as DeserializerOwned
        let de_owned = DeserializerOwned::new(de);

        // returns a Deserializer referencing to decoder
        Some(Ok(Box::new(erased::Deserializer::erase(de_owned))))
    }
}

#[async_trait]
impl<R, W> CodecWrite for Codec<R, W>
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

impl<R, W> Marshal for Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        match val.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
            Ok(_) => Ok(buf),
            Err(e) => Err(e.into()),
        }
    }
}

impl<R, W> Unmarshal for Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        let mut de = rmp_serde::Deserializer::new(buf);
        serde::Deserialize::deserialize(&mut de).map_err(|e| e.into())
    }
}
