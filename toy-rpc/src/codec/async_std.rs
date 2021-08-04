//! Codec implementation with tokio runtime

use futures::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter, ReadHalf, WriteHalf,
};

use crate::util::GracefulShutdown;

use super::*;

impl<R, W> Codec<R, W, ConnTypeReadWrite>
where
    R: AsyncRead + Send + Unpin,
    W: AsyncWrite + Send + Unpin,
{
    /// Creates a `Codec` with a reader and a writer
    ///
    /// The reader must implements the `AsyncRead` trait, and the writer
    /// must implements the `AsyncWrite` trait
    pub fn with_reader_writer(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            conn_type: PhantomData,
        }
    }
}

impl<T> Codec<BufReader<ReadHalf<T>>, BufWriter<WriteHalf<T>>, ConnTypeReadWrite>
where
    T: AsyncRead + AsyncWrite + Send + Unpin,
{
    /// Creates a `Codec` with a stream that implements both `AsyncRead` and `AsyncWrite`.
    pub fn new(stream: T) -> Self {
        let (reader, writer) = stream.split();
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);

        Self::with_reader_writer(reader, writer)
    }
}

#[async_trait]
impl<R, W> GracefulShutdown for Codec<R, W, ConnTypeReadWrite>
where
    R: AsyncRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
    async fn close(&mut self) {
        match self.writer.flush().await {
            Ok(()) => (),
            Err(e) => log::error!("Error closing connection: {}", e),
        };

        match AsyncWriteExt::close(&mut self.writer).await {
            Ok(()) => (),
            Err(e) => log::error!("Error closing connection: {}", e),
        };
    }
}

#[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
#[async_trait::async_trait]
impl<R, W> GracefulShutdown for Codec<R, W, ConnTypePayload>
where
    R: Send,
    W: GracefulShutdown + Send,
{
    async fn close(&mut self) {
        self.writer.close().await;
    }
}
