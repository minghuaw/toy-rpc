//! Implements PayloadRead and PayloadWrite for mpsc channels
//! 

use async_trait::async_trait;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use surf::Url;

use crate::error::Error;
use super::{PayloadRead, PayloadWrite};

pub(crate) struct HttpChan {
    uri: url::Url,
    http_client: surf::Client,
}

impl HttpChan {
    pub fn new(uri: url::Url) -> Self {
        Self {
            uri,
            http_client: surf::Client::new(),
        }
    }

    pub async fn connect(&self) -> Result<String, Error> {
        let res = self.http_client
            .connect(&self.uri)
            .recv_string()
            .await
            .map_err(|e| Error::TransportError { msg: e.to_string() })?;

        // log::info!("{}", res);

        Ok(res)
    }

    pub async fn post(&self, buf: Vec<u8>) -> Result<Vec<u8>, Error> {
        let res_body = match self.http_client.post(&self.uri)
                .content_type("application/octet-stream")
                .body(buf)
                .recv_bytes()
                .await
            {
                Ok(v) => v,
                Err(e) => return Err(Error::TransportError { msg: e.to_string() }),
            };
        
        Ok(res_body)
    }
}