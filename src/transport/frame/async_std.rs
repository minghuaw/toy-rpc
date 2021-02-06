use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
// use futures::{Stream, Sink};
// use std::{pin::Pin, task::{Context, Poll}};

use crate::error::Error;

use super::{Frame, FrameHeader, PayloadLen, HEADER_LEN, MAGIC};
use super::{FrameRead, FrameWrite}; // traits from upper level // structs from the upper level

// #[async_trait]
// impl<R: AsyncRead + Unpin + Send + Sync> FrameRead for R {
//     async fn read_frame(&mut self) -> Option<Result<Frame, Error>> {
//         // read magic first
//         let magic = &mut [0];
//         let _ = self.read_exact(magic).await.ok()?;
//         log::debug!("MAGIC read: {:?}", &magic);
//         if magic[0] != MAGIC {
//             return Some(Err(Error::TransportError{
//                 msg: "Wrong magic found.
//                 Is client using the same transport protocol?
//                 Client of version <0.5.0 is not compatible with Server of version >0.5.0"
//                 .into()
//             }))
//         }
        
//         // read header
//         let mut buf = vec![0; *HEADER_LEN];
//         let _ = self.read_exact(&mut buf).await.ok()?;
//         let header = match FrameHeader::from_slice(&buf) {
//             Ok(h) => h,
//             Err(e) => return Some(Err(e)),
//         };

//         // read frame payload
//         let mut payload = vec![0; header.payload_len as usize];
//         let _ = self.read_exact(&mut payload).await.ok()?;

//         Some(Ok(Frame::new(
//             header.message_id,
//             header.frame_id,
//             header.payload_type.into(),
//             payload,
//         )))
//     }
// }

// #[async_trait]
// impl<W: AsyncWrite + AsyncWriteExt + Unpin + Send + Sync> FrameWrite for W {
//     async fn write_frame(&mut self, frame: Frame) -> Result<(), Error> {
//         let Frame {
//             message_id,
//             frame_id,
//             payload_type,
//             payload,
//         } = frame;

//         // check if buf length exceed maximum
//         if payload.len() > PayloadLen::MAX as usize {
//             return Err(Error::TransportError {
//                 msg: format!(
//                     "Payload length exceeded maximum. Max is {}, found {}",
//                     PayloadLen::MAX,
//                     payload.len()
//                 ),
//             });
//         }

//         // construct frame header
//         let header = FrameHeader::new(message_id, frame_id, payload_type, payload.len() as u32);

//         // write magic first
//         self.write(&[MAGIC]).await?;

//         // write header
//         self.write(&header.to_vec()?).await?;
//         // self.flush().await?;

//         // write payload
//         let _ = self.write(&payload).await?;
//         self.flush().await?;

//         Ok(())
//     }
// }

// pub trait FrameStreamExt<T>
// where
//     T: AsyncBufRead + Unpin + Send + Sync,
// {
//     fn frame_stream(&mut self) -> FrameStream<T>;
// }

// impl<T: AsyncBufRead + Send + Sync + Unpin> FrameStreamExt<T> for T {
//     fn frame_stream(&mut self) -> FrameStream<T> {
//         FrameStream {
//             inner: self,
//             header: None,
//         }
//     }
// }
// pub trait FrameSinkExt<T>
// where
//     T: AsyncWrite + Send + Sync + Unpin,
// {
//     fn frame_sink(&mut self) -> FrameSink<T>;
// }

// impl<T: AsyncWrite + Send + Sync + Unpin> FrameSinkExt<T> for T {
//     fn frame_sink(&mut self) -> FrameSink<T> {
//         FrameSink {
//             inner: self,
//             buf: Vec::new(),
//         }
//     }
// }

// impl<'r, T: AsyncBufRead + Unpin + Send + Sync> Stream for FrameStream<'r, T> {
//     type Item = Result<Frame, Error>;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         let this = self.project();
//         let mut reader = this.inner;
//         let header = this.header;

//         loop {
//             match header {
//                 None => {
//                     let buf = futures::ready!(reader.as_mut().poll_fill_buf(cx)?);
//                     if buf.is_empty() {
//                         return Poll::Ready(None);
//                     }

//                     // pending if not enough bytes are found in buffer
//                     if buf.len() < *HEADER_LEN {
//                         // there will be more bytes coming
//                         return Poll::Pending;
//                     }

//                     // decode header
//                     match FrameHeader::from_slice(&buf[..*HEADER_LEN]) {
//                         Ok(h) => {
//                             header.get_or_insert(h);

//                             // consume the bytes used for header
//                             reader.as_mut().consume(*HEADER_LEN);

//                             // Cannot return Poll::Pending here
//                             continue;
//                         }
//                         Err(e) => {
//                             log::debug!("Parse error {}", e);
//                             return Poll::Ready(Some(Err(e)));
//                         }
//                     }
//                 }
//                 Some(ref h) => {
//                     log::debug!("{:?}", h);

//                     // if the message is of unit type
//                     if h.payload_len == 0 {
//                         // no need to return a Frame
//                         let frame = Frame {
//                             message_id: h.message_id,
//                             frame_id: h.frame_id,
//                             payload_type: h.payload_type.into(),
//                             payload: vec![],
//                         };
//                         return Poll::Ready(Some(Ok(frame)));
//                     }

//                     let buf = futures::ready!(reader.as_mut().poll_fill_buf(cx)?);
//                     if buf.is_empty() {
//                         return Poll::Ready(None);
//                     }

//                     if buf.len() < h.payload_len as usize {
//                         // there will be more bytes coming
//                         return Poll::Pending;
//                     }

//                     let payload = buf[..h.payload_len as usize].to_vec();

//                     // construct frame
//                     let frame = Frame {
//                         message_id: h.message_id,
//                         frame_id: h.frame_id,
//                         payload_type: h.payload_type.into(),
//                         payload,
//                     };

//                     // use a separate
//                     // no need to constuct a Frame to return

//                     reader.as_mut().consume(h.payload_len as usize);
//                     header.as_mut().take();

//                     return Poll::Ready(Some(Ok(frame)));
//                 }
//             }
//         }
//     }
// }

// impl<'w, T: AsyncWrite + Send + Sync + Unpin> Sink<Frame> for FrameSink<'w, T> {
//     type Error = Error;

//     fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     fn start_send(self: Pin<&mut Self>, mut item: Frame) -> Result<(), Self::Error> {
//         let this = self.project();
//         let buf = this.buf;
//         // let mut writer = this.inner;

//         // check payload len
//         let payload_len = item.payload.len();
//         if payload_len > PayloadLen::MAX as usize {
//             return Err(Error::TransportError {
//                 msg: format!(
//                     "Payload length exceeded maximum. Max is {}, found {}",
//                     PayloadLen::MAX,
//                     payload_len
//                 ),
//             });
//         }

//         // write header
//         let header = FrameHeader::new(
//             item.message_id,
//             item.frame_id,
//             item.payload_type,
//             payload_len as u32,
//         );

//         buf.append(&mut header.to_vec()?);
//         buf.append(&mut item.payload);

//         Ok(())
//     }

//     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         let this = self.project();
//         let mut writer = this.inner;
//         let buf = this.buf;

//         futures::ready!(writer.as_mut().poll_write(cx, &buf)).map_err(|e| Error::IoError(e))?;
//         futures::ready!(writer.as_mut().poll_flush(cx)).map_err(|e| Error::IoError(e))?;

//         buf.clear();

//         Poll::Ready(Ok(()))
//     }

//     fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         let this = self.project();
//         let writer = this.inner;

//         writer.poll_close(cx).map_err(|e| e.into())
//     }
// }
