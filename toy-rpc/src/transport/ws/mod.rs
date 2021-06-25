//! WebSocket transport support

use async_trait::async_trait;
use cfg_if::cfg_if;
use futures::stream::{SplitSink, SplitStream};
use futures::{Sink, SinkExt, Stream, StreamExt};
use tungstenite::Message as WsMessage;

use std::{io::ErrorKind, marker::PhantomData};

use super::{PayloadRead, PayloadWrite};
use crate::{error::Error, util::GracefulShutdown};

type WsSinkHalf<S> = SinkHalf<SplitSink<S, WsMessage>, CanSink>;
type WsStreamHalf<S> = StreamHalf<SplitStream<S>, CanSink>;

cfg_if! {
    if #[cfg(feature = "http_tide")] {
        pub(crate) struct CannotSink {}
        mod tide_ws;
    } else if #[cfg(feature = "http_warp")] {
        mod warp_ws;
    }
}
pub(crate) struct CanSink {}

pub struct WebSocketConn<S, N> {
    pub inner: S,
    can_sink: PhantomData<N>,
}

pub struct StreamHalf<S, Mode> {
    inner: S,
    can_sink: PhantomData<Mode>,
}

pub struct SinkHalf<S, Mode> {
    inner: S,
    can_sink: PhantomData<Mode>,
}

impl<S, E> WebSocketConn<S, CanSink>
where
    S: Stream<Item = Result<WsMessage, E>> + Sink<WsMessage> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            can_sink: PhantomData,
        }
    }

    pub fn split(self) -> (WsSinkHalf<S>, WsStreamHalf<S>) {
        let (writer, reader) = self.inner.split();

        let readhalf = StreamHalf {
            inner: reader,
            can_sink: PhantomData,
        };
        let writehalf = SinkHalf {
            inner: writer,
            can_sink: PhantomData,
        };
        (writehalf, readhalf)
    }
}

#[async_trait]
impl<S, E> PayloadRead for StreamHalf<S, CanSink>
where
    S: Stream<Item = Result<WsMessage, E>> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        match self.inner.next().await? {
            Err(e) => {
                return Some(Err(Error::IoError(std::io::Error::new(
                    ErrorKind::InvalidData,
                    e.to_string(),
                ))))
            }
            Ok(msg) => {
                if let WsMessage::Binary(bytes) = msg {
                    return Some(Ok(bytes));
                } else if let WsMessage::Close(_) = msg {
                    return None;
                }

                Some(Err(Error::IoError(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Expecting WebSocket::Message::Binary",
                ))))
            }
        }
    }
}

#[async_trait]
impl<S, E> PayloadWrite for SinkHalf<S, CanSink>
where
    S: Sink<WsMessage, Error = E> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
        let msg = WsMessage::Binary(payload);

        self.inner
            .send(msg)
            .await
            .map_err(|e| Error::IoError(std::io::Error::new(ErrorKind::InvalidData, e.to_string())))
    }
}

// GracefulShutdown is only required on the client side.
#[async_trait]
impl<S, E> GracefulShutdown for SinkHalf<S, CanSink>
where
    S: Sink<WsMessage, Error = E> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    async fn close(&mut self) {
        let msg = WsMessage::Close(None);

        match self
            .inner
            .send(msg)
            .await
            .map_err(|e| Error::IoError(std::io::Error::new(ErrorKind::InvalidData, e.to_string())))
        {
            Ok(()) => {}
            Err(e) => log::error!("Error closing WebSocket {}", e.to_string()),
        };
    }
}
