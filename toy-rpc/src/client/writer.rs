use std::time::Duration;
use async_trait::async_trait;
use brw::Running;

use crate::{
    Error, codec::split::ClientCodecWrite, 
    message::{
        ClientRequestBody, MessageId, RequestHeader, 
        TIMEOUT_TOKEN, TimeoutRequestBody, CANCELLATION_TOKEN,
        CANCELLATION_TOKEN_DELIM, 
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

#[async_trait]
impl<W: ClientCodecWrite> brw::Writer for ClientWriter<W> {
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
                self.writer.write_request(timeout_header, &timeout_body).await
            },
            ClientWriterItem::Request(header, body) => {
                self.writer.write_request(header, &body).await
            },
            ClientWriterItem::Cancel(id) => {
                let header = RequestHeader {
                    id,
                    service_method: CANCELLATION_TOKEN.into(),
                };
                let body: String =
                    format!("{}{}{}", CANCELLATION_TOKEN, CANCELLATION_TOKEN_DELIM, id);
                let body = Box::new(body) as ClientRequestBody;
                self.writer.write_request(header, &body).await
            },
            ClientWriterItem::Stop => {
                self.writer.close().await;
                return Running::Stop
            }
        };

        Running::Continue(res)
    }
}