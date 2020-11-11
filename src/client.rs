use async_std::net::{TcpStream, ToSocketAddrs};
use async_std::sync::{Arc, Mutex};
use async_std::task;
use erased_serde as erased;
use serde;
use std::sync::atomic::Ordering;
use std::marker::PhantomData;
use futures::channel::mpsc::{
    // channel, 
    Receiver, Sender
};
use futures::{
    // AsyncWrite, 
    StreamExt, SinkExt
};
use surf;

use crate::codec::{ClientCodec, DefaultCodec};
use crate::error::{Error, RpcError};
use crate::message::{AtomicMessageId, MessageId, RequestHeader, ResponseHeader};

// const CHANNEL_BUF_SIZE: usize = 64;

pub struct NotConnected {}
pub struct Connected {}

pub struct Client<Mode> {
    count: AtomicMessageId,
    codec: Arc<Mutex<Box<dyn ClientCodec>>>,

    mode: PhantomData<Mode>,
}

impl<Mode> Client<Mode> {
    pub fn new(stream: TcpStream) -> Client<Connected> {
        let box_codec: Box<dyn ClientCodec> = Box::new(DefaultCodec::new(stream));

        Client::<Connected> {
            count: AtomicMessageId::new(0u16),
            codec: Arc::new(Mutex::new(box_codec)),

            mode: PhantomData,
        }
    }

    pub fn dial(addr: impl ToSocketAddrs) -> Result<Client<Connected>, std::io::Error> {
        let stream = task::block_on(TcpStream::connect(addr))?;

        Ok(Self::new(stream))
    }

    // pub fn dial_http(addr: &'static str) -> Client<Connected> {
    //     // spawn a task to receive inputs from channel and send message as http request
    //     let (mut req_sender, mut req_recver) = channel::<Vec<u8>>(CHANNEL_BUF_SIZE);
    //     let (mut res_sender, mut res_recver) = channel::<Vec<u8>>(CHANNEL_BUF_SIZE);

    //     let handle = task::spawn(Client::<Connected>::_http_client_loop(addr, req_recver, res_sender));
    //     let codec = DefaultCodec::from_reader_writer(
    //         res_recver,
    //         req_sender
    //     );
    //     // unimplemented!()
    // }
}

impl Client<Connected> {
    pub fn call<Req, Res>(&self, service_method: impl ToString, args: Req) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        let codec = self.codec.clone();
        let id = self.count.fetch_add(1u16, Ordering::Relaxed);

        task::block_on(async move { Self::_async_call(service_method, &args, id, codec).await })
    }

    pub fn task<'a, Req, Res>(
        &self,
        service_method: impl ToString + Send + 'static,
        args: Req,
    ) -> task::JoinHandle<Result<Res, Error>>
    where
        Req: serde::Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static,
    {
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
        id: MessageId,
        codec: Arc<Mutex<Box<dyn ClientCodec>>>,
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        let _codec = &mut *codec.lock().await;
        let header = RequestHeader {
            id,
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

    async fn _http_client_loop(addr: &str, mut req_recver: Receiver<Vec<u8>>, mut res_sender: Sender<Vec<u8>>) -> Result<(), Error> {
        let mut http_client = surf::Client::new();
        http_client.set_base_url(surf::Url::parse(addr)?); // the url::ParseError will be converted to toy_rpc::error::Error

        while let Some(encoded_req) = req_recver.next().await {
            let body = surf::Body::from_bytes(encoded_req);
            let http_req = surf::post("").body(body);
            let mut http_res = http_client.send(http_req).await
                .map_err(|e| Error::TransportError{msg: format!("{}", e)})?;
            let encoded_res = http_res.take_body().into_bytes().await
                .map_err(|e| Error::TransportError{msg: format!("{}", e)})?;
            
            // send back result
            res_sender.send(encoded_res).await
                .map_err(|e| Error::TransportError{msg: format!("{}", e)})?;
        }
        
        Ok(())
    }
}
