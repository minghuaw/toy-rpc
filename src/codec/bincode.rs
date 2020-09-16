use std::io::Cursor; // serde doesn't support AsyncRead
use futures::io::{
    AsyncRead,
    BufReader,
    AsyncWrite,
    BufWriter,
    AsyncBufRead,
    AsyncWriteExt
};
use futures::StreamExt;
use async_trait::async_trait;
use bincode::{
    DefaultOptions,
    Options,
};
use erased_serde as erased;
use serde::de::Visitor;

use crate::error::Error;
use crate::rpc::{
    Metadata,
    MessageId
};
use crate::transport::frame::{
    PayloadType,
    FrameRead,
    FrameWrite,
    FrameStreamExt
};
use super::{
    Marshal,
    Unmarshal,
    CodecRead,
    CodecWrite,
    DeserializerOwned
};

impl From<bincode::Error> for crate::error::Error {
    fn from(err: bincode::Error) -> Self {
        Error::ParseError {source: err}
    }
}

pub struct Codec<R, W>
where 
    R: FrameRead + Send + Sync + Unpin,
    W: FrameWrite + Send + Sync + Unpin
{
    reader: R,
    writer: W,
}

impl<R, W> Codec<R, W> 
where 
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin
{
    pub fn from_reader_writer(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
        }
    }
}

impl<T> Codec<BufReader<T>, BufWriter<T>> where T: AsyncRead + AsyncWrite + Send + Sync + Unpin + Clone {
    pub fn new(stream: T) -> Self {
        Self::from_reader_writer(
            BufReader::new(stream.clone()), 
            BufWriter::new(stream.clone())
        )
    }
}

#[async_trait]
impl<R, W> CodecRead for Codec<R, W> 
where 
    R: FrameRead + Send + Sync + Unpin,
    W: FrameWrite + Send + Sync + Unpin
{
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>> 
    where 
        H: serde::de::DeserializeOwned 
    {
        let reader = &mut self.reader;

        Some(
            reader.frames().next().await?
                .and_then(|frame| {
                    Self::unmarshal(&frame.payload)
                })
        )
    }

    async fn read_body<'c>(&'c mut self) -> Option<Result<Box<dyn erased::Deserializer<'c> + Send + Sync + 'c>, Error>> {
        let reader = &mut self.reader;

        let de = match reader.frames().next().await? {
            Ok(frame) => {
                bincode::Deserializer::with_reader(
                    Cursor::new(frame.payload), 
                    bincode::DefaultOptions::new()
                        .with_fixint_encoding()
                )
            },
            Err(e) => return Some(Err(e))
        };

        // wrap the deserializer as DeserializerOwned
        let de_owned = DeserializerOwned::new(de);

        // returns a Deserializer referencing to decoder
        Some(
            Ok(
                Box::new(
                    erased::Deserializer::erase(
                        de_owned
                    )
                )
            )
        )
    }
}

#[async_trait]
impl<R, W> CodecWrite for Codec<R, W> 
where 
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin
{
    async fn write_header<H>(&mut self, header: H) -> Result<usize, Error>
        where H: serde::Serialize + Metadata + Send
    {
        let writer = &mut self.writer;

        let id = header.get_id();
        let buf = Self::marshal(&header)?;

        let bytes_sent = writer.write_frame(id, 0, PayloadType::Header, &buf).await?;
        Ok(bytes_sent)
    }

    async fn write_body(&mut self, message_id: MessageId, body: &(dyn erased::Serialize + Send + Sync)) -> Result<usize, Error> {
        let writer = &mut self.writer;

        let buf = Self::marshal(&body)?;

        let bytes_sent = writer.write_frame(message_id, 1, PayloadType::Data, &buf).await?;
        Ok(bytes_sent)
    }
}


impl<R, W> Marshal for Codec<R, W> 
where 
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin
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
    W: AsyncWrite + Send + Sync + Unpin
{
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .deserialize(buf)
            .map_err(|err| err.into())
    }
}

impl<'de, R, O> serde::Deserializer<'de> for DeserializerOwned<bincode::Deserializer<R, O>>
where 
    R: bincode::BincodeRead<'de>,
    O: bincode::Options
{
    type Error = <&'de mut bincode::Deserializer<R, O> as serde::Deserializer<'de>>::Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_any(visitor)
    }

    fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_bool(visitor)
    }

    fn deserialize_byte_buf<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_byte_buf(visitor)
    }

    fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_bytes(visitor)
    }

    fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_char(visitor)
    }

    fn deserialize_enum<V>(
            mut self,
            name: &'static str,
            variants: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_enum(name, variants, visitor)
    }

    fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_f32(visitor)
    }

    fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_f64(visitor)
    }

    fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_i16(visitor)
    }

    fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_i32(visitor)
    }

    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_i64(visitor)
    }

    fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_i8(visitor)
    }

    fn deserialize_identifier<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_identifier(visitor)
    }

    fn deserialize_ignored_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_ignored_any(visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_map(visitor)
    }

    fn deserialize_newtype_struct<V>(
            mut self,
            name: &'static str,
            visitor: V,
        ) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_option(visitor)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_seq(visitor)
    }

    fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_str(visitor)
    }

    fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_string(visitor)
    }

    fn deserialize_struct<V>(
            mut self,
            name: &'static str,
            fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_struct(name, fields, visitor)
    }

    fn deserialize_tuple<V>(mut self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_tuple(len, visitor)
    }

    fn deserialize_tuple_struct<V>(
            mut self,
            name: &'static str,
            len: usize,
            visitor: V,
        ) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_tuple_struct(name, len, visitor)
    }

    fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_u16(visitor)
    }

    fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_u32(visitor)
    }

    fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_u64(visitor)
    }

    fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_u8(visitor)
    }

    fn deserialize_unit<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_unit(visitor)
    }

    fn deserialize_unit_struct<V>(
            mut self,
            name: &'static str,
            visitor: V,
        ) -> Result<V::Value, Self::Error>
    where
            V: Visitor<'de> {
        self.inner.deserialize_unit_struct(name, visitor)
    }
}

