use std::sync::Arc;

use brw::{Running, Writer};

use crate::{codec::CodecWrite, error::Error, message::{ErrorMessage, MessageId, Metadata}, protocol::OutboundBody, service::HandlerResult};

use crate::protocol::{Header};

pub(crate) enum ServerWriterItem {
    Response {
        id: MessageId,
        result: HandlerResult
    },
    /// Publish subscription item to client
    Publication {
        id: MessageId,
        topic: String,
        content: Arc<Vec<u8>>
    }
}

pub(crate) struct ServerWriter<W> {
    writer: W,
}
impl<W: CodecWrite> ServerWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer
        }
    }

    async fn write_response(&mut self, id: MessageId, result: HandlerResult) -> Result<(), Error> {
        match result {
            Ok(body) => {
                log::trace!("Message {} Success", &id);
                let header = Header::Response{ id, is_ok: true };
                // self.write_response(header, &body).await?;
                self.writer.write_header(header).await?;
                self.writer.write_body(&id, &body).await
            }
            Err(err) => {
                log::trace!("Message {} Error", &id);
                let header = Header::Response { id, is_ok: false };
                let msg = ErrorMessage::from_err(err)?;
                // self.write_response(header, &msg).await?;
                self.writer.write_header(header).await?;
                self.writer.write_body(&id, &msg).await
            }
        }
    }

    async fn write_publication(&mut self, id: MessageId, topic: String, content: Arc<Vec<u8>>) -> Result<(), Error> {
        let header = Header::Publish{id, topic};
        self.writer.write_header(header).await?;
        self.writer.write_body_bytes(&id, *content).await
    }
}

#[async_trait::async_trait]
impl<W: CodecWrite> Writer for ServerWriter<W> {
    type Item = ServerWriterItem;
    type Ok = ();
    type Error = Error;

    async fn op(&mut self, item: Self::Item) -> Running<Result<Self::Ok, Self::Error>> {
        let res = match item {
            ServerWriterItem::Response{id, result} => {
                self.write_response(id, result).await
            },
            ServerWriterItem::Publication{id, topic, content} => {
                self.write_publication(id, topic, &content).await
            }
        };
        Running::Continue(res)
    }

    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()> {
        if let Err(err) = res {
            log::error!("{:?}", err);
        }
        Running::Continue(())
    }
}