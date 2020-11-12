use async_std::net::{TcpStream, ToSocketAddrs};
use async_std::sync::{Arc, Mutex};
use async_std::task;
use erased_serde as erased;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::io::{BufReader, BufWriter};
use futures::{SinkExt, StreamExt};
use serde;
use std::marker::PhantomData;
use std::sync::atomic::Ordering;
use surf;

use crate::codec::{ClientCodec, DefaultCodec};
use crate::error::{Error, RpcError};
use crate::message::{AtomicMessageId, MessageId, RequestHeader, ResponseHeader};
use crate::server::RPC_PATH;

const CHANNEL_BUF_SIZE: usize = 64;

pub struct NotConnected {}
pub struct Connected {}

type Codec = Arc<Mutex<Box<dyn ClientCodec>>>;
type Channel = (Sender<Vec<u8>>, Receiver<Vec<u8>>);

pub struct Client<T, Mode> {
    count: AtomicMessageId,
    inner_codec: T,

    mode: PhantomData<Mode>,
}

impl Client<Codec, NotConnected> {
    pub fn new(stream: TcpStream) -> Client<Codec, Connected> {
        let box_codec: Box<dyn ClientCodec> = Box::new(DefaultCodec::new(stream));

        Client::<Codec, Connected> {
            count: AtomicMessageId::new(0u16),
            inner_codec: Arc::new(Mutex::new(box_codec)),

            mode: PhantomData,
        }
    }

    pub fn dial(addr: impl ToSocketAddrs) -> Result<Client<Codec, Connected>, std::io::Error> {
        let stream = task::block_on(TcpStream::connect(addr))?;

        Ok(Self::new(stream))
    }
}

impl Client<Channel, NotConnected> {
    pub fn dial_http(addr: &'static str) -> Client<Channel, Connected> {
        let (req_sender, req_recver) = channel::<Vec<u8>>(CHANNEL_BUF_SIZE);
        let (res_sender, res_recver) = channel::<Vec<u8>>(CHANNEL_BUF_SIZE);

        let channel_codec = (req_sender, res_recver);

        task::spawn(Client::_http_client_loop(addr, req_recver, res_sender));

        Client::<Channel, Connected> {
            count: AtomicMessageId::new(0u16),
            inner_codec: channel_codec,

            mode: PhantomData,
        }
    }
}

impl Client<Codec, Connected> {
    pub fn call<Req, Res>(&self, service_method: impl ToString, args: Req) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        let codec = self.inner_codec.clone();
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
        let codec = self.inner_codec.clone();
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
        let codec = self.inner_codec.clone();
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

        Client::<Codec, Connected>::_handle_response(_codec.as_mut()).await
    }
}

impl<T> Client<T, Connected> {
    async fn _handle_response<Res>(codec: &mut dyn ClientCodec) -> Result<Res, Error>
    where
        Res: serde::de::DeserializeOwned,
    {
        // wait for response
        if let Some(header) = codec.read_response_header().await {
            let ResponseHeader { id: _, is_error } = header?;
            let deserializer = codec.read_response_body().await.ok_or(Error::NoneError)?;
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

impl Client<Channel, Connected> {
    async fn _async_call_http<Req, Res>(
        service_method: impl ToString,
        args: &Req,
        id: MessageId,
        mut req_sender: Sender<Vec<u8>>,
        mut res_recver: Receiver<Vec<u8>>,
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        let mut req_buf: Vec<u8> = Vec::new();
        let res_buf: Vec<u8> = Vec::with_capacity(0);

        let header = RequestHeader {
            id,
            service_method: service_method.to_string(),
        };
        let req = &args as &(dyn erased::Serialize + Send + Sync);

        // create temp codec
        let mut codec = DefaultCodec::from_reader_writer(
            BufReader::new(&res_buf[..]),
            BufWriter::new(&mut req_buf),
        );

        // write request to buffer
        let bytes_sent = codec.write_request(header, req).await?;
        log::info!("Request sent with {} bytes", bytes_sent);

        // send req buffer to client_loop
        req_sender
            .send(req_buf)
            .await
            .map_err(|e| Error::TransportError {
                msg: format!("{}", e),
            })?;

        // wait for response
        if let Some(encoded_res) = res_recver.next().await {
            let mut req_buf = Vec::with_capacity(0);
            let res_buf = encoded_res;

            // create a new codec to parse response
            let mut codec = DefaultCodec::from_reader_writer(
                BufReader::new(&res_buf[..]),
                BufWriter::new(&mut req_buf),
            );

            return Client::<Channel, Connected>::_handle_response(&mut codec).await;
        }

        Err(Error::NoneError)
    }

    async fn _http_client_loop(
        addr: &str,
        mut req_recver: Receiver<Vec<u8>>,
        mut res_sender: Sender<Vec<u8>>,
    ) -> Result<(), Error> {
        let mut http_client = surf::Client::new();
        http_client.set_base_url(surf::Url::parse(addr)?); // the url::ParseError will be converted to toy_rpc::error::Error

        while let Some(encoded_req) = req_recver.next().await {
            let body = surf::Body::from_bytes(encoded_req);
            let http_req = surf::post(RPC_PATH).body(body);
            let mut http_res =
                http_client
                    .send(http_req)
                    .await
                    .map_err(|e| Error::TransportError {
                        msg: format!("{}", e),
                    })?;
            let encoded_res =
                http_res
                    .take_body()
                    .into_bytes()
                    .await
                    .map_err(|e| Error::TransportError {
                        msg: format!("{}", e),
                    })?;

            // send back result
            res_sender
                .send(encoded_res)
                .await
                .map_err(|e| Error::TransportError {
                    msg: format!("{}", e),
                })?;
        }

        Ok(())
    }
}
