//! WebSocket transport support

use async_trait::async_trait;
use async_tungstenite::WebSocketStream;
use cfg_if::cfg_if;
use futures::io::{AsyncRead, AsyncWrite};
use futures::stream::{SplitSink, SplitStream};
use futures::{Sink, SinkExt, Stream, StreamExt};
use pin_project::pin_project;
use tungstenite::Message;

use std::{io::ErrorKind, marker::PhantomData};

use super::{PayloadRead, PayloadWrite};
use crate::{error::Error, util::GracefulShutdown};

type WsSinkHalf<S> = SinkHalf<SplitSink<S, Message>, CanSink>;
type WsStreamHalf<S> = StreamHalf<SplitStream<S>, CanSink>;

cfg_if! {
    if #[cfg(feature = "http_tide")] {
        pub(crate) struct CannotSink {}
        mod tide_ws;
    } else if #[cfg(feature = "http_warp")] {
        mod warp_ws;
    } else if #[cfg(feature = "http_axum")] {
        mod axum_ws;
    }
}
pub(crate) struct CanSink {}

pub struct WebSocketConn<S, N> {
    pub inner: S,
    can_sink: PhantomData<N>,
}

/// A wrapper around a type that impls Stream
#[pin_project]
pub struct StreamHalf<S, Mode> {
    #[pin]
    pub inner: S,
    pub can_sink: PhantomData<Mode>,
}

impl<S: Stream> Stream for StreamHalf<S, CanSink> {
    type Item = S::Item;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        this.inner.poll_next(cx)
    }
}

/// A wrapper around a type that impls Sink
#[pin_project]
pub struct SinkHalf<S, Mode> {
    #[pin]
    pub inner: S,
    pub can_sink: PhantomData<Mode>,
}

impl<S: Sink<Item>, Item> Sink<Item> for SinkHalf<S, CanSink> {
    type Error = S::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_ready(cx)
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        this.inner.start_send(item)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_close(cx)
    }
}

impl<S, E> WebSocketConn<S, CanSink>
where
    S: Stream<Item = Result<Message, E>> + Sink<Message> + Send + Sync + Unpin,
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
impl<T> PayloadRead for StreamHalf<SplitStream<WebSocketStream<T>>, CanSink>
where
    T: AsyncRead + AsyncWrite + Send + Unpin,
{
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        match self.next().await? {
            Err(e) => {
                return Some(Err(Error::IoError(std::io::Error::new(
                    ErrorKind::InvalidData,
                    e.to_string(),
                ))))
            }
            Ok(msg) => {
                if let Message::Binary(bytes) = msg {
                    return Some(Ok(bytes));
                } else if let Message::Close(_) = msg {
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
impl<T> PayloadWrite for SinkHalf<SplitSink<WebSocketStream<T>, Message>, CanSink>
where
    T: AsyncRead + AsyncWrite + Send + Unpin,
{
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), Error> {
        let msg = Message::Binary(payload.to_owned());

        self.send(msg)
            .await
            .map_err(|e| Error::Internal(Box::new(e)))
    }
}

// GracefulShutdown is only required on the client side.
#[async_trait]
impl<T> GracefulShutdown for SinkHalf<SplitSink<WebSocketStream<T>, Message>, CanSink>
where
    T: AsyncRead + AsyncWrite + Send + Unpin,
{
    async fn close(&mut self) {
        let msg = Message::Close(None);

        if let Err(err) = self.send(msg).await {
            match err {
                tungstenite::Error::ConnectionClosed => { },
                tungstenite::Error::AlreadyClosed => { },
                e @ _ => {
                    log::error!("{:?}", e)
                }
            }
        }
    }
}
