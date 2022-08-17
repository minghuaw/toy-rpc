use std::sync::Arc;
use std::time::Duration;

use crate::{
    codec::CodecWrite,
    message::{MessageId, Metadata, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM},
    protocol::{Header, OutboundBody},
    util::{GracefulShutdown, Running},
    Error,
};

pub enum ClientWriterItem {
    Request(MessageId, String, Duration, Box<OutboundBody>),
    Publish(MessageId, String, Arc<Vec<u8>>),
    Subscribe(MessageId, String),
    Unsubscribe(MessageId, String),

    // Client will respond to Publish message sent from the server
    // Thus needs to reply with the seq_id
    // Ack(SeqId),
    Cancel(MessageId),
    Stopping,
    Stop,
}

pub struct ClientWriter<W> {
    pub writer: W,
}

impl<W: CodecWrite> ClientWriter<W> {
    pub async fn write_request(
        &mut self,
        header: Header,
        body: &(dyn erased_serde::Serialize + Send + Sync),
    ) -> Result<(), Error> {
        let id = header.id();
        self.writer.write_header(header).await?;
        self.writer.write_body(id, body).await?;
        Ok(())
    }

    pub async fn write_publish_item(&mut self, header: Header, bytes: &[u8]) -> Result<(), Error> {
        let id = header.id();
        self.writer.write_header(header).await?;
        self.writer.write_body_bytes(id, bytes).await?;
        Ok(())
    }
}

impl<W: CodecWrite + GracefulShutdown> ClientWriter<W> {
    async fn op(
        &mut self,
        item: ClientWriterItem,
    ) -> Result<Running, Error> {
        match item {
            ClientWriterItem::Request(id, service_method, duration, body) => {
                let header = Header::Request {
                    id,
                    service_method,
                    timeout: duration,
                };
                log::debug!("{:?}", &header);
                self.write_request(header, &body).await
                    .map(|_| Running::Continue)
            }
            ClientWriterItem::Cancel(id) => {
                let header = Header::Cancel(id);
                log::debug!("{:?}", &header);
                let body: String =
                    format!("{}{}{}", CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, id);
                let body = Box::new(body) as Box<OutboundBody>;
                self.write_request(header, &body).await
                    .map(|_| Running::Continue)
            }
            ClientWriterItem::Publish(id, topic, body) => {
                let header = Header::Publish { id, topic };
                log::debug!("{:?}", &header);
                self.write_publish_item(header, &body).await
                    .map(|_| Running::Continue)
            }
            ClientWriterItem::Subscribe(id, topic) => {
                let header = Header::Subscribe { id, topic };
                log::debug!("{:?}", &header);
                self.write_request(header, &()).await
                    .map(|_| Running::Continue)
            }
            ClientWriterItem::Unsubscribe(id, topic) => {
                let header = Header::Unsubscribe { id, topic };
                log::debug!("{:?}", &header);
                self.write_request(header, &()).await
                    .map(|_| Running::Continue)
            }
            ClientWriterItem::Stopping => {
                self.writer.close().await;
                Ok(Running::Continue)
            },
            ClientWriterItem::Stop => Ok(Running::Stop),
        }
    }

    async fn handle_error(error: Error) -> Result<Running, Error> {
        Err(error)
    }
}
