//! Impplementation of `CodecRead`, `CodecWrite`, `Marshal` and `Unmarshal` traits with `bincode`

use async_trait::async_trait;
use bincode::{DefaultOptions, Options};
use erased_serde as erased;
use futures::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};
use futures::{StreamExt, SinkExt};
use serde::de::Visitor;
use std::{io::Cursor}; // serde doesn't support AsyncRead

use super::{Codec, CodecRead, CodecWrite, DeserializerOwned, Marshal, Unmarshal};
use crate::error::Error;
use crate::macros::impl_inner_deserializer;
use crate::message::{MessageId, Metadata};
use crate::transport::frame::{Frame, FrameRead, FrameStreamExt, FrameSinkExt, FrameWrite, PayloadType};

use super::{ConnTypeWebSocket, ConnTypeReadWrite, PayloadRead, PayloadWrite};

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

        let de = match reader.frame_stream().next().await? {
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

impl<R, W, C> Marshal for Codec<R, W, C>
// where
//     R: Send,
//     W: Send,
{
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .serialize(&val)
            .map_err(|err| err.into())
    }
}

impl<R, W, C> Unmarshal for Codec<R, W, C>
// where
//     R: Send,
//     W: Send,
{
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .deserialize(buf)
            .map_err(|err| err.into())
    }
}

#[async_trait]
impl<R, W> CodecRead for Codec<R, W, ConnTypeWebSocket>
where 
    R: PayloadRead + Send,
    W: Send,
{
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
    where
            H: serde::de::DeserializeOwned 
    {
        let reader = &mut self.reader;

        Some(
            reader
                .read_payload()
                .await?
                .and_then(|payload| Self::unmarshal(&payload))
        )
    }

    async fn read_body(&mut self) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
        let reader = &mut self.reader;

        let de = match reader.read_payload().await? {
            Ok(payload) => {
                log::debug!("frame: {:?}", payload);
                bincode::Deserializer::with_reader(
                    Cursor::new(payload),
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
impl<R, W> CodecWrite for Codec<R, W, ConnTypeWebSocket>
where 
    R: Send,
    W: PayloadWrite + Send,
{
    async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
    where
            H: serde::Serialize + Metadata + Send 
    {
        let writer = &mut self.writer;
        let buf = Self::marshal(&header)?;
        // log::trace!("Header id: {} sent", &id);
        writer.write_payload(buf).await
    }

    async fn write_body(&mut self, _: &MessageId, body: &(dyn erased::Serialize + Send + Sync)) -> Result<(), Error> {
        let writer = &mut self.writer;
        let buf = Self::marshal(&body)?;
        // log::trace!("Body id: {} sent", id);
        writer.write_payload(buf).await
    }
}
