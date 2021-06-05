//! RPC Client impementation

use cfg_if::cfg_if;
use flume::{Sender};
use futures::{Future, TryFutureExt, channel::oneshot, lock::Mutex};
use pin_project::pin_project;
use std::{collections::HashMap, marker::PhantomData, pin::Pin, sync::Arc, task::{Context, Poll}, time::Duration};
use crossbeam::atomic::AtomicCell;

use crate::{
    message::{
        AtomicMessageId, ClientRequestBody, ClientResponseResult, MessageId, RequestHeader,
        ClientMessage,
    },
    Error,
};
use crate::util::Terminate;

cfg_if! {
    if #[cfg(any(
        all(
            feature = "serde_bincode",
            not(feature = "serde_json"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_cbor",
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_json",
            not(feature = "serde_bincode"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_rmp",
            not(feature = "serde_cbor"),
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
        )
    ))] {
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        use crate::codec::DefaultCodec;
    }
}

cfg_if! {
    if #[cfg(feature = "async_std_runtime")] {
        mod async_std;
    } else if #[cfg(feature = "tokio_runtime")] {
        mod tokio;
    }
}

/// Call of a RPC request. The result can be obtained by `.await`ing the `Call`.
/// The call can be cancelled with `cancel()` method.
///
/// # Example
///
#[pin_project]
pub struct Call<Res> {
    id: MessageId,
    // cancel: oneshot::Sender<MessageId>,
    cancel: Sender<MessageId>,
    #[pin]
    done: oneshot::Receiver<Result<Res, Error>>,
    handle: Box<dyn Terminate + Send + Sync>,
}

impl<Res> Call<Res>
where
    Res: serde::de::DeserializeOwned,
{
    /// Cancel the RPC call

    pub fn cancel(self) {
        let mut handle = self.handle;
        match self.cancel.send(self.id) {
            Ok(_) => {
                log::info!("Call is canceled");
            }
            Err(_) => {
                log::error!("Failed to cancel")
            }
        }

        handle.terminate();
    }
}

impl<Res> Future for Call<Res>
where
    Res: serde::de::DeserializeOwned,
{
    type Output = Result<Res, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let done: Pin<&mut oneshot::Receiver<Result<Res, Error>>> = this.done;

        match done.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => match res {
                Ok(r) => Poll::Ready(r),
                Err(_canceled) => Poll::Ready(Err(Error::Canceled(Some(*this.id)))),
            },
        }
    }
}

/// Type state for creating `Client`
pub struct NotConnected {}
/// Type state for creating `Client`
pub struct Connected {}

// There will be a dedicated task for reading and writing, so there should be no
// contention across tasks or threads
// type Codec = Box<dyn ClientCodec>;
type ResponseMap = HashMap<MessageId, oneshot::Sender<ClientResponseResult>>;

/// RPC client
///
#[cfg_attr(not(any(feature = "async_std_runtime", feature = "tokio_runtime")), allow(dead_code))]
pub struct Client<Mode> {
    count: AtomicMessageId,
    pending: Arc<Mutex<ResponseMap>>,
    // timeout: Mutex<Option<Duration>>,
    timeout: AtomicCell<Option<Duration>>,

    // both reader and writer tasks should return nothing cliente handles will be used to drop the tasks
    // The Drop trait should be impled when tokio or async_std runtime is enabled
    reader_stop: Sender<()>,
    writer_tx: Sender<ClientMessage>,

    marker: PhantomData<Mode>,
}

// seems like it still works even without this impl
impl<Mode> Drop for Client<Mode> {
    fn drop(&mut self) {
        // log::debug!("Dropping client");

        if self.reader_stop.send(()).is_err() {
            log::error!("Failed to send stop signal to reader loop")
        }
        if self.writer_tx.send(ClientMessage::Stop).is_err() {
            log::error!("Failed to send stop signal to writer loop")
        }
    }
}

impl Client<Connected> {
    /// Sets the timeout duration **ONLY** for the next RPC request
    ///
    /// Example
    /// 
    /// ```rust 
    ///
    /// ```
    pub fn timeout(&self, duration: Duration) -> &Self {
        self.timeout.store(Some(duration));
        &self
    }
}

