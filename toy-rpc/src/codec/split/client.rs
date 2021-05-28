//! Implements `SplittableClientCodec`

use super::*;
/// Split the ClientCodec into a reader half and writer half
///
/// This is trait replaces `ClientCodec`.
pub trait SplittableClientCodec {
    /// Type of the reader half
    type Reader: ClientCodecRead;

    /// Type of the write half
    type Writer: ClientCodecWrite;

    /// Splits a client codec into a reader half and a writer half
    fn split(self) -> (Self::Writer, Self::Reader);
}

/// A client side codec that can read response header and response body
#[async_trait]
pub trait ClientCodecRead: Send {
    /// Reads the response header. Returns `None` if the underlying connection is closed
    async fn read_response_header(&mut self) -> Option<Result<ResponseHeader, Error>>;

    /// Reads the response body. Returns `None` if the underlying connection is closed
    async fn read_response_body(&mut self) -> Option<Result<RequestDeserializer, Error>>;
}

/// A client side codec that can write requests
#[async_trait]
pub trait ClientCodecWrite: Send {
    /// Writes a request to the server
    async fn write_request(
        // (Probably) don't need to worry about header/body interleaving
        // because rust guarantees only one mutable reference at a time
        &mut self,
        header: RequestHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error>;
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
        impl<R, W> SplittableClientCodec for Codec<R, W, ConnTypeReadWrite>
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

cfg_if! {
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

        impl<R, W> SplittableClientCodec for Codec<R, W, ConnTypePayload>
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
impl<T> ClientCodecRead for T
where
    T: CodecRead + Send,
{
    async fn read_response_header(&mut self) -> Option<Result<ResponseHeader, Error>> {
        self.read_header().await
    }

    async fn read_response_body(&mut self) -> Option<Result<RequestDeserializer, Error>> {
        self.read_body().await
    }
}

#[async_trait]
impl<T> ClientCodecWrite for T
where
    T: CodecWrite + Send,
{
    async fn write_request(
        &mut self,
        header: RequestHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<(), Error> {
        let id = header.get_id();

        log::trace!("Sending request id: {}", &id);

        self.write_header(header).await?;
        self.write_body(&id, body).await?;

        Ok(())
    }
}
