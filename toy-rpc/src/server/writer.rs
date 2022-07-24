use std::sync::Arc;

use crate::{
    codec::CodecWrite,
    error::Error,
    message::{ErrorMessage, MessageId},
    pubsub::SeqId,
    service::ServiceCallResult,
    util::{GracefulShutdown, Running},
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
    // Ack {
    //     // Server will only need to Ack Publish request from client.
    //     // Thus should reply with the MessageId that came from the client
    //     id: MessageId,
    // },
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

    // // Ack message
    // async fn write_ack(&mut self, id: MessageId) -> Result<(), Error> {
    //     let header = Header::Ack(id);
    //     self.writer.write_header(header).await?;
    //     Ok(())
    // }
    
    async fn op(
        &mut self,
        item: ServerWriterItem,
    ) -> Running<Result<(), Error>, Option<Error>> {
        let res = match item {
            ServerWriterItem::Response { id, result } => self.write_response(id, result).await,
            ServerWriterItem::Publication {
                seq_id,
                topic,
                content,
            } => {
                let id = seq_id.0;
                self.write_publication(id, topic, &content).await
            }
            // ServerWriterItem::Ack { id } => self.write_ack(id).await,
            ServerWriterItem::Stopping => Ok(self.writer.close().await),
            ServerWriterItem::Stop => return Running::Stop(None),
        };
        Running::Continue(res)
    }
}

// #[async_trait::async_trait]
// impl<W: CodecWrite + GracefulShutdown> Writer for ServerWriter<W> {
//     type Item = ServerWriterItem;
//     type Ok = ();
//     type Error = Error;


//     async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<(), Option<Self::Error>> {
//         if let Err(err) = res {
//             log::error!("{}", err);
//         }
//         Running::Continue(())
//     }
// }
