use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws::{handshake};

// use tungstenite::Message as WsMessage;
// use futures::channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use futures::{StreamExt};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::codec::{Decoder, Encoder};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use web::{Bytes};

use crate::error::Error;

use super::{Server, AsyncServiceMap, Arc};

#[cfg(any(
    all(
        feature = "serde_bincode",
        not(feature = "serde_json"),
        not(feature = "serde_cbor"),
        not(feature = "serde_rmp"),
    ),
    all(
        feature = "serde_cbor",
        not(feature = "serde_json"),
        not(feature = "serde_bincode"),
        not(feature = "serde_rmp"),
    ),
    all(
        feature = "serde_json",
        not(feature = "serde_bincode"),
        not(feature = "serde_cbor"),
        not(feature = "serde_rmp"),
    ),
    all(
        feature = "serde_rmp",
        not(feature = "serde_cbor"),
        not(feature = "serde_json"),
        not(feature = "serde_bincode"),
    ),
    feature = "docs"
))]
/// The following impl block is controlled by feature flag. It is enabled
/// if and only if **exactly one** of the the following feature flag is turned on
/// - `serde_bincode`
/// - `serde_json`
/// - `serde_cbor`
/// - `serde_rmp`
impl Server {
    // pub async fn serve_websocket<S, E>(&self, ws_stream: S) -> Result<(), Error> 
    // where
    //     S: Stream<Item = Result<WsMessage, E>> + Sink<WsMessage> + Send + Sync + Unpin,
    //     E: std::error::Error + 'static,
    // {
    //     let ws_stream = WebSocketConn::new(ws_stream);
    //     let codec = DefaultCodec::with_websocket(ws_stream);

    //     Self::_serve_codec(codec, self.services.clone()).await
    // }

    #[cfg(feature = "actix-web")]
    async fn _handle_http(
        state: web::Data<Server>,
        req_body: web::Bytes,
    ) -> Result<web::Bytes, actix_web::Error> {
        use futures::io::{BufReader, BufWriter};

        let input = req_body.to_vec();
        let mut output: Vec<u8> = Vec::new();

        let mut codec =
            super::DefaultCodec::with_reader_writer(BufReader::new(&*input), BufWriter::new(&mut output));
        let services = state.services.clone();

        Self::_serve_codec_once(&mut codec, &services)
            .await
            .map_err(|e| actix_web::Error::from(e))?;

        // construct response
        Ok(web::Bytes::from(output))
    }

    #[cfg(feature = "actix-web")]
    async fn _handle_connect() -> Result<String, actix_web::Error> {
        Ok("CONNECT request is received".to_string())
    }

