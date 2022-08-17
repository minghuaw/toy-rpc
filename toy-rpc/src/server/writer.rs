use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    codec::CodecWrite,
    error::Error,
    message::{ErrorMessage, MessageId},
    pubsub::SeqId,
    service::ServiceCallResult,
    util::{GracefulShutdown, Running, Writer},
};

use crate::protocol::Header;

#[cfg_attr(feature = "http_actix_web", derive(actix::Message))]
#[cfg_attr(feature = "http_actix_web", rtype(result = "()"))]
pub(crate) enum ServerWriterItem {
    Response {
        id: MessageId,
        result: ServiceCallResult,
    },
    /// Publish subscription item to client
    Publication {
        seq_id: SeqId,
        topic: String,
        content: Arc<Vec<u8>>,
    },
    Stopping,
    Stop,
}

pub(crate) struct ServerWriter<W> {
    writer: W,
}

impl<W: CodecWrite + GracefulShutdown> ServerWriter<W> {
    #[cfg(not(feature = "http_actix_web"))]
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    async fn write_response(
        &mut self,
        id: MessageId,
        result: ServiceCallResult,
    ) -> Result<(), Error> {
        match result {
            Ok(body) => {
                log::trace!("Message {} Success", &id);
                let header = Header::Response { id, is_ok: true };
                self.writer.write_header(header).await?;
                self.writer.write_body(id, &body).await?;
                Ok(())
            }
            Err(err) => {
                log::trace!("Message {} Error", &id);
                let header = Header::Response { id, is_ok: false };
                let msg = match ErrorMessage::from_err(err) {
                    Ok(m) => m,
                    Err(err) => {
                        log::debug!("Non-sendable error: {}", err);
                        return Ok(());
                    }
                };
                self.writer.write_header(header).await?;
                self.writer.write_body(id, &msg).await?;
                Ok(())
            }
        }
    }

    async fn write_publication(
        &mut self,
        id: MessageId,
        topic: String,
        content: &[u8],
    ) -> Result<(), Error> {
        let header = Header::Publish { id, topic };
        self.writer.write_header(header).await?;
        self.writer.write_body_bytes(id, &content).await?;
        Ok(())
    }

}

#[async_trait]
impl<W> Writer for ServerWriter<W> 
where
    W: CodecWrite + GracefulShutdown,
{
    type Item = ServerWriterItem;

    async fn op(&mut self, item: ServerWriterItem) -> Result<Running, Error> {
        match item {
            ServerWriterItem::Response { id, result } => self
                .write_response(id, result)
                .await
                .map(|_| Running::Continue),
            ServerWriterItem::Publication {
                seq_id,
                topic,
                content,
            } => {
                let id = seq_id.0;
                self.write_publication(id, topic, &content).await.map(|_| Running::Continue)
            }
            // ServerWriterItem::Ack { id } => self.write_ack(id).await,
            ServerWriterItem::Stopping => {
                self.writer.close().await;
                Ok(Running::Continue)
            },
            ServerWriterItem::Stop => Ok(Running::Stop),
        }
    }
}
