use cfg_if::cfg_if;
use std::{time::Duration};

use futures::{channel::oneshot};

cfg_if!{
    if #[cfg(any(
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
    ))] {
        use std::{sync::Arc, collections::HashMap};
        use brw::{Context, Running};
        use futures::{Sink, SinkExt};
        
        use super::{writer::ClientWriterItem};
    }
}

use toy_rpc_core::{error::Error, message::{ClientRequestBody, ClientResponseResult, MessageId, RequestHeader}};


#[cfg_attr(all(not(feature = "tokio_runtime"), not(feature = "async_std_runtime")), allow(dead_code))]
pub enum ClientBrokerItem {
    SetTimeout(Duration),
    Request{
        header: RequestHeader, 
        body: ClientRequestBody, 
        resp_tx: oneshot::Sender<Result<ClientResponseResult, Error>>,
    },
    Response(MessageId, ClientResponseResult),
    Cancel(MessageId),
    Stop,
}

#[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
use ::tokio::{task::{self}};
#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use ::async_std::task::{self};

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
pub struct ClientBroker {
    pub pending: HashMap<MessageId, oneshot::Sender<Result<ClientResponseResult, Error>>>,
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

    async fn op<W>(&mut self, _: &Arc<Context<Self::Item>>, item: Self::Item, mut writer: W) -> Running<Result<Self::Ok, Self::Error>>
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
                let (tx, rx) = oneshot::channel();
                let fut = async move {
                    // Takes care of receiving/cancel error
                    match rx.await {
                        Ok(res) => res,
                        Err(_) => Err(Error::Canceled(Some(id)))
                    }
                };
                if let Some(dur) = self.next_timeout.take() {
                    if let Err(err) = writer.send(ClientWriterItem::Timeout(id, dur)).await {
                        return Running::Continue(Err(err.into()))
                    }

                    task::spawn(async move {
                        #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                        let res = ::tokio::time::timeout(dur, fut).await;
                        #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                        let res = ::async_std::future::timeout(dur, fut).await;

                        let res = match res {
                            Ok(res) => res,
                            Err(_) => {
                                resp_tx.send(Err(Error::Timeout(Some(id))))
                                    .unwrap_or_else(|_| log::error!("Error sending over response sender"));
                                return
                            }
                        };
                        resp_tx.send(res)
                            .unwrap_or_else(|_| log::error!("Error sending over response sender"));
                    });
                } else {
                    task::spawn(async move {
                        let res = fut.await;
                        resp_tx.send(res)
                            .unwrap_or_else(|_| log::error!("Error sending over response sender"));
                    });
                }
                let res = writer.send(ClientWriterItem::Request(header, body)).await;
                self.pending.insert(id, tx);

                res.map_err(|err| err.into())
            },
            ClientBrokerItem::Response(id , res) => {
                if let Some(tx) = self.pending.remove(&id) {
                    tx.send(Ok(res)) 
                        .map_err(|_| {
                            Error::Internal("InternalError: client failed to send response over channel".into())
                        })
                } else {
                    Err(Error::Internal("Done channel not found".into()))
                        
                }
            },
            ClientBrokerItem::Cancel(id) => {
                if let Some(tx) = self.pending.remove(&id) {
                    tx.send(Err(Error::Canceled(Some(id))))
                        .unwrap_or_else(|_| log::error!("Error sending over response sender"));
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