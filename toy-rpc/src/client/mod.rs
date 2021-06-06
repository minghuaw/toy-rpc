//! RPC Client impementation

use cfg_if::cfg_if;
use flume::{Sender};
use futures::{Future, channel::oneshot, lock::Mutex};
use pin_project::pin_project;
use std::{collections::HashMap, marker::PhantomData, pin::Pin, sync::{Arc, atomic::Ordering}, task::{Context, Poll}, time::Duration};
use crossbeam::atomic::AtomicCell;

use crate::{Error, codec::split::SplittableClientCodec, message::{
        AtomicMessageId, ClientRequestBody, ClientResponseResult, MessageId, RequestHeader,
        ClientMessage,
    }};
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

impl Client<NotConnected> {
    /// Creates an RPC 'Client` over socket with a specified codec
    ///
    /// Example
    ///
    /// ```rust
    /// let addr = "127.0.0.1:8080";
    /// let stream = TcpStream::connect(addr).await.unwrap();
    /// let codec = Codec::new(stream);
    /// let client = Client::with_codec(codec);
    /// ```
    #[cfg(any(
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))]
    pub fn with_codec<C>(codec: C) -> Client<Connected>
    where
        C: SplittableClientCodec + Send + Sync + 'static,
    {
        let (writer, reader) = codec.split();
        let (writer_tx, writer_rx) = flume::unbounded();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (reader_stop, stop) = flume::bounded(1);
        
        #[cfg(all(
            feature = "async_std_runtime",
            not(feature = "tokio_runtime")
        ))]
        {
            ::async_std::task::spawn(reader_loop(reader, pending.clone(), stop));
            ::async_std::task::spawn(writer_loop(writer, writer_rx));
        }

        #[cfg(all(
            feature = "tokio_runtime",
            not(feature = "async_std_runtime")
        ))]
        {
            ::tokio::task::spawn(reader_loop(reader, pending.clone(), stop));
            ::tokio::task::spawn(writer_loop(writer, writer_rx));
        }

        Client::<Connected> {
            count: AtomicMessageId::new(0),
            pending,
            timeout: AtomicCell::new(None),
            reader_stop,
            writer_tx,

            marker: PhantomData,
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

    /// Invokes the named function and wait synchronously in a blocking manner.
    ///
    /// This function internally calls `task::block_on` to wait for the response.
    /// Do NOT use this function inside another `task::block_on`.async_std
    ///
    /// Example
    ///
    /// ```rust
    /// let args = "arguments";
    /// let reply: Result<String, Error> = client.call("EchoService.echo", &args);
    /// println!("{:?}", reply);
    /// ```
    #[cfg(any(
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))]
    pub fn call_blocking<Req, Res>(
        &self,
        service_method: impl ToString,
        args: Req,
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static,
    {
        let call = self.call(service_method, args);

        #[cfg(all(
            feature = "async_std_runtime",
            not(feature = "tokio_runtime")
        ))]
        let res = ::async_std::task::block_on(call);


        #[cfg(all(
            feature = "tokio_runtime",
            not(feature = "async_std_runtime")
        ))]
        let res = ::tokio::task::block_in_place(|| {
            futures::executor::block_on(call)
        });

        res
    }


    /// Invokes the named RPC function call asynchronously and returns a cancellation `Call`
    /// 
    /// The `Call<Res>` type takes one type argument `Res` which is the type of successful RPC execution.
    /// The result can be obtained by `.await`ing on the `Call`, which returns type `Result<Res, toy_rpc::Error>`
    /// `Call` can be cancelled by calling the `cancel()` function. 
    /// The request will be sent in a background task. 
    /// 
    /// Example 
    /// 
    /// ```rust
    /// // Get the result by `.await`ing on the `Call`
    /// let call: Call<i32> = client.call("SomeService.echo_i32", 7i32);
    /// let reply: Result<i32, toy_rpc::Error> = call.await;
    /// 
    /// // Cancel the call
    /// let call: Call<()> = client.call("SomeService.infinite_loop", ());
    /// call.cancel();
    /// ```
    #[cfg(any(
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))]
    pub fn call<Req, Res>(&self, service_method: impl ToString, args: Req) -> Call<Res>
    where
        Req: serde::Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static,
    {
        // Prepare RPC request
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        let service_method = service_method.to_string();
        let header = RequestHeader { id, service_method };
        let body = Box::new(args) as ClientRequestBody;
        let timeout = self.timeout.take();

        // Prepare response handler
        let (done_tx, done_rx) = oneshot::channel();
        let (cancel_tx, cancel_rx) = flume::bounded(1);
        let pending = self.pending.clone();
        let writer_tx = self.writer_tx.clone();

        #[cfg(all(
            feature = "async_std_runtime",
            not(feature = "tokio_runtime")
        ))]
        let handle = ::async_std::task::spawn(
            handle_call(pending, header, body, writer_tx, cancel_rx, done_tx, timeout)
        );

        #[cfg(all(
            feature = "tokio_runtime",
            not(feature = "async_std_runtime")
        ))]
        let handle = ::tokio::task::spawn(
            handle_call(pending, header, body, writer_tx, cancel_rx, done_tx, timeout)
        );

        // Creates Call
        Call::<Res> {
            id,
            cancel: cancel_tx,
            done: done_rx,
            handle: Box::new(handle)
        }
    }
}

cfg_if! {
    if #[cfg(any(
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))] {
        use flume::Receiver;
        use futures::{FutureExt, select};
        
        use crate::codec::split::{ClientCodecRead, ClientCodecWrite};
        use crate::message::{ResponseHeader, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, 
            Metadata, TIMEOUT_TOKEN, TimeoutRequestBody};
    
        // #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
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
        
        // #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
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
        
        // #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
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
        
        // #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
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

            // Send a timeout request first if timeout is set
            if let Some(duration) = &timeout {
                writer_tx.send_async(
                    ClientMessage::Timeout(id, duration.clone())
                ).await?;
            }
            // Then send out the RPC request itself
            writer_tx.send_async(
                ClientMessage::Request(header, body)
            ).await?;
        
            // Done channels that .await for response
            let (resp_tx, resp_rx) = oneshot::channel();
            // insert done channel to ResponseMap
            {
                let mut _pending = pending.lock().await;
                _pending.insert(id, resp_tx);
            }
        
            // .await both the cancellation and RPC response
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
        
        // #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
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