    #[cfg(any(feature = "actix-web", feature = "docs"))]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "actix-web")))]
    /// Configuration for integration with an actix-web scope.
    /// A convenient funciont "handle_http" may be used to achieve the same thing
    /// with the `actix-web` feature turned on.
    ///
    /// The `DEFAULT_RPC_PATH` will be appended to the end of the scope's path.
    ///
    /// This is enabled
    /// if and only if **exactly one** of the the following feature flag is turned on
    /// - `serde_bincode`
    /// - `serde_json`
    /// - `serde_cbor`
    /// - `serde_rmp`
    ///
    /// Example
    ///
    /// ```rust
    /// use toy_rpc::Server;
    /// use toy_rpc::macros::{export_impl, service};
    /// use actix_web::{App, HttpServer, web};
    ///
    /// struct FooService { }
    ///
    /// #[export_impl]
    /// impl FooService {
    ///     // define some "exported" functions
    /// }
    ///
    /// #[actix::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let addr = "127.0.0.1:8888";
    ///     
    ///     let foo_service = Arc::new(FooService { });
    ///
    ///     let server = Server::builder()
    ///         .register("foo_service", service!(foo_servicem FooService))
    ///         .build();
    ///
    ///     let app_data = web::Data::new(server);
    ///
    ///     HttpServer::new(
    ///         move || {
    ///             App::new()
    ///                 .service(
    ///                     web::scope("/rpc/")
    ///                         .app_data(app_data.clone())
    ///                         .configure(Server::scope_config)
    ///                         // The line above may be replaced with line below
    ///                         //.configure(Server::handle_http()) // use the convenience `handle_http`
    ///                 )
    ///         }
    ///     )
    ///     .bind(addr)?
    ///     .run()
    ///     .await
    /// }
    /// ```
    ///
    pub fn scope_config(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/")
                // .app_data(data)
                .service(
                    web::resource(super::DEFAULT_RPC_PATH)
                        .route(web::get().to(Server::_handle_http_ws))
                        .route(web::method(actix_web::http::Method::CONNECT).to(|| {
                            actix_web::HttpResponse::Ok().body("CONNECT request is received")
                        })),
                ),
        );
    }

    #[cfg(any(all(feature = "actix-web", not(feature = "tide"),), feature = "docs"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(all(feature = "actix-web", not(feature = "tide"))))
    )]
    /// A conevience function that calls the corresponding http handling
    /// function depending on the enabled feature flag
    ///
    /// | feature flag | function name  |
    /// | ------------ |---|
    /// | `tide`| [`into_endpoint`](#method.into_endpoint) |
    /// | `actix-web` | [`scope_config`](#method.scope_config) |
    ///
    /// This is enabled
    /// if and only if **exactly one** of the the following feature flag is turned on
    /// - `serde_bincode`
    /// - `serde_json`
    /// - `serde_cbor`
    /// - `serde_rmp`
    ///
    /// Example
    ///
    /// ```rust
    /// use toy_rpc::Server;
    /// use toy_rpc::macros::{export_impl, service};
    /// use actix_web::{App, web};
    ///
    /// struct FooService { }
    ///
    /// #[export_impl]
    /// impl FooService {
    ///     // define some "exported" functions
    /// }
    ///
    /// #[actix::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let addr = "127.0.0.1:8888";
    ///     
    ///     let foo_service = Arc::new(FooService { });
    ///
    ///     let server = Server::builder()
    ///         .register("foo_service", service!(foo_servicem FooService))
    ///         .build();
    ///
    ///     let app_data = web::Data::new(server);
    ///
    ///     HttpServer::new(
    ///         move || {
    ///             App::new()
    ///                 .service(hello)
    ///                 .service(
    ///                     web::scope("/rpc/")
    ///                         .app_data(app_data.clone())
    ///                         .configure(Server::handle_http()) // use the convenience `handle_http`
    ///                 )
    ///         }
    ///     )
    ///     .bind(addr)?
    ///     .run()
    ///     .await
    /// }
    /// ```
    pub fn handle_http() -> fn(&mut web::ServiceConfig) {
        Self::scope_config
    }

    async fn _handle_http_ws(
        state: web::Data<Server>, 
        req: HttpRequest, 
        stream: web::Payload
    ) -> Result<HttpResponse, actix_web::Error> 
    {
        let mut res = handshake(&req)?;
        let services = state.services.clone();

        let (chunk_sender, chunk_recver) = unbounded_channel::<Result<Bytes, actix_web::Error>>();
        let (resp_sender, resp_recver) = unbounded_channel::<Vec<u8>>();
        let (frame_sender, frame_recver) = unbounded_channel::<Vec<u8>>();

        let chunk_recver = UnboundedReceiverStream::new(chunk_recver);
        let resp_recver = UnboundedReceiverStream::new(resp_recver);
        let frame_recver = UnboundedReceiverStream::new(frame_recver);

        let _chunk_sender = chunk_sender.clone();
        actix_web::rt::spawn(
            async move {
                match Self::_encode_ws_resp(resp_recver, _chunk_sender).await {
                    Ok(()) => { },
                    Err(e) => {
                        log::debug!("{:?}", e);
                    }
                }
            }
        );
        
        actix_web::rt::spawn(
            async move {
                match Self::_serve_ws_message(services, frame_recver, resp_sender).await {
                    Ok(()) => { },
                    Err(e) => {
                        log::debug!("{:?}", e);
                    }
                }
            }
        );

        let _chunk_sender = chunk_sender.clone();
        actix_web::rt::spawn(
            async move {
                match Self::_decode_ws_frame(stream, frame_sender, _chunk_sender).await {
                    Ok(()) => { },
                    Err(e) => {
                        log::debug!("{:?}", e);
                    }
                }
            }
        );

        Ok(res.streaming(chunk_recver))
        // unimplemented!()
    }

    async fn _encode_ws_resp(
        mut resp_recver: UnboundedReceiverStream<Vec<u8>>, 
        chunk_sender: UnboundedSender<Result<Bytes, actix_web::Error>>
    ) -> Result<(), actix_web::Error> {
        while let Some(payload) = resp_recver.next().await {
            let mut ws_codec = actix_http::ws::Codec::new();
            let frame = Bytes::from(payload);
            let msg = actix_http::ws::Message::Binary(frame);
            let mut buf =  web::BytesMut::new();
            ws_codec.encode(msg, &mut buf)?;
            let resp = buf.freeze();
            chunk_sender.send(Ok(resp))
                .map_err(|e| Error::from(e))?;

        }

        Ok(())
    }

    async fn _serve_ws_message(
        services: Arc<AsyncServiceMap>,
        frame_recver: UnboundedReceiverStream<Vec<u8>>, 
        resp_sender: UnboundedSender<Vec<u8>>
    ) -> Result<(), actix_web::Error> {
        let codec = super::DefaultCodec::with_channels(frame_recver, resp_sender);
        Self::_serve_codec(codec, services).await
            .map_err(|e| actix_web::Error::from(e))
    }

    async fn _decode_ws_frame(
        mut stream: web::Payload, 
        frame_sender: UnboundedSender<Vec<u8>>,
        chunk_sender: UnboundedSender<Result<Bytes, actix_web::Error>>,
    ) -> Result<(), actix_web::Error> {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let mut ws_codec = actix_http::ws::Codec::new();
            let mut buf = web::BytesMut::new();
            buf.extend_from_slice(&chunk);

            log::debug!("WebSocket frame received");

            let frame = match ws_codec.decode(&mut buf)? {
                None => break,
                Some(f) => f
            };

            log::debug!("{:?}", frame);
           
            match frame {
                actix_http::ws::Frame::Text(_b) => {
                    break
                },
                actix_http::ws::Frame::Binary(b) => {
                    log::debug!("Sending Binary frame over channel");
                    frame_sender.send(b.to_vec())
                        .map_err(|e| Error::from(e))?;
                    // frame_sender.flush().await
                    //     .map_err(|e| Error::from(e))?;
                },
                actix_http::ws::Frame::Continuation(_) => { },
                actix_http::ws::Frame::Ping(b) => {
                    let msg = actix_http::ws::Message::Pong(b);
                    let mut pong_buf = web::BytesMut::new();
                    ws_codec.encode(msg, &mut pong_buf)?;
                    let pong = pong_buf.freeze();
                    chunk_sender.send(Ok(pong))
                        .map_err(|e| Error::from(e))?;
                },
                actix_http::ws::Frame::Pong(_) => { },
                actix_http::ws::Frame::Close(_) => {
                    break;
                }
            }
        }
        Ok(())
    }
}