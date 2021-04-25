//! Implements split of Codec into read half and write half
use async_trait::async_trait;
use std::marker::PhantomData;

use super::*;

pub trait Split {
    type Reader: ServerCodecRead;
    type Writer: ServerCodecWrite;

    fn split(self) -> (Self::Reader, Self::Writer);
}

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

        impl<R, W> Split for Codec<R, W, ConnTypeReadWrite> 
        where 
            R: FrameRead + Send + Sync + Unpin,
            W: FrameWrite + Send + Sync + Unpin,
        {
            type Reader = CodecReadHalf::<R, Self, ConnTypeReadWrite>;
            type Writer = CodecWriteHalf::<W, Self, ConnTypeReadWrite>;

            fn split(self) -> (Self::Reader, Self::Writer) {
                (
                    CodecReadHalf::<R, Self, ConnTypeReadWrite> {
                        reader: self.reader,
                        marker: PhantomData,
                        conn_type: PhantomData
                    },
                    CodecWriteHalf::<W, Self, ConnTypeReadWrite> {
                        writer: self.writer,
                        marker: PhantomData,
                        conn_type: PhantomData,
                    }
                )
            }
        }

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

        impl<R, W> Split for Codec<R, W, ConnTypePayload> 
        where 
            R: PayloadRead + Send,
            W: PayloadWrite + Send,
        {
            type Reader = CodecReadHalf::<R, Self, ConnTypePayload>;
            type Writer = CodecWriteHalf::<W, Self, ConnTypePayload>;

            fn split(self) -> (Self::Reader, Self::Writer) {
                (
                    CodecReadHalf::<R, Self, ConnTypePayload> {
                        reader: self.reader,
                        marker: PhantomData,
                        conn_type: PhantomData
                    },
                    CodecWriteHalf::<W, Self, ConnTypePayload> {
                        writer: self.writer,
                        marker: PhantomData,
                        conn_type: PhantomData,
                    }
                )
            }
        }

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


#[async_trait]
pub trait ServerCodecRead: Send {
    async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>>;
    async fn read_request_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>>;
}

#[async_trait]
pub trait ServerCodecWrite: Send {
    // (Probably) don't need to worry about header/body interleaving
    // because rust guarantees only one mutable reference at a time
    async fn write_response(
        &mut self,
        header: ResponseHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error>;  
}

#[async_trait]
impl<T> ServerCodecRead for T 
where 
    T: CodecRead + Send
{
    async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>> {
        self.read_header().await
    }

    async fn read_request_body(&mut self) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
        self.read_body().await
    }
}

#[async_trait]
impl<T> ServerCodecWrite for T 
where 
    T: CodecWrite + Send,
{
    async fn write_response(&mut self, header: ResponseHeader, body: &(dyn erased::Serialize + Send + Sync)) -> Result<(), Error> {
        let id = header.get_id();

        log::trace!("Sending response id: {}", &id);

        self.write_header(header).await?;
        self.write_body(&id, body).await?;

        Ok(())
    }
}