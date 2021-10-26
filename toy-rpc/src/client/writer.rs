use std::sync::Arc;

use cfg_if::cfg_if;

use crate::pubsub::SeqId;

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))] {
        use std::time::Duration;
        use async_trait::async_trait;
        use brw::Running;

        use crate::{
            Error, codec::CodecWrite,
            message::{
                Metadata, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, MessageId
            },
            protocol::{
                Header, OutboundBody
            },
            util:: GracefulShutdown
        };

        pub enum ClientWriterItem {
            Request(MessageId, String, Duration, Box<OutboundBody>),
            Publish(MessageId, String, Arc<Vec<u8>>),
            Subscribe(MessageId, String),
            Unsubscribe(MessageId, String),

            // Client will respond to Publish message sent from the server
            // Thus needs to reply with the seq_id
            Ack(SeqId),
            Cancel(MessageId),
            Stopping,
            Stop,
        }

        pub struct ClientWriter<W> {
            pub writer: W
        }

        impl<W: CodecWrite> ClientWriter<W> {
            pub async fn write_request(
                &mut self,
                header: Header,
                body: &(dyn erased_serde::Serialize + Send + Sync),
            ) -> Result<(), Error> {
                let id = header.id();
                self.writer.write_header(header).await?;
                self.writer.write_body(id, body).await
            }

            pub async fn write_publish_item(
                &mut self,
                header: Header,
                bytes: &[u8]
            ) -> Result<(), Error> {
                let id = header.id();
                self.writer.write_header(header).await?;
                self.writer.write_body_bytes(id, bytes).await
            }
        }

        #[async_trait]
        impl<W: CodecWrite + GracefulShutdown> brw::Writer for ClientWriter<W> {
            type Item = ClientWriterItem;
            type Ok = ();
            type Error = Error;

            async fn op(&mut self, item: Self::Item) -> Running<Result<Self::Ok, Self::Error>> {
                let res = match item {
                    ClientWriterItem::Request(id, service_method, duration, body) => {
                        let header = Header::Request{id, service_method, timeout: duration};
                        log::debug!("{:?}", &header);
                        self.write_request(header, &body).await
                    },
                    ClientWriterItem::Cancel(id) => {
                        let header = Header::Cancel(id);
                        log::debug!("{:?}", &header);
                        let body: String =
                            format!("{}{}{}", CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, id);
                        let body = Box::new(body) as Box<OutboundBody>;
                        self.write_request(header, &body).await
                    },
                    ClientWriterItem::Publish(id, topic, body) => {
                        let header = Header::Publish{id, topic};
                        log::debug!("{:?}", &header);
                        self.write_publish_item(header, &body).await
                    },
                    ClientWriterItem::Subscribe(id, topic) => {
                        let header = Header::Subscribe{id, topic};
                        log::debug!("{:?}", &header);
                        self.write_request(header, &()).await
                    },
                    ClientWriterItem::Unsubscribe(id, topic) => {
                        let header = Header::Unsubscribe{id, topic};
                        log::debug!("{:?}", &header);
                        self.write_request(header, &()).await
                    },
                    ClientWriterItem::Ack(seq_id) => {
                        let header = Header::Ack(seq_id.0);
                        log::debug!("{:?}", &header);
                        // There is no body frame for Ack message
                        self.writer.write_header(header).await
                    },
                    ClientWriterItem::Stopping => {
                        Ok(self.writer.close().await)
                    },
                    ClientWriterItem::Stop => {
                        return Running::Stop
                    }
                };

                Running::Continue(res)
            }

            async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()> {
                if let Err(err) = res {
                    #[cfg(feature = "debug")]
                    log::error!("{:?}", err);

                    // Drop the writer if unable to write to connection
                    if let Error::IoError(_) = err {
                        return Running::Stop
                    }
                }
                Running::Continue(())
            }
        }
    }
}
