use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::Error;

use super::{Frame, FrameHeader, PayloadLen, HEADER_LEN};
use super::{FrameRead, FrameWrite};

#[async_trait]
impl<R: AsyncRead + Unpin + Send + Sync> FrameRead for R {
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

        Some(Ok(Frame::new(
            header.message_id,
            header.frame_id,
            header.payload_type.into(),
            payload,
        )))
    }
}

#[async_trait]
impl<W: AsyncWrite + AsyncWriteExt + Unpin + Send + Sync> FrameWrite for W {
    async fn write_frame(&mut self, frame: Frame) -> Result<(), Error> {
        let Frame {
            message_id,
            frame_id,
            payload_type,
            payload,
        } = frame;

        // check if buf length exceed maximum
        if payload.len() > PayloadLen::MAX as usize {
            return Err(Error::TransportError {
                msg: format!(
                    "Payload length exceeded maximum. Max is {}, found {}",
                    PayloadLen::MAX,
                    payload.len()
                ),
            });
        }

        // construct frame header
        let header = FrameHeader::new(message_id, frame_id, payload_type, payload.len() as u32);

        // log::debug!("FrameHeader {:?}", &header);
        // write header
        self.write(&header.to_vec()?).await?;
        // self.flush().await?;

        // write payload
        let _ = self.write(&payload).await?;
        self.flush().await?;

        Ok(())
    }
}
