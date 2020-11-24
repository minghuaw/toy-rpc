use async_std::net::{TcpStream, ToSocketAddrs};
use async_std::sync::{channel, Receiver, Sender};
use async_std::sync::{Arc, Mutex};
use async_std::task;
use erased_serde as erased;
use futures::io::{BufReader, BufWriter};
use std::marker::PhantomData;
use std::sync::atomic::Ordering;

use crate::codec::{ClientCodec, DefaultCodec};
use crate::error::{Error, RpcError};
use crate::message::{AtomicMessageId, MessageId, RequestHeader, ResponseHeader};

// #[cfg(feature = "tide")]
use crate::server::DEFAULT_RPC_PATH;

const CHANNEL_BUF_SIZE: usize = 64;

/// Type state for creating `Client`
pub struct NotConnected {}

/// Type state for creating `Client`
pub struct Connected {}

type Codec = Arc<Mutex<Box<dyn ClientCodec>>>;
type Channel = (Sender<Vec<u8>>, Receiver<Vec<u8>>);

/// RPC Client
pub struct Client<T, Mode> {
    count: AtomicMessageId,
    inner_codec: T,

    mode: PhantomData<Mode>,
}

impl Client<Codec, NotConnected> {
    /// Creates an RPC `Client` over socket with a specified `async_std::net::TcpStream` and the default codec
    /// 
    /// # Example
    /// ```
    /// use async_std::net::TcpStream;
    /// 
    /// #[async_std::main]
    /// async fn main() {
    ///     let stream = TcpStream::connect("127.0.0.1:8888").unwrap();
    ///     let client = Client::with_stream(stream);
    /// }
    /// ```
    pub fn with_stream(stream: TcpStream) -> Client<Codec, Connected> {
        let codec = DefaultCodec::new(stream);

        Self::with_codec(codec)
    }

    /// Creates an RPC 'Client` over socket with a specified codec
    pub fn with_codec<C>(codec: C) -> Client<Codec, Connected> 
    where 
        C: ClientCodec + Send + Sync + 'static
    {
        let box_codec: Box<dyn ClientCodec> = Box::new(codec);

        Client::<Codec, Connected> {
            count: AtomicMessageId::new(0u16),
            inner_codec: Arc::new(Mutex::new(box_codec)),

            mode: PhantomData,
        }
    }

    /// Connects the an RPC server over socket at the specified network address
    pub fn dial(addr: impl ToSocketAddrs) -> Result<Client<Codec, Connected>, Error> {
        let stream = task::block_on(TcpStream::connect(addr))?;

        Ok(Self::with_stream(stream))
    }
}

#[cfg(feature = "surf")]
impl Client<Channel, NotConnected> {
    /// Connects to an HTTP RPC server at the specified network address using the defatul codec
    /// 
    /// If a network path were to be supplpied, the network path must end with a slash "/"
    /// 
    /// # Example
    /// 
    /// ```
    /// use toy_rpc::client::Client;
    /// 
    /// #[async_std::main]
    /// async fn main() {
    ///     let addr = "http://127.0.0.1:8888/rpc/";
    ///     let client = Client::dial_http(addr).unwrap();
    /// 
    ///     let args = "arguments"
    ///     let reply: Result<String, String> = client.call_http("echo_service.echo", &args);
    ///     println!("{:?}", reply);
    /// }
    /// ```
    /// 
    /// TODO: check if the path ends with a slash
    /// TODO: add a check to test the connection
    /// TODO: try send and recv trait object
    pub fn dial_http(addr: &'static str) -> Result<Client<Channel, Connected>, Error> {
        let (req_sender, req_recver) = channel::<Vec<u8>>(CHANNEL_BUF_SIZE);
        let (res_sender, res_recver) = channel::<Vec<u8>>(CHANNEL_BUF_SIZE);

        let channel_codec = (req_sender, res_recver);

        let mut http_client = surf::Client::new();
        http_client.set_base_url(surf::Url::parse(addr)?); // the url::ParseError will be converted to toy_rpc::error::Error

        task::spawn(Client::_http_client_loop(
            http_client,
            req_recver,
            res_sender,
        ));

        Ok(Client::<Channel, Connected> {
            count: AtomicMessageId::new(0u16),
            inner_codec: channel_codec,

            mode: PhantomData,
        })
    }
}

