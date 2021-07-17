//! Implements json codec with `async_std` runtime

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "serde_bincode")] {

    } else if #[cfg(feature = "serde_cbor")] {

    } else if #[cfg(feature = "serde_rmp")] {

    } else {
        use futures::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
        use async_trait::async_trait;
        use erased_serde as erased;
        use std::marker::PhantomData;

        use super::*;
        use crate::codec::split::{SplittableCodec};
        use crate::codec::split::{CodecReadHalf, CodecWriteHalf};
        use crate::util::GracefulShutdown;

        #[async_trait]
        impl<R, C> CodecRead for CodecReadHalf<R, C, ConnTypeReadWrite>
        where
            R: AsyncBufRead + Send + Unpin,
            C: Unmarshal + EraseDeserializer + Send,
        {
            // async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
            // where
            //     H: serde::de::DeserializeOwned,
            // {
            //     let mut buf = String::new();
            //     match self.reader.read_line(&mut buf).await {
            //         Ok(n) => {
            //             if n == 0 {
            //                 // EOF, probably end of connection
            //                 return None;
            //             }

            //             Some(Self::unmarshal(buf.as_bytes()))
            //         }
            //         Err(err) => {
            //             Some(Err(err.into()))
            //         }
            //     }
            // }

            // async fn read_body(
            //     &mut self
            // ) -> Option<Result<Box<InboundBody>, Error>> {
            //     let mut buf = String::new();
            //     match self.reader.read_line(&mut buf).await {
            //         Ok(n) => {
            //             if n == 0 {
            //                 // EOF, probably client closed connection
            //                 return None;
            //             }

            //             let de = Self::from_bytes(buf.into_bytes());
            //             Some(Ok(de))
            //         }
            //         Err(err) => return Some(Err(err.into())),
            //     }
            // }

            async fn read_bytes(&mut self) -> Option<Result<Vec<u8>, Error>> {
                let mut buf = String::new();
                match self.reader.read_line(&mut buf).await {
                    Ok(n) => {
                        if n == 0 {
                            // EOF
                            return None
                        }
                        Some(Ok(buf.into_bytes()))
                    },
                    Err(err) => Some(Err(err.into()))
                }
            }
        }

        #[async_trait]
        impl<W, C> CodecWrite for CodecWriteHalf<W, C, ConnTypeReadWrite>
        where
            W: AsyncWrite + Send + Unpin,
            C: Marshal + Send,
        {
            async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
            where
                H: serde::Serialize + Metadata + Send,
            {
                let _ = header.get_id();
                let buf = Self::marshal(&header)?;

                let _ = self.writer.write(&buf).await?;
                self.writer.flush().await?;

                Ok(())
            }

            async fn write_body(
                &mut self,
                _id: MessageId,
                body: &(dyn erased::Serialize + Send + Sync),
            ) -> Result<(), Error> {
                let buf = Self::marshal(&body)?;

                let _ = self.writer.write(&buf).await?;
                self.writer.flush().await?;

                Ok(())
            }

            async fn write_body_bytes(&mut self, _: MessageId, bytes: &[u8]) -> Result<(), Error> {
                let _ = self.writer.write(bytes).await?;
                self.writer.flush().await?;
                Ok(())
            }

        }

        impl<R, W> SplittableCodec for Codec<R, W, ConnTypeReadWrite>
        where
            R: AsyncBufRead + Send + Unpin,
            W: AsyncWrite + GracefulShutdown + Send + Unpin,
        {
            type Reader = CodecReadHalf::<R, Self, ConnTypeReadWrite>;
            type Writer = CodecWriteHalf::<W, Self, ConnTypeReadWrite>;

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

        #[async_trait]
        impl<T> GracefulShutdown for T
        where
            T: AsyncWrite + Send + Unpin
        {
            async fn close(&mut self) {
                match self.flush().await {
                    Ok(()) => (),
                    Err(e) => log::error!("Error closing connection: {}", e),
                };
                match AsyncWriteExt::close(self).await {
                    Ok(()) => (),
                    Err(e) => log::error!("Error closing connection: {}", e),
                };
            }
        }
    }
}
