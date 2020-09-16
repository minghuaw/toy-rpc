use futures::io::{
    AsyncRead,
    AsyncWrite,
    BufReader,
    BufWriter,
    AsyncBufReadExt,
    AsyncWriteExt
};
use std::io::Cursor; // serde doesn't support AsyncRead
use async_trait::async_trait;
use serde;
use erased_serde as erased;
use serde::de::Visitor;

use crate::error::Error;
use crate::rpc::{
    Metadata,
    MessageId
};
use super::{
    Marshal,
    Unmarshal,
    CodecRead,
    CodecWrite,
    DeserializerOwned
};

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        Error::ParseError { source: Box::new(err)}
    }
}

pub struct Codec<T> where T: AsyncRead + AsyncWrite + Send + Sync + Clone {
    reader: BufReader<T>,
    writer: BufWriter<T>,
}

impl<T> Codec<T> where T: AsyncRead + AsyncWrite + Send + Sync + Clone {
    pub fn new(stream: T) -> Self {
        Self {
            reader: BufReader::new(stream.clone()),
            writer: BufWriter::new(stream.clone()),
        }
    }
}

#[async_trait]
impl<T> CodecRead for Codec<T> 
where T: Unpin + AsyncRead + AsyncWrite + Send + Sync + Clone
{
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
    where H: serde::de::DeserializeOwned {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf).await {
            Ok(_) => {
                Some(
                    Self::unmarshal(buf.as_bytes())
                )
            },
            Err(_) => {
                None
            }
        }
    }

    async fn read_body<'c>(&'c mut self) -> Option<Result<Box<dyn erased::Deserializer<'c> + Send + Sync + 'c>, Error>> {
        let mut buf = String::new();
        
        let de = match self.reader.read_line(&mut buf).await {
            Ok(_) => {
                serde_json::Deserializer::from_reader(
                    Cursor::new(
                        buf.into_bytes()
                    )
                )
            },
            Err(e) => return Some(Err(e.into()))
        };

        // wrap the deserializer as DeserializerOwned
        let de_owned = DeserializerOwned::new(de);

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
impl<T> CodecWrite for Codec<T> 
where T: Unpin + AsyncRead + AsyncWrite + Send + Sync + Clone {
    async fn write_header<H>(&mut self, header: H) -> Result<usize, Error>
    where H: serde::Serialize + Metadata + Send {
        let _ = header.get_id();
        let buf = Self::marshal(&header)?;

        let bytes_sent = self.writer.write(&buf).await?;
        self.writer.write(b"\n").await?;
        self.writer.flush().await?;
        Ok(bytes_sent)
    }

    async fn write_body(&mut self, _: MessageId, body: &(dyn erased::Serialize + Send + Sync)) -> Result<usize, Error> {
        let buf = Self::marshal(&body)?;

        let bytes_sent = self.writer.write(&buf).await?;
        self.writer.write(b"\n").await?;
        self.writer.flush().await?;
        Ok(bytes_sent)
    }
}

impl<T> Marshal for Codec<T> where T: AsyncRead + AsyncWrite + Send + Sync + Clone {
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(val)
            .map_err(|e| e.into())
    }
}

impl<T> Unmarshal for Codec<T> where T: AsyncRead + AsyncWrite + Send + Sync + Clone {
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        serde_json::from_slice(buf)
            .map_err(|e| e.into())
    }
}

impl<'de, R> serde::Deserializer<'de> for DeserializerOwned<serde_json::Deserializer<R>> where R: serde_json::de::Read<'de> {
    type Error = <&'de mut serde_json::Deserializer<R> as serde::Deserializer<'de>>::Error;

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
