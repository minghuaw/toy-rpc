use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(any(
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))] {
        use std::time::Duration;
        use async_trait::async_trait;
        use brw::Running;

        use crate::{message::Metadata, util::GracefulShutdown};

        use crate::{
            Error, codec::CodecWrite,
            message::{
                CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, MessageId
            },
            protocol::{
                Header, OutboundBody
            }
        };

        pub enum ClientWriterItem {
            Request(MessageId, String, Duration, Box<OutboundBody>),
            Publish(MessageId, String, Box<OutboundBody>),
            Subscribe(MessageId, String),
            Unsubscribe(MessageId, String),
            Cancel(MessageId),
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
                let id = header.get_id();
                self.writer.write_header(header).await?;
                self.writer.write_body(id, body).await
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
                        self.write_request(header, &body).await
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
                    }
                    ClientWriterItem::Stop => {
                        self.writer.close().await;
                        return Running::Stop
                    }
                };

                Running::Continue(res)
            }
        }
    }
}
