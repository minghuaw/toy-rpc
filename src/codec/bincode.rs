//! Impplementation of `CodecRead`, `CodecWrite`, `Marshal` and `Unmarshal` traits with `bincode`

use async_trait::async_trait;
use bincode::{DefaultOptions, Options};
use erased_serde as erased;
use futures::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};
use futures::StreamExt;
use serde::de::Visitor;
use std::io::Cursor; // serde doesn't support AsyncRead

use super::{Codec, CodecRead, CodecWrite, DeserializerOwned, Marshal, Unmarshal};
use crate::error::Error;
use crate::macros::impl_inner_deserializer;
use crate::message::{MessageId, Metadata};
use crate::transport::frame::{FrameRead, FrameStreamExt, FrameWrite, PayloadType};

impl<'de, R, O> serde::Deserializer<'de> for DeserializerOwned<bincode::Deserializer<R, O>>
where
    R: bincode::BincodeRead<'de>,
    O: bincode::Options,
{
    type Error = <&'de mut bincode::Deserializer<R, O> as serde::Deserializer<'de>>::Error;

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
                .frames()
                .next()
                .await?
                .and_then(|frame| Self::unmarshal(&frame.payload)),
        )
    }

    async fn read_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
        let reader = &mut self.reader;

        let de = match reader.frames().next().await? {
            Ok(frame) => {
                log::debug!("frame: {:?}", frame);
                bincode::Deserializer::with_reader(
                    Cursor::new(frame.payload),
                    bincode::DefaultOptions::new().with_fixint_encoding(),
                )
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

        let bytes_sent = writer.write_frame(&id, 0, PayloadType::Header, &buf).await?;

        log::trace!("Header id: {} written with {} bytes", &id, &bytes_sent);

        Ok(())
    }

    async fn write_body(
        &mut self,
        id: MessageId,
        body: Box<(dyn erased::Serialize + Send + Sync)>,
    ) -> Result<(), Error> {
        let writer = &mut self.writer;

        let buf = Self::marshal(&body)?;

        let bytes_sent = writer
            .write_frame(&id, 1, PayloadType::Data, &buf)
            .await?;

        log::trace!("Body id: {} written with {} bytes", id, &bytes_sent);
        
        Ok(())
    }
}

impl<R, W> Marshal for Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .serialize(&val)
            .map_err(|err| err.into())
    }
}

impl<R, W> Unmarshal for Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .deserialize(buf)
            .map_err(|err| err.into())
    }
}
