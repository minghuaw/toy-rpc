use std::sync::atomic::Ordering;

use ::async_std::task;
use futures::{AsyncRead, AsyncWrite};
use async_trait::async_trait;

use crate::codec::split::ClientCodecSplit;

use super::*;

impl Client<NotConnected, task::JoinHandle<()>> {
    pub fn with_codec<C>(codec: C) -> Client<Connected, task::JoinHandle<()>> 
    where 
        C: ClientCodecSplit + Send + Sync + 'static
    {
        // let codec: Box<dyn ClientCodec> = Box::new(codec);
        let (writer, reader) = codec.split();
        let (req_sender, req_recver) = flume::unbounded();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let handle = task::spawn(reader_loop(reader, pending.clone()));
        let reader_handle = Some(handle);
        let handle = task::spawn(writer_loop(writer, req_recver));
        let writer_handle = Some(handle);

        Client::<Connected, task::JoinHandle<()>> {
            count: AtomicMessageId::new(0),
            pending,
            requests: req_sender,
            reader_handle,
            writer_handle,

            marker: PhantomData
        }
    }
}

impl<Mode, Handle: TerminateTask> Drop for Client<Mode, Handle> {
    fn drop(&mut self) {
        log::debug!("Dropping client");

        self.reader_handle.take()
            .map(|h| h.terminate());
        self.writer_handle.take()
            .map(|h| h.terminate());
    }
}

impl Client<Connected, task::JoinHandle<()>> {
    fn call_blocking<Req, Res>(&self, service_method: impl ToString, args: Req) -> Result<Res, Error>
    where
            Req: serde::Serialize + Send + Sync,
            Res: serde::de::DeserializeOwned + Send, {
        unimplemented!()
    }

    async fn call<Req, Res>(&self, service_method: impl ToString, args: Req) -> Result<Call<Res>, Error>
    where
            Req: serde::Serialize + Send + Sync + 'static,
            Res: serde::de::DeserializeOwned + Send + 'static, 
    {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        let service_method = service_method.to_string();
        let header = RequestHeader { id, service_method };
        let body = Box::new(args) as RequestBody;

        // send request to writer
        self.requests.send_async(
            (header, body)
        ).await?;

        // create oneshot channel
        let (resp_tx, resp_rx) = oneshot::channel();
        let (done_tx, done_rx) = oneshot::channel();
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        
        // insert done channel to ResponseMap
        {
            let mut _pending = self.pending.lock().await;
            _pending.insert(id, resp_tx);
        }

        // spawn a task to handle response
        let request_tx = self.requests.clone();
        task::spawn(async move {
            match handle_response::<Res>(request_tx, cancel_rx, resp_rx, done_tx).await {
                Ok(_) => { },
                Err(err) => {
                    log::error!("{:?}", err);
                }
            }
        });

        // create Call
        let call = Call::<Res> {
            cancel: cancel_tx,
            done: done_rx
        };
        Ok(call)
    }
}