impl Client<Codec, Connected> {
    /// Invokes the named function and wait synchronously
    pub fn call<Req, Res>(&mut self, service_method: impl ToString, args: Req) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        task::block_on(self.async_call(service_method, args))
    }

    /// Invokes the named function asynchronously by spawning a new task and returns the `JoinHandle`
    pub fn spawn_task<Req, Res>(
        &mut self,
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

    /// Invokes the named function asynchronously 
    pub async fn async_call<Req, Res>(
        &mut self,
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

#[cfg(feature = "surf")]
impl Client<Channel, Connected> {
    /// Similar to `call()`, it invokes the named function and wait synchronously,
    /// but this is for `Client` connected to a HTTP RPC server
    pub fn call_http<Req, Res>(
        &mut self,
        service_method: impl ToString,
        args: Req,
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        task::block_on(self.async_call_http(service_method, args))
    }

    /// Similar to `spawn_task()`. It invokes the named function asynchronously by spawning a new task and returns the `JoinHandle`,
    /// but this is for `Client` connected to a HTTP RPC server
    pub fn spawn_task_http<Req, Res>(
        &'static mut self,
        service_method: impl ToString + Send + 'static,
        args: Req,
    ) -> task::JoinHandle<Result<Res, Error>>
    where 
        Req: serde::Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static,
    {
        task::spawn(self.async_call_http(service_method, args))
    }

    /// Similar to `async_call()`, it invokes the named function asynchronously,
    /// but this is for `Client` connected to a HTTP RPC server
    pub async fn async_call_http<Req, Res>(
        &mut self,
        service_method: impl ToString,
        args: Req,
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        let req_sender = &self.inner_codec.0;
        let res_recver = &self.inner_codec.1;

        let id = self.count.fetch_add(1u16, Ordering::Relaxed);
        Self::_async_call_http(service_method, &args, id, req_sender, res_recver).await
    }

    async fn _async_call_http<Req, Res>(
        service_method: impl ToString,
        args: &Req,
        id: MessageId,
        req_sender: &Sender<Vec<u8>>,
        res_recver: &Receiver<Vec<u8>>,
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
        let mut codec = DefaultCodec::with_reader_writer(
            BufReader::new(&res_buf[..]),
            BufWriter::new(&mut req_buf),
        );

        // write request to buffer
        let bytes_sent = codec.write_request(header, req).await?;
        log::info!("Request sent with {} bytes", bytes_sent);

        // send req buffer to client_loop
        req_sender.send(req_buf).await;

        // wait for response
        let encoded_res = res_recver
            .recv()
            .await
            .map_err(|e| Error::TransportError { msg: e.to_string() })?;

        let mut req_buf = Vec::with_capacity(0);
        let res_buf = encoded_res;

        // create a new codec to parse response
        let mut codec = DefaultCodec::with_reader_writer(
            BufReader::new(&res_buf[..]),
            BufWriter::new(&mut req_buf),
        );

        return Client::<Channel, Connected>::_handle_response(&mut codec).await;
    }

    /// TODO: try send and recv trait object over the channel instead of `Vec<u8>`
    async fn _http_client_loop(
        http_client: surf::Client,
        req_recver: Receiver<Vec<u8>>,
        res_sender: Sender<Vec<u8>>,
    ) -> Result<(), Error> {
        loop {
            let encoded_req = req_recver
                .recv()
                .await
                .map_err(|e| Error::TransportError { msg: e.to_string() })?;

            let body = surf::Body::from_bytes(encoded_req);
            let http_req = http_client.post(DEFAULT_RPC_PATH).body(body).build();

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
            res_sender.send(encoded_res).await;
        }
    }
}
