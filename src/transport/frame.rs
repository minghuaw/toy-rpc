use async_trait::async_trait;
use bincode::{DefaultOptions, Options};
use futures::io::{AsyncBufRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use futures::ready;
use futures::task::{Context, Poll};
use futures::Stream;
use lazy_static::lazy_static;
use log;
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

#[async_trait]
pub trait FrameRead: AsyncBufRead {
    async fn read_frame(&mut self) -> Option<Result<Frame, Error>>;
}

#[async_trait]
pub trait FrameWrite: AsyncWrite {
    async fn write_frame(
        &mut self,
        message_id: MessageId,
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
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(message_id: MessageId, frame_id: FrameId, payload: Vec<u8>) -> Self {
        Self {
            message_id,
            frame_id,
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

        Some(Ok(Frame::new(header.message_id, header.frame_id, payload)))
    }
}

#[async_trait]
impl<W: AsyncWrite + AsyncWriteExt + Unpin + Send + Sync> FrameWrite for W {
    async fn write_frame(
        &mut self,
        message_id: MessageId,
        frame_id: FrameId,
        payload_type: PayloadType,
        buf: &[u8],
    ) -> Result<usize, Error> {
        // check if buf length exceed maximum
        if buf.len() > PayloadLen::MAX as usize {
            return Err(Error::TransportError {
                msg: format!("Expected {}, found {}", PayloadLen::MAX, buf.len())
            });
        }

        // construct frame header
        let header = FrameHeader::new(message_id, frame_id, payload_type, buf.len() as u32);

        // log::debug!("FrameHeader {:?}", &header);
        // write header
        self.write(&header.to_vec()?).await?;
        self.flush().await?;

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

impl<'r, T: AsyncBufRead + Unpin + Send + Sync> Stream for FrameStream<'r, T> {
    type Item = Result<Frame, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let mut reader = this.inner;
        let header = this.header;

        loop {
            match header {
                None => {
                    let buf = ready!(reader.as_mut().poll_fill_buf(cx)?);
                    if buf.len() == 0 {
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
                            payload: vec![],
                        };

                        return Poll::Ready(Some(Ok(frame)));
                    }

                    let buf = ready!(reader.as_mut().poll_fill_buf(cx)?);
                    if buf.len() == 0 {
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

pub trait FrameStreamExt<T>
where
    T: AsyncBufRead + Unpin + Send + Sync,
{
    fn frames<'a>(&'a mut self) -> FrameStream<'a, T>;
}

impl<T: AsyncBufRead + Send + Sync + Unpin> FrameStreamExt<T> for T {
    fn frames<'a>(&'a mut self) -> FrameStream<'a, T> {
        FrameStream {
            inner: self,
            header: None,
        }
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
