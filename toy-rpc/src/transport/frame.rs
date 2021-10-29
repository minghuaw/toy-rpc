//! A custom framed binary transport

use async_trait::async_trait;
use bincode::{DefaultOptions, Options};
use cfg_if::cfg_if;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;

use crate::error::IoError;
use crate::message::MessageId;
use crate::{error::Error, util::GracefulShutdown};

use super::as_io_err_other;

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
        // default uses fixint size
        bincode::serialized_size(&FrameHeader::default()).unwrap() as usize;
}

/// Trait for custom binary transport protocol
///
/// `AsyncBufRead` or `AsyncRead` is required because `async_std::net::TcpStream`
/// only implements `AsyncWrite` and `AsyncRead`
///
#[async_trait]
pub trait FrameRead {
    /// Reads a frame
    async fn read_frame(&mut self) -> Option<Result<Frame, IoError>>;
}

/// Trait for custom binary transport protocol
///
/// `AsyncWrite` is required because `async_std::net::TcpStream`
/// only implements `AsyncWrite` and `AsyncRead`
///
#[async_trait]
pub trait FrameWrite {
    /// Writes a frame
    async fn write_frame(
        &mut self,
        frame_header: FrameHeader,
        payload: &[u8],
    ) -> Result<(), IoError>;
}

/// Header of a frame
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrameHeader {
    message_id: MessageId,
    frame_id: FrameId,
    payload_type: u8, // this is not used for now
    payload_len: PayloadLen,
}

impl FrameHeader {
    /// Constructs a new frame header
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

    /// Constructs a new frame header from bytes
    pub fn from_slice(buf: &[u8]) -> Result<Self, Error> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .deserialize(&buf)
            .map_err(|err| Error::ParseError(err))
    }

    /// Convert a frame header to bytes
    pub fn to_vec(&self) -> Result<Vec<u8>, IoError> {
        DefaultOptions::new()
            .with_fixint_encoding()
            .serialize(self)
            .map_err(|err| IoError::new(std::io::ErrorKind::InvalidData, err.to_string()))
    }
}

/// Type of payload carried by a frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayloadType {
    /// Message header
    Header,
    /// Message body
    Data,
    /// Message trailer
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

/// Frame
#[derive(Debug)]
pub struct Frame {
    /// Message id
    pub message_id: MessageId,
    /// (RESERVED) Frame id, this is a separate id reserved for multi-frame transport
    pub frame_id: FrameId,
    /// Type of the payload
    pub payload_type: PayloadType,
    /// Payload
    pub payload: Vec<u8>,
}

impl Frame {
    /// Constructs a new frame
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
impl<R: AsyncRead + Unpin + Send> FrameRead for R {
    async fn read_frame(&mut self) -> Option<Result<Frame, IoError>> {
        // read magic first
        let magic = &mut [0];
        let _ = self.read_exact(magic).await.ok()?;
        if magic[0] != MAGIC {
            return Some(Err(std::io::Error::new(
                ErrorKind::InvalidData,
                INVALID_PROTOCOL,
            )));
        }

        // read header
        let mut buf = vec![0; *HEADER_LEN];
        let _ = self.read_exact(&mut buf).await.ok()?;
        let header = match FrameHeader::from_slice(&buf) {
            Ok(h) => h,
            Err(e) => {
                let err = as_io_err_other(&e);
                return Some(Err(err));
            }
        };

        // determine if end frame is received
        if let PayloadType::Trailer = header.payload_type.into() {
            if header.frame_id == END_FRAME_ID && header.message_id == 0 && header.payload_len == 0
            {
                return None;
            }
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
impl<W: AsyncWrite + Unpin + Send> FrameWrite for W {
    async fn write_frame(
        &mut self,
        frame_header: FrameHeader,
        payload: &[u8],
    ) -> Result<(), IoError> {
        // check if buf length exceeds maximum
        if payload.len() > PayloadLen::MAX as usize {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Payload length exceeded maximum. Max is {}, found {}",
                    PayloadLen::MAX,
                    payload.len()
                ),
            ));
        }

        // write magic first
        self.write_all(&[MAGIC]).await?;

        // write header
        self.write_all(&frame_header.to_vec()?).await?;

        // write payload
        let _ = self.write_all(&payload).await?;
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
        // let end_frame = Frame::new(0, END_FRAME_ID, PayloadType::Trailer, Vec::with_capacity(0));
        let end_frame_header = FrameHeader::new(0, END_FRAME_ID, PayloadType::Trailer, 0);
        let payload = Vec::with_capacity(0);
        self.write_frame(end_frame_header, &payload)
            .await
            .unwrap_or_else(|e| log::error!("{}", e));
    }
}
