use async_std::net::{TcpListener, TcpStream};
use async_std::sync::Arc;
use async_std::task;
use erased_serde as erased;
use futures::StreamExt;
use std::collections::HashMap;

#[cfg(features="http_tide")]
use tide;

use crate::codec::{DefaultCodec, ServerCodec};
use crate::error::{Error, RpcError};
use crate::message::{MessageId, RequestHeader, ResponseHeader};
use crate::service::{
    ArcAsyncServiceCall, AsyncServiceMap, HandleService, HandlerResult, HandlerResultFut,
};

#[cfg(feature = "http_tide")]
const DEFAULT_PATH: &str = "/_rpc_";

#[derive(Clone)]
pub struct Server {
    services: Arc<AsyncServiceMap>,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    pub async fn accept(&self, listener: TcpListener) -> Result<(), Error> {
        let mut incoming = listener.incoming();

        while let Some(conn) = incoming.next().await {
            let stream = conn?;
            log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

            task::spawn(Self::_serve_conn(stream, self.services.clone()));
        }

        Ok(())
    }

    async fn _serve_conn(stream: TcpStream, services: Arc<AsyncServiceMap>) -> Result<(), Error> {
        // let _stream = stream;
        let peer_addr = stream.peer_addr()?;

        // using feature flag controlled default codec
        let codec = DefaultCodec::new(stream);

        // let fut = task::spawn_blocking(|| Self::_serve_codec(codec, services)).await;
        let fut = Self::_serve_codec(codec, services);

        let ret = fut.await;
        log::info!("Client disconnected from {}", peer_addr);
        ret
    }

    async fn _serve_codec<C>(mut codec: C, services: Arc<AsyncServiceMap>) -> Result<(), Error>
    where
        C: ServerCodec + Send + Sync,
    {
        loop {
            Self::_serve_codec_once(&mut codec, &services).await?
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    async fn _serve_codec_once<C>(codec: &mut C, services: &Arc<AsyncServiceMap>) -> Result<(), Error>
    where 
        C: ServerCodec + Send + Sync 
    {
        if let Some(header) = codec.read_request_header().await {
            // destructure header
            let RequestHeader { id, service_method } = header?;
            // let service_method = &service_method[..];
            let pos = service_method
                .rfind(".")
                .ok_or(Error::RpcError(RpcError::MethodNotFound))?;
            let service_name = &service_method[..pos];
            let method_name = service_method[pos + 1..].to_owned();

            log::info!("Message {}, service: {}, method: {}", id, service_name, method_name);

            // look up the service
            // TODO; consider adding a new error type
            let call: ArcAsyncServiceCall = services
                .get(service_name)
                .ok_or(Error::RpcError(RpcError::MethodNotFound))?
                .clone();

            // read body
            let res = {
                log::debug!("Reading request body");
                let deserializer = codec.read_request_body().await.unwrap()?;

                log::debug!("Calling handler");
                // pass ownership to the `call`
                call(method_name, deserializer).await
            };

            // send back result
            let bytes_sent = Self::_send_response(codec, id, res).await?;
            log::debug!("Response sent with {} bytes", bytes_sent);
        }

        Ok(())
    }

    // async fn _handle_request<C>()

    async fn _send_response<C>(
        _codec: &mut C,
        id: MessageId,
        res: HandlerResult,
    ) -> Result<usize, Error>
    where
        C: ServerCodec + Send + Sync,
    {
        match res {
            Ok(b) => {
                log::info!("Message {} Success", id.clone());
                let header = ResponseHeader {
                    id,
                    is_error: false,
                };

                let bytes_sent = _codec.write_response(header, &b).await?;
                Ok(bytes_sent)
            }
            Err(e) => {
                log::info!("Message {} Error", id.clone());
                let header = ResponseHeader { id, is_error: true };

                let body = match e {
                    Error::RpcError(rpc_err) => Box::new(rpc_err),
                    _ => Box::new(RpcError::ServerError(e.to_string())),
                };

                //
                let bytes_sent = _codec.write_response(header, &body).await?;
                Ok(bytes_sent)
            }
        }
    }

    pub async fn serve_conn(&self, stream: TcpStream) -> Result<(), Error> {
        Self::_serve_conn(stream, self.services.clone()).await
    }

    pub async fn serve_codec<C>(&self, codec: C) -> Result<(), Error>
    where
        C: ServerCodec + Send + Sync,
    {
        Self::_serve_codec(codec, self.services.clone()).await
    }
}

impl Server {
    #[cfg(feature = "http_tide")]
    pub fn handle_http(&'static self) -> tide::Server<&Self> {
        use futures::io::{BufReader, BufWriter};

        let mut app = tide::Server::with_state(self);
        app.at(DEFAULT_PATH).all(|mut req: tide::Request<&'static Server>| async move {
            let input = req.body_bytes().await?;
            let mut output: Vec<u8> = Vec::new();
            
            let mut codec = DefaultCodec::from_reader_writer(
                BufReader::new(&input[..]), 
                BufWriter::new(&mut output)
            );
            let services = req.state().services.clone();

            Self::_serve_codec_once(&mut codec, &services).await?;

            // construct tide::Response 
            Ok(tide::Body::from_bytes(output))
        });

        app
    }
}

pub struct ServerBuilder {
    services: AsyncServiceMap,
}

impl ServerBuilder {
    pub fn new() -> Self {
        ServerBuilder {
            services: HashMap::new(),
        }
    }

    pub fn register<S, T>(self, service_name: &'static str, service: S) -> Self
    where
        S: HandleService<T> + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let call = move |method_name: String,
                         _deserializer: Box<(dyn erased::Deserializer<'static> + Send)>|
              -> HandlerResultFut { service.call(&method_name, _deserializer) };

        let mut ret = self;
        ret.services.insert(service_name, Arc::new(call));
        ret
    }

    pub fn build(self) -> Server {
        Server {
            services: Arc::new(self.services),
        }
    }
}
