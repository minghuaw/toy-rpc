//! Implements `SplittableCodec`

#[cfg(any(feature = "tokio_runtime", feature = "async_std_runtime"))]
use async_trait::async_trait;
use std::marker::PhantomData;

use crate::{error::IoError, util::GracefulShutdown};

use super::*;

#[allow(dead_code)]
pub(crate) struct CodecReadHalf<R, C, CT> {
    pub reader: R,
    pub marker: PhantomData<C>,
    pub conn_type: PhantomData<CT>,
}

#[allow(dead_code)]
pub(crate) struct CodecWriteHalf<W, C, CT> {
    pub writer: W,
    pub marker: PhantomData<C>,
    pub conn_type: PhantomData<CT>,
}

impl<W, C, CT> Marshal for CodecWriteHalf<W, C, CT>
where
    C: Marshal,
{
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        C::marshal(val)
    }
}

impl<R, C, CT> Unmarshal for CodecReadHalf<R, C, CT>
where
    C: Unmarshal,
{
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        C::unmarshal(buf)
    }
}

impl<R, C, CT> EraseDeserializer for CodecReadHalf<R, C, CT>
where
    C: EraseDeserializer,
{
    fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send> {
        C::from_bytes(buf)
    }
}

/// Split a Codec into a writing half and a reading half
pub trait SplittableCodec: Marshal + Unmarshal {
    /// Type of the writing half
    type Writer: CodecWrite + GracefulShutdown;
    /// Type of the reading half
    type Reader: CodecRead;

    /// Split the codec into a writer and a reader
    fn split(self) -> (Self::Writer, Self::Reader);
}

/* -------------------------------------------------------------------------- */
/*                              // TCP Transport                              */
/* -------------------------------------------------------------------------- */
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
        use crate::transport::frame::{PayloadType, FrameRead, FrameWrite, FrameHeader};

        #[async_trait]
        impl<R, C> CodecRead for CodecReadHalf<R, C, ConnTypeReadWrite>
        where
            R: FrameRead + Send + Unpin,
            C: Unmarshal + EraseDeserializer + Send
        {
            async fn read_bytes(&mut self) -> Option<Result<Vec<u8>, Error>> {
                self.reader.read_frame().await
                    .map(|res| {
                        res.map(|f| f.payload)
                            .map_err(Into::into)
                    })
            }
        }

        #[async_trait]
        impl<W, C> CodecWrite for CodecWriteHalf<W, C, ConnTypeReadWrite>
        where
            W: FrameWrite + Send + Unpin,
            C: Marshal + Send,
        {
            async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
            where
                H: serde::Serialize + Metadata + Send,
            {
                let writer = &mut self.writer;

                let id = header.id();
                let buf = Self::marshal(&header)?;
                // let frame = Frame::new(id, 0, PayloadType::Header, buf);
                let frame_header = FrameHeader::new(id, 0, PayloadType::Header, buf.len() as u32);

                writer.write_frame(frame_header, &buf).await?;
                Ok(())
            }

            async fn write_body(
                &mut self,
                id: MessageId,
                body: &(dyn erased::Serialize + Send + Sync),
            ) -> Result<(), Error> {
                let writer = &mut self.writer;
                let buf = Self::marshal(&body)?;
                // let frame = Frame::new(id.to_owned(), 1, PayloadType::Data, buf.to_owned());
                let frame_header = FrameHeader::new(id, 1, PayloadType::Data, buf.len() as u32);
                writer.write_frame(frame_header, &buf).await?;
                Ok(())
            }

            async fn write_body_bytes(&mut self, id: MessageId, bytes: &[u8]) -> Result<(), Error> {
                // let frame = Frame::new(*id, 1, PayloadType::Data, bytes);
                let frame_header = FrameHeader::new(id, 1, PayloadType::Data, bytes.len() as u32);
                self.writer.write_frame(frame_header, bytes).await?;
                Ok(())
            }
        }

        impl<R, W> SplittableCodec for Codec<R, W, ConnTypeReadWrite>
        where
            R: FrameRead + Send + Unpin,
            W: FrameWrite + GracefulShutdown + Send + Unpin
        {
            type Writer = CodecWriteHalf::<W, Self, ConnTypeReadWrite>;
            type Reader = CodecReadHalf::<R, Self, ConnTypeReadWrite>;

            fn split(self) -> (Self::Writer, Self::Reader) {
                (
                    CodecWriteHalf::<W, Self, ConnTypeReadWrite> {
                        writer: self.writer,
                        marker: PhantomData,
                        conn_type: PhantomData,
                    },
                    CodecReadHalf::<R, Self, ConnTypeReadWrite> {
                        reader: self.reader,
                        marker: PhantomData,
                        conn_type: PhantomData
                    }
                )
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                           // WebSocket Transport                           */
/* -------------------------------------------------------------------------- */
cfg_if! {
    if #[cfg(all(
        any(
            feature = "async_std_runtime",
            feature = "tokio_runtime",
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
        #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
        use crate::transport::{PayloadRead, PayloadWrite};

        #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
        #[async_trait]
        impl<R, C> CodecRead for CodecReadHalf<R, C, ConnTypePayload>
        where
            R: PayloadRead + Send,
            C: Unmarshal + EraseDeserializer + Send
        {
            async fn read_bytes(&mut self) -> Option<Result<Vec<u8>, Error>> {
                self.reader.read_payload().await
                    .map(|res| res.map_err(Into::into))
            }
        }

        #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
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
                writer.write_payload(&buf).await?;
                Ok(())
            }

            async fn write_body(
                &mut self,
                _: MessageId,
                body: &(dyn erased::Serialize + Send + Sync),
            ) -> Result<(), Error> {
                let buf = Self::marshal(&body)?;
                let writer = &mut self.writer;
                writer.write_payload(&buf).await?;
                Ok(())
            }

            async fn write_body_bytes(&mut self, _: MessageId, bytes: &[u8]) -> Result<(), Error> {
                self.writer.write_payload(bytes).await?;
                Ok(())
            }
        }

        #[async_trait]
        impl<W, C, Conn> GracefulShutdown for CodecWriteHalf<W, C, Conn>
        where
            W: GracefulShutdown + Send,
            C: Send,
            Conn: Send,
        {
            async fn close(&mut self) {
                self.writer.close().await;
            }
        }

        #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
        impl<R, W> SplittableCodec for Codec<R, W, ConnTypePayload>
        where
            R: PayloadRead + Send,
            W: PayloadWrite + GracefulShutdown + Send,
        {
            type Writer = CodecWriteHalf::<W, Self, ConnTypePayload>;
            type Reader = CodecReadHalf::<R, Self, ConnTypePayload>;

            fn split(self) -> (Self::Writer, Self::Reader) {
                (
                    CodecWriteHalf::<W, Self, ConnTypePayload> {
                        writer: self.writer,
                        marker: PhantomData,
                        conn_type: PhantomData,
                    },
                    CodecReadHalf::<R, Self, ConnTypePayload> {
                        reader: self.reader,
                        marker: PhantomData,
                        conn_type: PhantomData
                    }
                )
            }
        }
    }
}
