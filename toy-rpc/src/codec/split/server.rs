use super::*;

pub trait ServerCodecSplit {
    type Reader: ServerCodecRead;
    type Writer: ServerCodecWrite;

    fn split(self) -> (Self::Writer, Self::Reader);
}
#[async_trait]
pub trait ServerCodecRead: Send {
    async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>>;
    async fn read_request_body(
        &mut self,
    ) -> Option<Result<RequestDeserializer, Error>>;
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

cfg_if!{
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
        impl<R, W> ServerCodecSplit for Codec<R, W, ConnTypeReadWrite> 
        where 
            R: FrameRead + Send + Sync + Unpin,
            W: FrameWrite + Send + Sync + Unpin,
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

        impl<R, W> ServerCodecSplit for Codec<R, W, ConnTypePayload> 
        where 
            R: PayloadRead + Send,
            W: PayloadWrite + Send,
        {
            type Reader = CodecReadHalf::<R, Self, ConnTypePayload>;
            type Writer = CodecWriteHalf::<W, Self, ConnTypePayload>;

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

#[async_trait]
impl<T> ServerCodecRead for T 
where 
    T: CodecRead + Send
{
    async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>> {
        self.read_header().await
    }

    async fn read_request_body(&mut self) -> Option<Result<RequestDeserializer, Error>> {
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