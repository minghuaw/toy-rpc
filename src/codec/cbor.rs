use async_trait::async_trait;
use erased_serde as erased;
use futures::io::{AsyncBufRead, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use futures::StreamExt;
use serde::de::Visitor;
use std::io::Cursor; // serde doesn't support AsyncRead

use super::{CodecRead, CodecWrite, DeserializerOwned, Marshal, Unmarshal};
use crate::error::Error;
use crate::macros::impl_inner_deserializer;
use crate::message::{MessageId, Metadata};
use crate::transport::frame::{FrameRead, FrameStreamExt, FrameWrite, PayloadType};

impl<'de, R> serde::Deserializer<'de> for DeserializerOwned<serde_cbor::Deserializer<R>>
where
    R: serde_cbor::de::Read<'de>,
{
    type Error = <&'de mut serde_cbor::Deserializer<R> as serde::Deserializer<'de>>::Error;

    // use a macro to generate the code
    impl_inner_deserializer!();
}

pub struct Codec<R, W> {
    reader: R,
    writer: W,
}

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
                serde_cbor::Deserializer::from_reader(Cursor::new(frame.payload))
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
    async fn write_header<H>(&mut self, header: H) -> Result<usize, Error>
    where
        H: serde::Serialize + Metadata + Send,
    {
        let writer = &mut self.writer;

        let id = header.get_id();
        let buf = Self::marshal(&header)?;

        let bytes_sent = writer.write_frame(id, 0, PayloadType::Header, &buf).await?;
        Ok(bytes_sent)
    }

    async fn write_body(
        &mut self,
        message_id: MessageId,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error> {
        let writer = &mut self.writer;

        let buf = Self::marshal(&body)?;

        let bytes_sent = writer
            .write_frame(message_id, 1, PayloadType::Data, &buf)
            .await?;
        Ok(bytes_sent)
    }
}

impl<R, W> Marshal for Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        serde_cbor::to_vec(val).map_err(|e| e.into())
    }
}

impl<R, W> Unmarshal for Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        serde_cbor::from_slice(buf).map_err(|e| e.into())
    }
}
