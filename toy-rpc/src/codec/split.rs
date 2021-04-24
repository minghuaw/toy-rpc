//! Implements split of Codec into read half and write half
use async_trait::async_trait;
use std::marker::PhantomData;

use super::*;

pub struct CodecReadHalf<R, C, CT>{
    pub reader: R,
    marker: PhantomData<C>,
    conn_type: PhantomData<CT>,
}
pub struct CodecWriteHalf<W, C, CT> {
    pub writer: W,
    marker: PhantomData<C>,
    conn_type: PhantomData<CT>,
}

impl<W, C, CT> Marshal for CodecWriteHalf<W, C, CT> 
where 
    C: Marshal
{
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        C::marshal(val)
    }
}

impl<R, C, CT> Unmarshal for CodecReadHalf<R, C, CT> 
where 
    C: Unmarshal
{
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        C::unmarshal(buf)
    }
}

impl<R, C, CT> EraseDeserializer for CodecReadHalf<R, C, CT> 
where
    C: EraseDeserializer
{
    fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send> {
        C::from_bytes(buf)
    }
}

impl<R, W, ConnType> Codec<R, W, ConnType> {
    fn split(self) -> (CodecReadHalf<R, Self, ConnType>, CodecWriteHalf<W, Self, ConnType>) {
        (
            CodecReadHalf {
                reader: self.reader,
                marker: PhantomData,
                conn_type: PhantomData
            },
            CodecWriteHalf {
                writer: self.writer,
                marker: PhantomData,
                conn_type: PhantomData,
            }
        )
    }
}


cfg_if! {
    if #[cfg(all(
        any(feature = "async_std_runtime", feature = "tokio_runtime"),
        any(
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
                feature = "serde_rmp",
                not(feature = "serde_cbor"),
                not(feature = "serde_json"),
                not(feature = "serde_bincode"),
            )
        )
    ))] {
        use crate::transport::frame::{Frame, PayloadType, FrameRead, FrameWrite};

        #[async_trait]
        impl<R, C> CodecRead for CodecReadHalf<R, C, ConnTypeReadWrite> 
        where 
            R: FrameRead + Send + Sync + Unpin,
            C: Unmarshal + EraseDeserializer + Send
        {
            async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
            where
                H: serde::de::DeserializeOwned,
            {
                let reader = &mut self.reader;
        
                Some(
                    reader
                        .read_frame()
                        .await?
                        .and_then(|frame| Self::unmarshal(&frame.payload)),
                )
            }
        
            async fn read_body(
                &mut self,
            ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
                let reader = &mut self.reader;
        
                match reader.read_frame().await? {
                    Ok(frame) => {
                        let de = C::from_bytes(frame.payload);
                        Some(Ok(de))
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
        }
        
        #[async_trait]
        impl<W, C> CodecWrite for CodecWriteHalf<W, C, ConnTypeReadWrite> 
        where 
            W: FrameWrite + Send + Sync + Unpin,
            C: Marshal + Send,
        {
            async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
            where
                H: serde::Serialize + Metadata + Send,
            {
                let writer = &mut self.writer;
        
                let id = header.get_id();
                let buf = Self::marshal(&header)?;
                let frame = Frame::new(id, 0, PayloadType::Header, buf);
        
                writer.write_frame(frame).await
            }
        
            async fn write_body(
                &mut self,
                id: &MessageId,
                body: &(dyn erased::Serialize + Send + Sync),
            ) -> Result<(), Error> {
                let writer = &mut self.writer;
        
                let buf = Self::marshal(&body)?;
                let frame = Frame::new(id.to_owned(), 1, PayloadType::Data, buf);
        
                writer.write_frame(frame).await
            }
        }
    }
}

cfg_if!{
    if #[cfg(all(
        any(
            feature = "async_std_runtime",
            feature = "tokio_runtime",
            feature = "http_tide",
            feature = "http_warp",
            feature = "http_actix_web"
        ),
        any(
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
            )
        )
    ))] {
        use crate::transport::{PayloadRead, PayloadWrite};

        #[async_trait]
        impl<R, C> CodecRead for CodecReadHalf<R, C, ConnTypePayload>
        where
            R: PayloadRead + Send,
            C: Unmarshal + EraseDeserializer + Send 
        {
            async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
            where
                H: serde::de::DeserializeOwned,
            {
                let reader = &mut self.reader;

                Some(
                    reader
                        .read_payload()
                        .await?
                        .and_then(|payload| Self::unmarshal(&payload)),
                )
            }

            async fn read_body(
                &mut self,
            ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
                let reader = &mut self.reader;

                match reader.read_payload().await? {
                    Ok(payload) => {
                        let de = Self::from_bytes(payload);
                        Some(Ok(de))
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
        }

        #[async_trait]
        impl<W, C> CodecWrite for CodecWriteHalf<W, C, ConnTypePayload> 
        where 
            W: PayloadWrite + Send,
            C: Marshal + Send,
        {
            async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
            where
                H: serde::Serialize + Metadata + Send,
            {
                let writer = &mut self.writer;
                let buf = Self::marshal(&header)?;
                writer.write_payload(buf).await
            }

            async fn write_body(
                &mut self,
                _: &MessageId,
                body: &(dyn erased::Serialize + Send + Sync),
            ) -> Result<(), Error> {
                let writer = &mut self.writer;
                let buf = Self::marshal(&body)?;
                writer.write_payload(buf).await
            }
        }
    }
}