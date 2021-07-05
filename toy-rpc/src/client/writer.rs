use cfg_if::cfg_if;

use crate::{message::Metadata, util::GracefulShutdown};

cfg_if!{
    if #[cfg(any(
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))] {
        use async_trait::async_trait;
        use brw::Running;

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
            Request(Header, Box<OutboundBody>),
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
                log::debug!("{:?}", &header);
                let id = header.get_id();
                self.writer.write_header(header).await?;
                self.writer.write_body(&id, body).await
            }
        }

        #[async_trait]
        impl<W: CodecWrite + GracefulShutdown> brw::Writer for ClientWriter<W> {
            type Item = ClientWriterItem;
            type Ok = ();
            type Error = Error;
        
            async fn op(&mut self, item: Self::Item) -> Running<Result<Self::Ok, Self::Error>> {
                let res = match item {
                    ClientWriterItem::Request(header, body) => {
                        self.write_request(header, &body).await
                    },
                    ClientWriterItem::Cancel(id) => {
                        let header = Header::Cancel(id);
                        let body: String =
                            format!("{}{}{}", CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, id);
                        let body = Box::new(body) as Box<OutboundBody>;
                        self.write_request(header, &body).await
                    },
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