cfg_if! {
    if #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))] {
        use flume::Receiver;
        use futures::{FutureExt, select};
        
        use crate::codec::split::{ClientCodecRead, ClientCodecWrite};
        use crate::message::{ResponseHeader, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, 
            Metadata, TIMEOUT_TOKEN, TimeoutRequestBody};
    
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        pub(crate) async fn reader_loop(
            mut reader: impl ClientCodecRead,
            pending: Arc<Mutex<ResponseMap>>,
            stop: Receiver<()>,
        ) {
            loop {
                select! {
                    _ = stop.recv_async().fuse() => {
                        return 
                    },
                    res = read_once(&mut reader, &pending).fuse() => {
                        res.unwrap_or_else(|err| 
                            log::error!("{:?}", err)
                        )
                    }
                }
            }
        }
        
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        async fn read_once(
            reader: &mut impl ClientCodecRead,
            pending: &Arc<Mutex<ResponseMap>>,
        ) -> Result<(), Error> {
            if let Some(header) = reader.read_response_header().await {
                // [1] destructure header
                let ResponseHeader { id, is_error } = header?;
                // [2] get resposne body
                let deserialzer =
                    reader
                        .read_response_body()
                        .await
                        .ok_or_else(|| Error::IoError(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Unexpected EOF reading response body",
                        )))?;
                let deserializer = deserialzer?;
        
                let res = match is_error {
                    false => Ok(deserializer),
                    true => Err(deserializer),
                };
        
                // [3] send back response
                {
                    let mut _pending = pending.lock().await;
                    if let Some(done_sender) = _pending.remove(&id) {
                        done_sender.send(res).map_err(|_| {
                            Error::Internal(
                                "InternalError: client failed to send response over channel".into(),
                            )
                        })?;
                    }
                }
            }
            Ok(())
        }
        
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        pub(crate) async fn writer_loop(
            mut writer: impl ClientCodecWrite,
            msgs: Receiver<ClientMessage>,
        ) {
            while let Ok(msg) = msgs.recv_async().await {
                match msg {
                    ClientMessage::Timeout(id, dur) => {
                        let timeout_header = RequestHeader {
                            id,
                            service_method: TIMEOUT_TOKEN.into()
                        };
                        let timeout_body = Box::new(
                            TimeoutRequestBody::new(dur)
                        ) as ClientRequestBody;
                        writer.write_request(timeout_header, &timeout_body).await
                    },
                    ClientMessage::Request(header, body) => {
                        writer.write_request(header, &body).await  
                    },
                    ClientMessage::Cancel(id) => {
                        let header = RequestHeader {
                            id,
                            service_method: CANCELLATION_TOKEN.into(),
                        };
                        let body: String =
                            format!("{}{}{}", CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, id);
                        let body = Box::new(body) as ClientRequestBody;
                        writer.write_request(header, &body).await  
                    },
                    ClientMessage::Stop => {
                        return
                    }     
                }
                .unwrap_or_else(|err| {
                    log::error!("{:?}", err)
                })
            }            
        }
        
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        async fn handle_call<Res>(
            pending: Arc<Mutex<ResponseMap>>,
            header: RequestHeader,
            body: ClientRequestBody,
            writer_tx: Sender<ClientMessage>,
            cancel: Receiver<MessageId>,
            done: oneshot::Sender<Result<Res, Error>>,
            timeout: Option<Duration>,
        ) -> Result<(), Error>
        where
            Res: serde::de::DeserializeOwned + Send,
        {
            let id = header.get_id();
            writer_tx.send_async(
                ClientMessage::Request(header, body)
            ).await?;
        
            let (resp_tx, resp_rx) = oneshot::channel();
        
            // insert done channel to ResponseMap
            {
                let mut _pending = pending.lock().await;
                _pending.insert(id, resp_tx);
            }
        
            select! {
                res = cancel.recv_async().fuse() => {
                    if let Ok(id) = res {
                        writer_tx.send_async(
                            ClientMessage::Cancel(id)
                        ).await?;
                    }
                },
                res = handle_response::<Res>(resp_rx, timeout).fuse() => { 
                    done.send(res)
                        .unwrap_or_else(|_| {
                            log::error!("Failed to send result over done channel")
                        });
                }
            };
        
            Ok(())
        }
        
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        async fn handle_response<Res>(
            response: oneshot::Receiver<ClientResponseResult>,
            // done: oneshot::Sender<Result<Res, Error>>,
            timeout: Option<Duration>,
        ) -> Result<Res, Error>
        where
            Res: serde::de::DeserializeOwned + Send,
        {
            let val = match timeout {
                None => {
                    response.await
                        .map_err(|err| Error::Internal(Box::new(err)))?
                }
                Some(duration) => {
                    #[cfg(all(
                        feature = "async_std_runtime",
                        not(feature = "tokio_runtime")
                    ))]
                    match ::async_std::future::timeout(duration, async {
                        response.await
                            .map_err(|err| Error::Internal(Box::new(err)))
                    }).await {
                        Ok(res) => res?,
                        Err(_) => {
                            // No need to deserialize if already timed out
                            return Err(Error::Timeout(None))
                        }
                    }

                    #[cfg(all(
                        feature = "tokio_runtime",
                        not(feature = "async_std_runtime")
                    ))]
                    match ::tokio::time::timeout(duration, async {
                        response.await
                            .map_err(|err| Error::Internal(Box::new(err)))
                    }).await {
                        Ok(res) => res?,
                        Err(_) => {
                            // No need to deserialize if already timed out
                            return Err(Error::Timeout(None))
                        }
                    }
                }
            };

            let res = match val {
                Ok(mut resp_body) => erased_serde::deserialize(&mut resp_body)
                    .map_err(|err| Error::ParseError(Box::new(err))),
                Err(mut err_body) => erased_serde::deserialize(&mut err_body).map_or_else(
                    |err| Err(Error::ParseError(Box::new(err))),
                    |msg| Err(Error::from_err_msg(msg)),
                ), // handles error msg sent from server
            };
        
            // done.send(res)
            //     .map_err(|_| Error::Internal("Failed to send over done channel".into()))?;
            // Ok(())
            res
        }
    }
}

