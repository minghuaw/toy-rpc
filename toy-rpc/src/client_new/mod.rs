use cfg_if::cfg_if;
use flume::{Receiver, Sender};
use futures::{channel::oneshot, lock::Mutex, Future, FutureExt};
use pin_project::pin_project;
use std::{
    collections::HashMap,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::message::CANCELLATION_TOKEN;
use crate::{
    codec::split::{ClientCodecRead, ClientCodecWrite},
    message::{
        AtomicMessageId, MessageId, RequestBody, RequestHeader, ResponseHeader, ResponseResult,
        CANCELLATION_TOKEN_DELIM,
    },
    util::TerminateTask,
    Error,
};

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
        use crate::codec::DefaultCodec;
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "async_std_runtime",
        feature = "http_tide"
    ))] {
        mod async_std;
    } else {
        mod tokio;
    }
}

/// Type state for creating `Client`
pub struct NotConnected {}
/// Type state for creating `Client`
pub struct Connected {}

// There will be a dedicated task for reading and writing, so there should be no
// contention across tasks or threads
// type Codec = Box<dyn ClientCodec>;
type ResponseMap = HashMap<MessageId, oneshot::Sender<ResponseResult>>;

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
}

impl<Res> Call<Res>
where
    Res: serde::de::DeserializeOwned,
{
    pub fn cancel(self) {
        match self.cancel.send(self.id) {
            Ok(_) => {
                log::info!("Call is canceled");
            }
            Err(_) => {
                log::error!("Failed to cancel")
            }
        }
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
                Err(_canceled) => Poll::Ready(Err(Error::Canceled(Some(this.id.clone())))),
            },
        }
    }
}

/// RPC client
///
pub struct Client<Mode, Handle: TerminateTask> {
    count: AtomicMessageId,
    pending: Arc<Mutex<ResponseMap>>,

    // new request will be sent over this channel
    requests: Sender<(RequestHeader, RequestBody)>,

    // both reader and writer tasks should return nothing
    // The handles will be used to drop the tasks
    // The Drop trait should be impled when tokio or async_std runtime is enabled
    reader_handle: Option<Handle>,
    writer_handle: Option<Handle>,

    marker: PhantomData<Mode>,
}

pub(crate) async fn reader_loop(
    mut reader: impl ClientCodecRead,
    pending: Arc<Mutex<ResponseMap>>,
) {
    loop {
        match read_once(&mut reader, &pending).await {
            Ok(_) => {}
            Err(err) => log::error!("{:?}", err),
        }
    }
}

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
                .ok_or(Error::IoError(std::io::Error::new(
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

pub(crate) async fn writer_loop(
    mut writer: impl ClientCodecWrite,
    requests: Receiver<(RequestHeader, RequestBody)>,
) {
    loop {
        match write_once(&mut writer, &requests).await {
            Ok(_) => {}
            Err(err) => log::error!("{:?}", err),
        }
    }
}

async fn write_once(
    writer: &mut impl ClientCodecWrite,
    request: &Receiver<(RequestHeader, RequestBody)>,
) -> Result<(), Error> {
    if let Ok(req) = request.recv_async().await {
        let (header, body) = req;
        writer.write_request(header, &body).await?;
    }
    Ok(())
}

async fn handle_call<Res>(
    pending: Arc<Mutex<ResponseMap>>,
    header: RequestHeader,
    body: RequestBody,
    request_tx: Sender<(RequestHeader, RequestBody)>,
    cancel: oneshot::Receiver<MessageId>,
    done: oneshot::Sender<Result<Res, Error>>,
) -> Result<(), Error>
where
    Res: serde::de::DeserializeOwned + Send,
{
    let id = header.id.clone();
    request_tx.send_async((header, body)).await?;

    let (resp_tx, resp_rx) = oneshot::channel();

    // insert done channel to ResponseMap
    {
        let mut _pending = pending.lock().await;
        _pending.insert(id, resp_tx);
    }

    handle_response(request_tx, cancel, resp_rx, done).await?;

    Ok(())
}

async fn handle_response<Res>(
    request: Sender<(RequestHeader, RequestBody)>,
    cancel: oneshot::Receiver<MessageId>,
    response: oneshot::Receiver<ResponseResult>,
    done: oneshot::Sender<Result<Res, Error>>,
) -> Result<(), Error>
where
    Res: serde::de::DeserializeOwned + Send,
{
    let val: Result<ResponseResult, Error> = futures::select! {
        cancel_res = cancel.fuse() => {
            match cancel_res {
                Ok(id) => Err(Error::Canceled(Some(id))),
                Err(_) => Err(Error::Canceled(None))
            }
        },
        resp_res = response.fuse() => resp_res.map_err(|err| Error::Internal(
            format!("InternalError: {:?}", err).into()
        ))
    };

    let res = match val {
        Ok(result) => match result {
            Ok(mut resp_body) => erased_serde::deserialize(&mut resp_body)
                .map_err(|e| Error::ParseError(Box::new(e))),
            Err(mut err_body) => erased_serde::deserialize(&mut err_body).map_or_else(
                |e| Err(Error::ParseError(Box::new(e))),
                |msg| Err(Error::from_err_msg(msg)),
            ),
        },
        Err(err) => {
            if let &Error::Canceled(opt) = &err {
                // send a cancel request to server only if the
                // the request id is successully received
                if let Some(id) = opt {
                    let header = RequestHeader {
                        id,
                        service_method: CANCELLATION_TOKEN.into(),
                    };
                    let body: String =
                        format!("{}{}{}", CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, id);
                    let body = Box::new(body) as RequestBody;
                    request.send_async((header, body)).await?;
                }
            }

            Err(err)
        }
    };

    done.send(res)
        .map_err(|_| Error::Internal("InternalError: Failed to send over done channel".into()))?;

    Ok(())
}
