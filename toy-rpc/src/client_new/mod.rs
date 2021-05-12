use std::{collections::HashMap, marker::PhantomData, pin::Pin, sync::Arc, task::{Context, Poll}};
use flume::Receiver;
use pin_project::pin_project;
use futures::{Future, FutureExt, channel::oneshot, lock::Mutex};
use cfg_if::cfg_if;

use crate::{Error, codec::ClientCodec, message::{AtomicMessageId, MessageId, RequestHeader, RequestMessage, ResponseHeader}};

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

cfg_if!{
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

/// The serialized representation of the response body
type RequestBody = Box<dyn serde::Serialize + Send + Sync>;
type ResponseBody = Box<dyn erased_serde::Deserializer<'static> + Send>;
type ResponseResult = Result<ResponseBody, ResponseBody>;

type Codec = Arc<Mutex<Box<dyn ClientCodec>>>;
type ResponseMap = HashMap<MessageId, oneshot::Sender<ResponseResult>>;

/// Call of a RPC request. The result can be obtained by `.await`ing the `Call`.
/// The call can be cancelled with `cancel()` method.
/// 
/// # Example
/// 
#[pin_project]
pub struct Call<Res> {
    cancel: oneshot::Sender<()>,
    #[pin]
    done: oneshot::Receiver<Result<ResponseBody, ResponseBody>>,
    
    // return type
    marker: PhantomData<Res>
}

impl<Res> Call<Res> 
where 
    Res: serde::de::DeserializeOwned
{
    pub fn cancel(self) {
        unimplemented!()
    }
}

impl<Res> Future for Call<Res> 
where 
    Res: serde::de::DeserializeOwned
{
    type Output = Result<Res, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let done: Pin<&mut oneshot::Receiver<Result<ResponseBody, ResponseBody>>> = this.done;
        // let done = Pin::new(&mut self.done);

        match done.poll(cx) {
            Poll::Ready(result) => {
                match result {
                    Ok(val) => {
                        match val {
                            Ok(mut resp_body) => {
                                let resp = erased_serde::deserialize(&mut resp_body)
                                    .map_err(|e| Error::ParseError(Box::new(e)));
                                Poll::Ready(resp)
                            },
                            Err(mut err_body) => {
                                let err = erased_serde::deserialize(&mut err_body)
                                    .map_or_else(
                                        |e| Err(Error::ParseError(Box::new(e))), 
                                        |msg| Err(Error::from_err_msg(msg))
                                    );
                                Poll::Ready(err)
                            }
                        }
                    },
                    Err(canceled) => {
                        let err = Err(Error::Internal(
                            format!("Failed to read from done channel: {:?}", canceled).into()
                        ));
                        Poll::Ready(err)
                    }
                }
            },
            Poll::Pending => {
                Poll::Pending
            }
        }
    }
}

/// RPC client
///
pub struct Client<Mode> {
    count: AtomicMessageId,
    inner_codec: Codec,
    pending: Arc<Mutex<ResponseMap>>,

    marker: PhantomData<Mode>,
}

impl Client<Connected> {
    async fn reader_loop(req: Receiver<RequestMessage>) {
        unimplemented!()
    }
}

pub(crate) async fn serialize_and_send_request<Req>(
    service_method: impl ToString,
    args: &Req,
    id: MessageId,
    codec: &mut Box<dyn ClientCodec>,
) -> Result<(), Error>
where 
    Req: serde::Serialize + Send + Sync
{
    let header = RequestHeader {
        id, 
        service_method: service_method.to_string()
    };
    let req = &args as &(dyn erased_serde::Serialize + Send + Sync);
    codec.write_request(header, req).await
}

pub(crate) async fn read_response(
    codec: Codec,
    pending: Arc<Mutex<ResponseMap>>
) -> Result<(), Error> {
    let mut codec = codec.lock().await;
    // wait for response
    if let Some(header) = codec.read_response_header().await {
        // [1] destructure header
        let ResponseHeader {id, is_error } = header?;
        // [2] get resposne body
        let deserialzer = codec.read_response_body().await
            .ok_or(Error::IoError(
                std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Unexpected EOF reading response body",
                )
            ))?;
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

pub(crate) async fn execute_call<Req>(
    codec: Codec,
    service_method: impl ToString,
    args: Req,
    pending: Arc<Mutex<ResponseMap>>,
    id: MessageId,
    cancel: Receiver<()>,
) -> Result<(), Error>
where 
    Req: serde::Serialize + Send + Sync,
{
    let (resp_body_tx, resp_body_rx) = oneshot::channel();
    {
        let mut _pending = pending.lock().await;
        _pending.insert(id, resp_body_tx);
    }

    {
        let mut _codec = &mut *codec.lock().await;
        serialize_and_send_request(service_method, &args, id, _codec).await?;
        
    }
    // let read_response_fut = read_response(_codec, pending);
    // let res = futures::select! {
    //     _ = cancel.fuse() => Err(Error::Canceled),
    //     read_res = read_response_fut.fuse() => read_res,
    // };

    // spawn reading task


    Ok(())
}