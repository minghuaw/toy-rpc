//! RPC Client impementation

use cfg_if::cfg_if;
use flume::{Sender};
use futures::{lock::Mutex, Future, channel::oneshot};
use pin_project::pin_project;
use std::{
    collections::HashMap,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::{
    message::{
        AtomicMessageId, ClientRequestBody, ClientResponseResult, MessageId, RequestHeader,
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
    cancel: oneshot::Sender<MessageId>,
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

    // new request will be sent over this channel
    requests: Sender<(RequestHeader, ClientRequestBody)>,

    // both reader and writer tasks should return nothingcliente handles will be used to drop the tasks
    // The Drop trait should be impled when tokio or async_std runtime is enabled
    reader_stop: Sender<()>,
    writer_stop: Sender<()>,

    marker: PhantomData<Mode>,
}

// seems like it still works even without this impl
impl<Mode> Drop for Client<Mode> {
    fn drop(&mut self) {
        // log::debug!("Dropping client");

        if self.reader_stop.send(()).is_err() {
            log::error!("Failed to send stop signal to reader loop")
        }
        if self.writer_stop.send(()).is_err() {
            log::error!("Failed to send stop signal to writer loop")
        }
    }
}


cfg_if! {
    if #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))] {
        use flume::Receiver;
        use futures::{FutureExt, select};
        
        use crate::codec::split::{ClientCodecRead, ClientCodecWrite};
        use crate::message::{ResponseHeader, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, Metadata};
    
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
                        match res {
                            Ok(_) => {}
                            Err(err) => log::error!("{:?}", err),
                        }
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
                        // .ok_or(Error::IoError(std::io::Error::new(
                        //     std::io::ErrorKind::UnexpectedEof,
                        //     "Unexpected EOF reading response body",
                        // )))?;
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
            requests: Receiver<(RequestHeader, ClientRequestBody)>,
            stop: Receiver<()>,
        ) {
            loop {
                select! {
                    _ = stop.recv_async().fuse() => {
                        // finish sending all requests available before dropping
                        for (header, body) in requests.drain().into_iter() {
                            match writer.write_request(header, &body).await {
                                Ok(_) => { },
                                Err(err) => log::error!("{:?}", err)
                            }
                        }
                        return 
                    },
                    res = write_once(&mut writer, &requests).fuse() => {
                        match res {
                            Ok(_) => {}
                            Err(err) => log::error!("{:?}", err),
                        }
                    }
                }
            }
        }
        
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        async fn write_once(
            writer: &mut impl ClientCodecWrite,
            request: &Receiver<(RequestHeader, ClientRequestBody)>,
        ) -> Result<(), Error> {
            if let Ok(req) = request.recv_async().await {
                let (header, body) = req;
                writer.write_request(header, &body).await?;
            }
            Ok(())
        }
        
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        async fn handle_call<Res>(
            pending: Arc<Mutex<ResponseMap>>,
            header: RequestHeader,
            body: ClientRequestBody,
            request_tx: Sender<(RequestHeader, ClientRequestBody)>,
            cancel: oneshot::Receiver<MessageId>,
            done: oneshot::Sender<Result<Res, Error>>,
        ) -> Result<(), Error>
        where
            Res: serde::de::DeserializeOwned + Send,
        {
            let id = header.get_id();
            request_tx.send_async((header, body)).await?;
        
            let (resp_tx, resp_rx) = oneshot::channel();
        
            // insert done channel to ResponseMap
            {
                let mut _pending = pending.lock().await;
                _pending.insert(id, resp_tx);
            }
        
            select! {
                res = cancel.fuse() => {
                    if let Ok(id) = res {
                        let header = RequestHeader {
                            id,
                            service_method: CANCELLATION_TOKEN.into(),
                        };
                        let body: String =
                            format!("{}{}{}", CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, id);
                        let body = Box::new(body) as ClientRequestBody;
                        request_tx.send_async((header, body)).await?;
                    }
                },
                res = handle_response(resp_rx, done).fuse() => {
                    match res {
                        Ok(_) => { },
                        Err(err) => log::error!("{:?}", err)
                    }
                }
            };
        
            Ok(())
        }
        
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        async fn handle_response<Res>(
            response: oneshot::Receiver<ClientResponseResult>,
            done: oneshot::Sender<Result<Res, Error>>,
        ) -> Result<(), Error>
        where
            Res: serde::de::DeserializeOwned + Send,
        {
            let val = response
                .await
                // cancellation of the oneshot channel is not intended
                // and thus should be considered as an InternalError
                .map_err(|err| Error::Internal(Box::new(err)))?;
            let res = match val {
                Ok(mut resp_body) => erased_serde::deserialize(&mut resp_body)
                    .map_err(|err| Error::ParseError(Box::new(err))),
                Err(mut err_body) => erased_serde::deserialize(&mut err_body).map_or_else(
                    |err| Err(Error::ParseError(Box::new(err))),
                    |msg| Err(Error::from_err_msg(msg)),
                ), // handles error msg sent from server
            };
        
            done.send(res)
                .map_err(|_| Error::Internal("Failed to send over done channel".into()))?;
            Ok(())
        }
    }
}

