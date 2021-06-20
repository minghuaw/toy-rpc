use std::{sync::Arc, time::Duration};
use std::collections::HashMap;
use async_trait::async_trait;
use brw::{Context, Running};
use futures::{Sink, SinkExt, channel::oneshot};

use crate::{Error, message::{ClientRequestBody, ClientResponseResult, MessageId, RequestHeader}};

use super::{writer::ClientWriterItem};

#[cfg_attr(all(not(feature = "tokio_runtime"), not(feature = "async_std_runtime")), allow(dead_code))]
pub enum ClientBrokerItem {
    SetTimeout(Duration),
    Request{
        header: RequestHeader, 
        // service_method: String,
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
    // counter: MessageId,
    pub pending: HashMap<MessageId, oneshot::Sender<Result<ClientResponseResult, Error>>>,
    pub timingouts: HashMap<MessageId, JoinHandle<()>>,
    pub next_timeout: Option<Duration>
}

#[async_trait]
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
                // self.counter = self.counter + 1;
                // let id = self.counter;
                // let header = RequestHeader {id, service_method};
                let id = header.id;
                if let Some(dur) = self.next_timeout.take() {
                    if let Err(err) = writer.send(ClientWriterItem::Timeout(id, dur)).await {
                        return Running::Continue(Err(err.into()))
                    }

                    let broker = ctx.broker.clone();
                    // #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                    // {
                        let handle = task::spawn(async move {
                            sleep(dur).await;
                            broker.send_async(ClientBrokerItem::IsTimedout(id)).await
                                .unwrap_or_else(|err| log::error!("{:?}", err));
                        });
                        self.timingouts.insert(id, handle);
                    // }

                    // #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                    // {
                    //     let handle = task::spawn(async move {
                    //         ::async_std::task::sleep(dur).await;
                    //         broker.send_async(ClientBrokerItem::IsTimedout(id)).await
                    //             .unwrap_or_else(|err| log::error!("{:?}", err));
                    //     });
                    //     self.timingouts.insert(id, handle);
                    // }
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
                if let Some(handle) = self.timingouts.remove(&id) {
                    handle.await;
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