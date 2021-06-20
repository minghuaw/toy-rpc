use std::{sync::Arc, time::Duration};
use async_trait::async_trait;
use brw::{Context, Running};
use futures::{Sink, SinkExt, channel::oneshot};

use crate::{Error, message::{ClientRequestBody, ClientResponseResult, MessageId, RequestHeader}};

use super::{ResponseMap, writer::ClientWriterItem};

#[cfg_attr(all(not(feature = "tokio_runtime"), not(feature = "async_std_runtime")), allow(dead_code))]
pub enum ClientBrokerItem {
    // Timeout(Duration),
    Request{
        header: RequestHeader, 
        body: ClientRequestBody, 
        resp_tx: oneshot::Sender<ClientResponseResult>,
        timeout: Option<Duration>
    },
    Response(MessageId, ClientResponseResult),
    Cancel(MessageId),
    Stop,
}

pub struct ClientBroker {
    // counter: MessageId,
    pub pending: ResponseMap,
    // next_timeout: Option<Duration>
}

#[async_trait]
impl brw::Broker for ClientBroker {
    type Item = ClientBrokerItem;
    type WriterItem = ClientWriterItem;
    type Ok = ();
    type Error = Error;

    async fn op<W>(&mut self, _: &Arc<Context<Self::Item>>, item: Self::Item, mut writer: W) -> Running<Result<Self::Ok, Self::Error>>
    where W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin {
        let res = match item {
            // ClientBrokerItem::Timeout(dur) => {
            //     self.next_timeout = Some(dur);
            //     Ok(())
            // },
            ClientBrokerItem::Request{
                header,
                body,
                resp_tx,
                mut timeout,
            } => {
                // self.counter = self.counter + 1;
                // let id = self.counter;
                // let header = RequestHeader {id, service_method};
                let id = header.id;
                if let Some(dur) = timeout.take() {
                    if let Err(err) = writer.send(ClientWriterItem::Timeout(id, dur)).await {
                        return Running::Continue(Err(err.into()))
                    }
                }
                let res = writer.send(ClientWriterItem::Request(header, body)).await;
                self.pending.insert(id, resp_tx);
                res.map_err(|err| err.into())
            },
            ClientBrokerItem::Response(id , res) => {
                if let Some(resp_tx) = self.pending.remove(&id) {
                    resp_tx.send(res) 
                        .map_err(|_| {
                            Error::Internal("InternalError: client failed to send response over channel".into())
                        })
                } else {
                    Err(Error::Internal("Done channel not found".into()))
                }
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