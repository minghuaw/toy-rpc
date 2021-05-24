use async_trait::async_trait;
use bincode::{DefaultOptions, Options};
use cfg_if::cfg_if;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;

use crate::message::MessageId;
use crate::{error::Error, util::GracefulShutdown};

const INVALID_PROTOCOL: &str = "Magic byte mismatch.\rClient may be using a different protocol or version.\rClient of version <0.5.0 is not compatible with Server of version >0.5.0";
const END_FRAME_ID: FrameId = 131;

cfg_if! {
    if #[cfg(any(
        feature = "async_std_runtime",
        feature = "http_tide"
    ))] {
        use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
    } else if #[cfg(any(
        feature = "tokio_runtime",
        feature = "http_warp",
        feature = "http_actix_web"
    ))] {
        use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
    }
}

type FrameId = u8;
type PayloadLen = u32;
const MAGIC: u8 = 13;

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
pub trait FrameRead {
    async fn read_frame(&mut self) -> Option<Result<Frame, Error>>;
}

/// Trait for custom binary transport protocol
///
/// `AsyncWrite` is required because `async_std::net::TcpStream`
/// only implements `AsyncWrite` and `AsyncRead`
///
#[async_trait]
pub trait FrameWrite {
    async fn write_frame(&mut self, frame: Frame) -> Result<(), Error>;
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
            .map_err(|err| Error::ParseError(err))
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, Error> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .serialize(self)
            .map_err(|err| Error::ParseError(err))
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
            2 => Self::Trailer,
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
    pub fn new(
        message_id: MessageId,
        frame_id: FrameId,
        payload_type: PayloadType,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            message_id,
            frame_id,
            payload_type,
            payload,
        }
    }
}

#[async_trait]
impl<R: AsyncRead + Unpin + Send + Sync> FrameRead for R {
    async fn read_frame(&mut self) -> Option<Result<Frame, Error>> {
        // read magic first
        let magic = &mut [0];
        let _ = self.read_exact(magic).await.ok()?;
        if magic[0] != MAGIC {
            return Some(Err(Error::IoError(std::io::Error::new(
                ErrorKind::InvalidData,
                INVALID_PROTOCOL,
            ))));
        }

        // read header
        let mut buf = vec![0; *HEADER_LEN];
        let _ = self.read_exact(&mut buf).await.ok()?;
        let header = match FrameHeader::from_slice(&buf) {
            Ok(h) => h,
            Err(e) => return Some(Err(e)),
        };

        // determine if end frame is received
        match header.payload_type.into() {
            PayloadType::Trailer => {
                if header.frame_id == END_FRAME_ID
                    && header.message_id == 0
                    && header.payload_len == 0
                {
                    return None;
                }
            }
            _ => {}
        }

        // read frame payload
        let mut payload = vec![0; header.payload_len as usize];
        let _ = self.read_exact(&mut payload).await.ok()?;

        Some(Ok(Frame::new(
            header.message_id,
            header.frame_id,
            header.payload_type.into(),
            payload,
        )))
    }
}

#[async_trait]
impl<W: AsyncWrite + Unpin + Send + Sync> FrameWrite for W {
    async fn write_frame(&mut self, frame: Frame) -> Result<(), Error> {
        let Frame {
            message_id,
            frame_id,
            payload_type,
            payload,
        } = frame;

        // check if buf length exceed maximum
        if payload.len() > PayloadLen::MAX as usize {
            return Err(Error::IoError(std::io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Payload length exceeded maximum. Max is {}, found {}",
                    PayloadLen::MAX,
                    payload.len()
                ),
            )));
        }

        // construct frame header
        let header = FrameHeader::new(message_id, frame_id, payload_type, payload.len() as u32);

        // write magic first
        self.write(&[MAGIC]).await?;

        // write header
        self.write(&header.to_vec()?).await?;
        // self.flush().await?;

        // write payload
        let _ = self.write(&payload).await?;
        self.flush().await?;

        Ok(())
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

#[async_trait]
impl<T> GracefulShutdown for T
where
    T: FrameWrite + Send,
{
    async fn close(&mut self) {
        // send a trailer frame with message id 0 and END_FRAME_ID and empty payload
        let end_frame = Frame::new(0, END_FRAME_ID, PayloadType::Trailer, Vec::with_capacity(0));
        match self.write_frame(end_frame).await {
            Ok(_) => {}
            Err(err) => log::error!("{:?}", err),
        }
    }
}
