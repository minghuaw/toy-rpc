use ::tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter, ReadHalf, WriteHalf};
use ::tokio::io::split;

use super::*;

impl<R, W> Codec<R, W, ConnTypeReadWrite>
where
    R: AsyncRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
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
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin,
{
    pub fn new(stream: T) -> Self {
        let (reader, writer) = split(stream);
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);

        Self::with_reader_writer(reader, writer)
    }
}