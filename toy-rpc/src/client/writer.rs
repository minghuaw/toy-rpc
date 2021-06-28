use cfg_if::cfg_if;

use crate::{message::Metadata, util::GracefulShutdown};

cfg_if!{
    if #[cfg(any(
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))] {
        use std::time::Duration;
        use async_trait::async_trait;
        use brw::Running;

        use crate::message::{ClientRequestBody, MessageId, RequestHeader};

        use crate::{
            Error, codec::CodecWrite, 
            message::{
                TIMEOUT_TOKEN, CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, TimeoutRequestBody
            }
        };

        pub enum ClientWriterItem {
            Timeout(MessageId, Duration),
            Request(RequestHeader, ClientRequestBody),
            Cancel(MessageId),
            Stop,
        }

        pub struct ClientWriter<W> {
            pub writer: W
        }

        impl<W: CodecWrite> ClientWriter<W> {
            pub async fn write_request(
                &mut self,
                header: RequestHeader,
                body: &(dyn erased_serde::Serialize + Send + Sync)
            ) -> Result<(), Error> {
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
                    ClientWriterItem::Timeout(id, dur) => {
                        let timeout_header = RequestHeader {
                            id,
                            service_method: TIMEOUT_TOKEN.into()
                        };
                        let timeout_body = Box::new(
                            TimeoutRequestBody::new(dur)
                        ) as ClientRequestBody;
                        self.write_request(timeout_header, &timeout_body).await
                    },
                    ClientWriterItem::Request(header, body) => {
                        self.write_request(header, &body).await
                    },
                    ClientWriterItem::Cancel(id) => {
                        let header = RequestHeader {
                            id,
                            service_method: CANCELLATION_TOKEN.into(),
                        };
                        let body: String =
                            format!("{}{}{}", CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, id);
                        let body = Box::new(body) as ClientRequestBody;
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

