use async_trait::async_trait;
use bincode::{DefaultOptions, Options};
use futures::io::{AsyncBufRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
// use futures::ready;
use futures::task::{Context, Poll};
use futures::{Stream, Sink};
use lazy_static::lazy_static;
use pin_project::pin_project;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::error::Error;
use crate::message::MessageId;

type FrameId = u8;
type PayloadLen = u32;

// const HEADER_LEN: usize = 8; // header length in bytes
lazy_static! {
    static ref HEADER_LEN: usize =
        bincode::serialized_size(&FrameHeader::default()).unwrap() as usize;
}

/// Trait for custom binary transport protocol
/// 
/// `AsyncBufRead` or `AsyncRead` is required because `async_std::net::TcpStream`
/// only implements `AsyncWrite` and `AsyncRead`
/// 
#[async_trait]
pub trait FrameRead: AsyncBufRead {
    async fn read_frame(&mut self) -> Option<Result<Frame, Error>>;
}

/// Trait for custom binary transport protocol
/// 
/// `AsyncWrite` is required because `async_std::net::TcpStream`
/// only implements `AsyncWrite` and `AsyncRead`
/// 
#[async_trait]
pub trait FrameWrite: AsyncWrite {
    async fn write_frame(
        &mut self,
        message_id: &MessageId,
        frame_id: FrameId,
        payload_type: PayloadType,
        buf: &[u8],
    ) -> Result<usize, Error>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct FrameHeader {
    message_id: MessageId,
    frame_id: FrameId,
    payload_type: u8, // this is not used for now
    payload_len: PayloadLen,
}

impl FrameHeader {
    pub fn new(
        message_id: MessageId,
        frame_id: FrameId,
        payload_type: PayloadType,
        payload_len: PayloadLen,
    ) -> Self {
        Self {
            message_id,
            frame_id,
            payload_type: payload_type.into(),
            payload_len,
        }
    }

    pub fn from_slice(buf: &[u8]) -> Result<Self, Error> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .deserialize(&buf)
            .map_err(|err| Error::ParseError { source: err })
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, Error> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .serialize(self)
            .map_err(|err| Error::ParseError { source: err })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayloadType {
    Header,
    Data,
    Trailer,
}

impl Default for PayloadType {
    fn default() -> Self {
        PayloadType::Header
    }
}

impl From<u8> for PayloadType {
    fn from(t: u8) -> Self {
        match t {
            0 => Self::Header,
            1 => Self::Data,
            _ => Self::Trailer,
        }
    }
}

impl From<PayloadType> for u8 {
    fn from(t: PayloadType) -> Self {
        match t {
            PayloadType::Header => 0,
            PayloadType::Data => 1,
            PayloadType::Trailer => 2,
        }
    }
}

#[derive(Debug)]
pub struct Frame {
    pub message_id: MessageId,
    pub frame_id: FrameId,
    pub payload_type: PayloadType,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(message_id: MessageId, frame_id: FrameId, payload_type: PayloadType, payload: Vec<u8>) -> Self {
        Self {
            message_id,
            frame_id,
            payload_type,
            payload,
        }
    }
}

#[async_trait]
impl<R: AsyncBufRead + Unpin + Send + Sync> FrameRead for R {
    async fn read_frame(&mut self) -> Option<Result<Frame, Error>> {
        // read header
        let mut buf = vec![0; *HEADER_LEN];
        let _ = self.read_exact(&mut buf).await.ok()?;
        let header = match FrameHeader::from_slice(&buf) {
            Ok(h) => h,
            Err(e) => return Some(Err(e)),
        };

        // read frame payload
        let mut payload = vec![0; header.payload_len as usize];
        let _ = self.read_exact(&mut payload).await.ok()?;

        Some(
            Ok(
                Frame::new(
                    header.message_id, 
                    header.frame_id, 
                    header.payload_type.into(), 
                    payload
                )
            )
        )
    }
}

#[async_trait]
impl<W: AsyncWrite + AsyncWriteExt + Unpin + Send + Sync> FrameWrite for W {
    async fn write_frame(
        &mut self,
        message_id: &MessageId,
        frame_id: FrameId,
        payload_type: PayloadType,
        buf: &[u8],
    ) -> Result<usize, Error> {
        // check if buf length exceed maximum
        if buf.len() > PayloadLen::MAX as usize {
            return Err(Error::TransportError {
                msg: format!(
                    "Payload length exceeded maximum. Max is {}, found {}", 
                    PayloadLen::MAX, 
                    buf.len()
                ),
            });
        }

        // construct frame header
        let header = FrameHeader::new(*message_id, frame_id, payload_type, buf.len() as u32);

        // log::debug!("FrameHeader {:?}", &header);
        // write header
        self.write(&header.to_vec()?).await?;
        // self.flush().await?;

        // write payload
        let nbytes = self.write(buf).await?;
        self.flush().await?;

        Ok(nbytes)
    }
}

#[pin_project]
pub struct FrameStream<'r, T>
where
    T: AsyncBufRead + Send + Sync + Unpin,
{
    #[pin]
    inner: &'r mut T,
    header: Option<FrameHeader>,
}

pub trait FrameStreamExt<T>
where
    T: AsyncBufRead + Unpin + Send + Sync,
{
    fn frame_stream(&mut self) -> FrameStream<T>;
}

impl<T: AsyncBufRead + Send + Sync + Unpin> FrameStreamExt<T> for T {
    fn frame_stream(&mut self) -> FrameStream<T> {
        FrameStream {
            inner: self,
            header: None,
        }
    }
}

impl<'r, T: AsyncBufRead + Unpin + Send + Sync> Stream for FrameStream<'r, T> {
    type Item = Result<Frame, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let mut reader = this.inner;
        let header = this.header;

        loop {
            match header {
                None => {
                    let buf = futures::ready!(reader.as_mut().poll_fill_buf(cx)?);
                    if buf.is_empty() {
                        return Poll::Ready(None);
                    }

                    // pending if not enough bytes are found in buffer
                    if buf.len() < *HEADER_LEN {
                        // there will be more bytes coming
                        return Poll::Pending;
                    }

                    // decode header
                    match FrameHeader::from_slice(&buf[..*HEADER_LEN]) {
                        Ok(h) => {
                            header.get_or_insert(h);

                            // consume the bytes used for header
                            reader.as_mut().consume(*HEADER_LEN);

                            // Cannot return Poll::Pending here
                            continue;
                        }
                        Err(e) => {
                            log::debug!("Parse error {}", e);
                            return Poll::Ready(Some(Err(e)));
                        }
                    }
                }
                Some(ref h) => {
                    log::debug!("{:?}", h);

                    // if the message is of unit type
                    if h.payload_len == 0 {
                        let frame = Frame {
                            message_id: h.message_id,
                            frame_id: h.frame_id,
                            payload_type: h.payload_type.into(),
                            payload: vec![],
                        };

                        return Poll::Ready(Some(Ok(frame)));
                    }

                    let buf = futures::ready!(reader.as_mut().poll_fill_buf(cx)?);
                    if buf.is_empty() {
                        return Poll::Ready(None);
                    }

                    if buf.len() < h.payload_len as usize {
                        // there will be more bytes coming
                        return Poll::Pending;
                    }

                    let payload = &buf[..h.payload_len as usize];

                    // construct frame
                    let frame = Frame {
                        message_id: h.message_id,
                        frame_id: h.frame_id,
                        payload_type: h.payload_type.into(),
                        payload: payload.to_vec(),
                    };

                    reader.as_mut().consume(h.payload_len as usize);
                    header.as_mut().take();

                    return Poll::Ready(Some(Ok(frame)));
                }
            }
        }
    }
}


#[pin_project]
pub struct FrameSink<'w, T>
where 
    T: AsyncWrite + Send + Sync + Unpin,
{
    #[pin]
    inner: &'w mut T,
    buf: Vec<u8>,
}

pub trait FrameSinkExt<T> 
where 
    T: AsyncWrite + Send + Sync + Unpin
{
    fn frame_sink(&mut self) -> FrameSink<T>;
}

impl<T: AsyncWrite + Send + Sync + Unpin> FrameSinkExt<T> for T {
    fn frame_sink(&mut self) -> FrameSink<T> {
        FrameSink {
            inner: self,
            buf: Vec::new()
        }
    }
}

impl<'w, T: AsyncWrite + Send + Sync + Unpin> Sink<Frame> for FrameSink<'w, T> {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, mut item: Frame) -> Result<(), Self::Error> {
        let this = self.project();
        let buf = this.buf;
        // let mut writer = this.inner;

        // check payload len
        let payload_len = item.payload.len();
        if payload_len > PayloadLen::MAX as usize {
            return Err(
                Error::TransportError {
                    msg: format!(
                        "Payload length exceeded maximum. Max is {}, found {}", 
                        PayloadLen::MAX, 
                        payload_len
                    ),
                }
            )
        }

        // write header
        let header = FrameHeader::new(
            item.message_id,
            item.frame_id,
            item.payload_type,
            payload_len as u32,
        );
        
        buf.append(&mut header.to_vec()?);
        buf.append(&mut item.payload);

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let mut writer = this.inner;
        let buf = this.buf;

        futures::ready!(writer.as_mut().poll_write(cx, &buf))
            .map_err(|e| Error::IoError(e))?;
        futures::ready!(writer.as_mut().poll_flush(cx))
            .map_err(|e| Error::IoError(e))?;

        buf.clear();

        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let writer = this.inner;

        writer.poll_close(cx)
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_header_length() {
        let b = bincode::serialized_size(&FrameHeader::default()).unwrap();
        let l = std::mem::size_of::<FrameHeader>();

        println!("{:?}", b);
        println!("{:?}", l);
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    struct ModifiedHeader {
        message_id: MessageId,
        frame_id: FrameId,
        payload_type: u8,
        payload_len: PayloadLen,
    }

    #[test]
    fn bool_length() {
        let fh = bincode::serialized_size(&FrameHeader::default()).unwrap();
        let mh = bincode::serialized_size(&ModifiedHeader::default()).unwrap();

        println!("FrameHeader len: {}", fh);
        println!("ModifiedHeader len: {}", mh);
    }
}
