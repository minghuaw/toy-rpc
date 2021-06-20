use cfg_if::cfg_if;
use std::{time::Duration};

use futures::{channel::oneshot};

cfg_if!{
    if #[cfg(any(
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
    ))] {
        use std::{sync::Arc, collections::HashMap};
        use brw::{util::Conclude, Context, Running};
        use futures::{Sink, SinkExt};
        
        use super::{writer::ClientWriterItem};
    }
}

use crate::{Error, message::{ClientRequestBody, ClientResponseResult, MessageId, RequestHeader}};


#[cfg_attr(all(not(feature = "tokio_runtime"), not(feature = "async_std_runtime")), allow(dead_code))]
pub enum ClientBrokerItem {
    SetTimeout(Duration),
    Request{
        header: RequestHeader, 
        body: ClientRequestBody, 
        resp_tx: oneshot::Sender<Result<ClientResponseResult, Error>>,
    },
    Response(MessageId, ClientResponseResult),
    IsTimedout(MessageId),
    Cancel(MessageId),
    Stop,
}

#[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
use ::tokio::{time::sleep, task::{self, JoinHandle}};
#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use ::async_std::task::{self, JoinHandle, sleep};

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
pub struct ClientBroker {
    pub pending: HashMap<MessageId, oneshot::Sender<Result<ClientResponseResult, Error>>>,
    pub timingouts: HashMap<MessageId, JoinHandle<()>>,
    pub next_timeout: Option<Duration>
}

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
#[async_trait::async_trait]
impl brw::Broker for ClientBroker {
    type Item = ClientBrokerItem;
    type WriterItem = ClientWriterItem;
    type Ok = ();
    type Error = Error;

    async fn op<W>(&mut self, ctx: &Arc<Context<Self::Item>>, item: Self::Item, mut writer: W) -> Running<Result<Self::Ok, Self::Error>>
    where W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin {
        let res = match item {
            ClientBrokerItem::SetTimeout(dur) => {
                self.next_timeout = Some(dur);
                Ok(())
            },
            ClientBrokerItem::Request{
                header,
                body,
                resp_tx,
            } => {
                let id = header.id;
                if let Some(dur) = self.next_timeout.take() {
                    if let Err(err) = writer.send(ClientWriterItem::Timeout(id, dur)).await {
                        return Running::Continue(Err(err.into()))
                    }

                    let broker = ctx.broker.clone();
                    let handle = task::spawn(async move {
                        sleep(dur).await;
                        broker.send_async(ClientBrokerItem::IsTimedout(id)).await
                            .unwrap_or_else(|err| log::error!("{:?}", err));
                    });
                    self.timingouts.insert(id, handle);
                }
                let res = writer.send(ClientWriterItem::Request(header, body)).await;
                self.pending.insert(id, resp_tx);

                res.map_err(|err| err.into())
            },
            ClientBrokerItem::Response(id , res) => {
                self.timingouts.remove(&id);
                if let Some(resp_tx) = self.pending.remove(&id) {
                    resp_tx.send(Ok(res)) 
                        .map_err(|_| {
                            Error::Internal("InternalError: client failed to send response over channel".into())
                        })
                } else {
                    Err(Error::Internal("Done channel not found".into()))
                }
            },
            ClientBrokerItem::IsTimedout(id) => {
                if let Some(resp_tx) = self.pending.remove(&id) {
                    if let Err(err) = resp_tx.send(Err(Error::Timeout(Some(id))))
                        .map_err(|_| {
                            Error::Internal("InternalError: client failed to send response over channel".into())
                        }) 
                    {
                        return Running::Continue(Err(err))
                    }
                }
                if let Some(mut handle) = self.timingouts.remove(&id) {
                    handle.conclude();
                }
                Ok(())
            },
            ClientBrokerItem::Cancel(id) => {
                if let Some(resp_tx) = self.pending.remove(&id) {
                    drop(resp_tx);
                }
                writer.send(ClientWriterItem::Cancel(id)).await
                    .map_err(|err| err.into())
            },
            ClientBrokerItem::Stop => {
                if let Err(err) = writer.send(ClientWriterItem::Stop).await {
                    log::error!("{:?}", err);
                }
                return Running::Stop
            }
        };

        Running::Continue(res)
    }
}