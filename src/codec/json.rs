use async_trait::async_trait;
use erased_serde as erased;
use futures::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use serde;
use serde::de::Visitor;
use std::io::Cursor; // serde doesn't support AsyncRead

use super::{CodecRead, CodecWrite, DeserializerOwned, Marshal, Unmarshal};
use crate::error::Error;
use crate::macros::impl_inner_deserializer;
use crate::message::{MessageId, Metadata};

impl<'de, R> serde::Deserializer<'de> for DeserializerOwned<serde_json::Deserializer<R>>
where
    R: serde_json::de::Read<'de>,
{
    type Error = <&'de mut serde_json::Deserializer<R> as serde::Deserializer<'de>>::Error;

    // the rest is simply calling self.inner.deserialize_xxx()
    // use a macro to generate the code
    impl_inner_deserializer!();
}

pub struct Codec<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Clone,
{
    reader: BufReader<T>,
    writer: BufWriter<T>,
}

impl<T> Codec<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Clone,
{
    pub fn new(stream: T) -> Self {
        Self {
            reader: BufReader::new(stream.clone()),
            writer: BufWriter::new(stream.clone()),
        }
    }
}

#[async_trait]
impl<T> CodecRead for Codec<T>
where
    T: Unpin + AsyncRead + AsyncWrite + Send + Sync + Clone,
{
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
    where
        H: serde::de::DeserializeOwned,
    {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf).await {
            Ok(_) => Some(Self::unmarshal(buf.as_bytes())),
            Err(_) => None,
        }
    }

    async fn read_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + Sync + 'static>, Error>> {
        let mut buf = String::new();

        let de = match self.reader.read_line(&mut buf).await {
            Ok(_) => serde_json::Deserializer::from_reader(Cursor::new(buf.into_bytes())),
            Err(e) => return Some(Err(e.into())),
        };

        // wrap the deserializer as DeserializerOwned
        let de_owned = DeserializerOwned::new(de);

        Some(Ok(Box::new(erased::Deserializer::erase(de_owned))))
    }
}

#[async_trait]
impl<T> CodecWrite for Codec<T>
where
    T: Unpin + AsyncRead + AsyncWrite + Send + Sync + Clone,
{
    async fn write_header<H>(&mut self, header: H) -> Result<usize, Error>
    where
        H: serde::Serialize + Metadata + Send,
    {
        let _ = header.get_id();
        let buf = Self::marshal(&header)?;

        let bytes_sent = self.writer.write(&buf).await?;
        self.writer.write(b"\n").await?;
        self.writer.flush().await?;
        Ok(bytes_sent)
    }

    async fn write_body(
        &mut self,
        _: MessageId,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error> {
        let buf = Self::marshal(&body)?;

        let bytes_sent = self.writer.write(&buf).await?;
        self.writer.write(b"\n").await?;
        self.writer.flush().await?;
        Ok(bytes_sent)
    }
}

impl<T> Marshal for Codec<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Clone,
{
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(val).map_err(|e| e.into())
    }
}

impl<T> Unmarshal for Codec<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Clone,
{
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        serde_json::from_slice(buf).map_err(|e| e.into())
    }
}
