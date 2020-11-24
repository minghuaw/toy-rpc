use async_trait::async_trait;
use erased_serde as erased;

use crate::error::Error;
use crate::message::{MessageId, Metadata, RequestHeader, ResponseHeader};

#[cfg(all(feature = "serde_json", not(feauture = "bincode")))]
mod json;

#[cfg(all(feature = "serde_json", not(feauture = "bincode")))]
pub use crate::codec::json::Codec as DefaultCodec;

#[cfg(all(feature = "bincode", not(feature = "serde_json")))]
mod bincode;

#[cfg(all(feature = "bincode", not(feature = "serde_json")))]
pub use crate::codec::bincode::Codec as DefaultCodec;

#[async_trait]
pub trait ServerCodec: Send + Sync {
    async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>>;
    async fn read_request_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + Sync + 'static>, Error>>;

    async fn write_response(
        &mut self,
        header: ResponseHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error>;
}

#[async_trait]
pub trait ClientCodec: Send + Sync {
    async fn read_response_header(&mut self) -> Option<Result<ResponseHeader, Error>>;
    async fn read_response_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + Sync + 'static>, Error>>;

    async fn write_request(
        &mut self,
        header: RequestHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error>;
}

#[async_trait]
pub trait CodecRead: Unmarshal {
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
    where
        H: serde::de::DeserializeOwned;

    async fn read_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + Sync + 'static>, Error>>;
}

#[async_trait]
pub trait CodecWrite: Marshal {
    async fn write_header<H>(&mut self, header: H) -> Result<usize, Error>
    where
        H: serde::Serialize + Metadata + Send;

    async fn write_body(
        &mut self,
        message_id: MessageId,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error>;
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
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + Sync + 'static>, Error>> {
        self.read_body().await
    }

    async fn write_response(
        &mut self,
        header: ResponseHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error> {
        let id = header.get_id();

        let h = self.write_header(header).await?;
        let b = self.write_body(id, body).await?;

        Ok(h + b)
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
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + Sync + 'static>, Error>> {
        self.read_body().await
    }

    async fn write_request(
        &mut self,
        header: RequestHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error> {
        let id = header.get_id();

        let h = self.write_header(header).await?;
        let b = self.write_body(id, body).await?;

        Ok(h + b)
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
