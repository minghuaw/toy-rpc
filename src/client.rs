use async_std::net::{TcpStream, ToSocketAddrs};
use async_std::sync::{Arc, Mutex};
use async_std::task;
use erased_serde as erased;
use serde;
use std::sync::atomic::Ordering;

use crate::codec::{ClientCodec, DefaultCodec};
use crate::{Error, RpcError};
use crate::message::{AtomicMessageId, MessageId, RequestHeader, ResponseHeader};

pub struct Client {
    count: AtomicMessageId,
    codec: Arc<Mutex<Box<dyn ClientCodec>>>,
}

impl Client {
    pub fn new(stream: TcpStream) -> Self {
        let box_codec: Box<dyn ClientCodec> = Box::new(DefaultCodec::new(stream));

        Self {
            count: AtomicMessageId::new(0u16),
            codec: Arc::new(Mutex::new(box_codec)),
        }
    }

    pub fn dial(addr: impl ToSocketAddrs) -> Result<Self, std::io::Error> {
        let stream = task::block_on(TcpStream::connect(addr))?;

        Ok(Self::new(stream))
    }

    // pub fn dial_http(addr: impl ToSocketAddrs) -> Self {
    //     unimplemented!()
    // }

    pub fn call<Req, Res>(&self, service_method: impl ToString, args: Req) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        let codec = self.codec.clone();
        // let id = self.count.clone();
        let id = self.count.fetch_add(1u16, Ordering::Relaxed);

        task::block_on(async move { Self::_async_call(service_method, &args, id, codec).await })
    }

    pub fn task<'a, Req, Res, M>(
        &self,
        service_method: M,
        args: Req,
    ) -> task::JoinHandle<Result<Res, Error>>
    where
        Req: serde::Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static,
        M: ToString + Send + 'static,
    {
        // let _args = args.to_owned();
        let codec = self.codec.clone();
        let id = self.count.fetch_add(1u16, Ordering::Relaxed);

        task::spawn(async move { Self::_async_call(service_method, &args, id, codec).await })
    }

    pub async fn async_call<Req, Res>(
        &self,
        service_method: impl ToString,
        args: Req,
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        let codec = self.codec.clone();
        let id = self.count.fetch_add(1u16, Ordering::Relaxed);
        Self::_async_call(service_method, &args, id, codec).await
    }

    async fn _async_call<Req, Res>(
        service_method: impl ToString,
        args: &Req,
        _id: MessageId,
        codec: Arc<Mutex<Box<dyn ClientCodec>>>,
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        let _codec = &mut *codec.lock().await;
        // let _id = id.lock().await.clone();
        // let _id = id;

        // let id = _id;
        let header = RequestHeader {
            id: _id,
            service_method: service_method.to_string(),
        };
        let req = &args as &(dyn erased::Serialize + Send + Sync);

        // send request
        let bytes_sent = _codec.write_request(header, req).await?;
        log::info!("Request sent with {} bytes", bytes_sent);

        // wait for response
        if let Some(header) = _codec.read_response_header().await {
            let ResponseHeader { id: _, is_error } = header?;
            let deserializer = _codec.read_response_body().await.ok_or(Error::NoneError)?;
            let mut deserializer = deserializer?;

            let res = match is_error {
                false => {
                    let res: Res =
                        erased::deserialize(&mut deserializer).map_err(|e| Error::ParseError {
                            source: Box::new(e),
                        })?;

                    Ok(res)
                }
                true => {
                    let err: RpcError =
                        erased::deserialize(&mut deserializer).map_err(|e| Error::ParseError {
                            source: Box::new(e),
                        })?;

                    Err(Error::RpcError(err))
                }
            };

            return res;
        }

        Err(Error::NoneError)
    }
}
