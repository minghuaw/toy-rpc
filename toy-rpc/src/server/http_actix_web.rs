//! Implements integration with `actix_web`

use cfg_if::cfg_if;
use actix::{Actor, ActorContext, AsyncContext, Context, Recipient, Running, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use futures::{FutureExt};
use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Duration, pin::Pin, future::Future};

use crate::{codec::{EraseDeserializer, Marshal, Unmarshal}, error::Error, message::{ErrorMessage, ExecutionMessage, ExecutionResult, MessageId, RequestHeader, ResponseHeader}, service::{AsyncServiceMap, HandlerResult}};

use super::{preprocess_header, preprocess_request, execute_call};

// =============================================================================
// `WsMessageActor`
// =============================================================================

/// Parse incoming and outgoing websocket messages and look up services
///
/// In the "Started" state, it will start a new `ExecutionManager`
/// actor. Upon reception of a request, the 
pub struct WsMessageActor<C> {
    services: Arc<AsyncServiceMap>,
    manager: Option<Recipient<ExecutionMessage>>,
    req_header: Option<RequestHeader>,
    marker: PhantomData<C>,
}

impl<C> Actor for WsMessageActor<C>
where
    C: Marshal + Unmarshal + Unpin + 'static,
{
    type Context = ws::WebsocketContext<Self>;

    /// Start a new `ExecutionManager`
    fn started(&mut self, ctx: &mut Self::Context) {
        log::debug!("WsMessageActor is started");
        let responder: Recipient<ExecutionResult> = ctx.address().recipient();
        let manager = ExecutionManager{ 
            responder,
            durations: HashMap::new(),
            executions: HashMap::new(),
        };
        let addr = manager.start();

        self.manager = Some(addr.recipient());
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        log::debug!("WsMessageActor is stopping");
        if let Some(ref manager) = self.manager {
            manager.do_send(ExecutionMessage::Stop)
                .unwrap_or_else(|err| log::error!("{:?}", err));
        }

        Running::Stop
    }
}

impl<C> StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsMessageActor<C>
where 
    C: Marshal + Unmarshal + EraseDeserializer + Unpin + 'static,
{
    fn handle(
        &mut self, 
        item: Result<ws::Message, ws::ProtocolError>, 
        ctx: &mut Self::Context
    ) {
        match item {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => { }
            Ok(ws::Message::Text(text)) => {
                log::error!(
                    "Received Text message: {} while expecting a binary message",
                    text
                );
            },
            Ok(ws::Message::Continuation(_)) => { },
            Ok(ws::Message::Nop) => { },
            Ok(ws::Message::Close(_)) => {
                log::debug!("Received closing message");
                ctx.stop();
            },
            Ok(ws::Message::Binary(buf)) => {
                match self.req_header.take() {
                    None => match C::unmarshal(&buf) {
                        Ok(h) => {
                            self.req_header.get_or_insert(h);
                        },
                        Err(err) => {
                            log::error!("Failed to unmarshal request header: {}", err);
                        }
                    },
                    Some(header) => {
                        let deserializer = C::from_bytes(buf.to_vec());
                        // let RequestHeader{ id, service_method } = header;

                        match preprocess_header(&header) {
                            Ok(req_type) => {
                                match preprocess_request(&self.services, req_type, deserializer) {
                                    Ok(msg) => {
                                        if let Some(ref manager) = self.manager {
                                            manager.do_send(msg)
                                                .unwrap_or_else(|e| log::error!("{:?}", e));
                                        }
                                    },
                                    Err(err) => {
                                        log::error!("{:?}", err);
                                        match err {
                                            Error::ServiceNotFound => {
                                                Self::send_response_via_context(
                                                    ExecutionResult {
                                                        id: header.id,
                                                        result: Err(Error::ServiceNotFound)
                                                    }, 
                                                    ctx
                                                ).unwrap_or_else(|e| log::error!("{:?}", e));
                                            },
                                            _ => { }
                                        }
                                    }
                                }
                            },
                            Err(err) => {
                                // the only error returned is MethodNotFound,
                                // which should be sent back to client
                                let err = ExecutionResult {
                                    id: header.id,
                                    result: Err(err)
                                };
                                Self::send_response_via_context(err, ctx)
                                    .unwrap_or_else(|e| log::error!("{:?}", e));
                            }
                        }
                    }
                }
            },
            Err(err) => {
                log::error!("{:?}", err);
            }
        }
    }
}

impl<C> actix::Handler<ExecutionResult> for WsMessageActor<C>
where 
    C: Marshal + Unmarshal + Unpin + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: ExecutionResult, ctx: &mut Self::Context) -> Self::Result {
        Self::send_response_via_context(msg, ctx)
            .unwrap_or_else(|err|                 
                log::error!("Error encountered sending response via context: {:?}", err)
            );
    }
}

impl<C> WsMessageActor<C>
where
    C: Marshal + Unmarshal + Unpin + 'static,
{
    fn send_response_via_context(
        res: ExecutionResult,
        ctx: &mut <Self as Actor>::Context,
    ) -> Result<(), Error> {
        let ExecutionResult { id, result } = res;
        match result {
            Ok(body) => {
                log::trace!("Message {} Success", &id);
                let header = ResponseHeader {
                    id,
                    is_error: false
                };
                let buf = C::marshal(&header)?;
                ctx.binary(buf);

                let buf = C::marshal(&body)?;
                ctx.binary(buf);
            }
            Err(err) => {
                log::trace!("Message {} Error", id.clone());
                let header = ResponseHeader { id, is_error: true };
                let msg = match ErrorMessage::from_err(err) {
                    Ok(m) => m,
                    Err(e) => {
                        log::error!("Cannot send back IoError or ParseError: {:?}", e);
                        return Err(e);
                    }
                };

                // compose error response header
                let buf = C::marshal(&header)?;
                ctx.binary(buf);
                let buf = C::marshal(&msg)?;
                ctx.binary(buf);
            }
        };

        Ok(())
    }
}

// =============================================================================
// `ExecutionManager`
// =============================================================================

struct Cancel(MessageId);

/// The `ExecutionManager` will manage spawning and stopping of new 
/// `ExecutionActor` 
struct ExecutionManager {
    responder: Recipient<ExecutionResult>,
    durations: HashMap<MessageId, Duration>,
    executions: HashMap<MessageId, flume::Sender<Cancel>>,
}

impl Actor for ExecutionManager {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        log::debug!("ExecutionManager is started");
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        log::debug!("ExecutionManager is stopping");
        for (id, exec) in self.executions.drain() {
            exec.send(Cancel(id)) 
                .unwrap_or_else(|e| log::error!("{:?}", e));
        }

        Running::Stop
    }
}

impl actix::Handler<ExecutionMessage> for ExecutionManager {
    type Result = ();

    fn handle(&mut self, msg: ExecutionMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ExecutionMessage::TimeoutInfo(id, duration) => {
                self.durations.insert(id, duration);
            },
            ExecutionMessage::Request{
                call,
                id,
                method,
                deserializer
            } => {
                let call_fut = call(method, deserializer);
                let broker = ctx.address().recipient();

                let fut: Pin<Box<dyn Future<Output=()>>> = match self.durations.remove(&id) {
                    Some(duration) => {
                        Box::pin(
                            async move {
                                let result = Self::execute_timed_call(id, duration, call_fut).await;
                                let result = ExecutionResult{ id, result };
                                broker.do_send(ExecutionMessage::Result(result))
                                    .unwrap_or_else(|e| log::error!("{:?}", e));
                            }
                        )
                    },
                    None => {
                        Box::pin(
                            async move {
                                let result = execute_call(id, call_fut).await;
                                let result = ExecutionResult { id, result };
                                broker.do_send(ExecutionMessage::Result(result))
                                    .unwrap_or_else(|e| log::error!("{:?}", e));
                            }
                        )
                    }
                };
                let (tx, rx) = flume::bounded(1);
                self.executions.insert(id, tx);
                
                actix::spawn(async move {
                    futures::select! {
                        _ = rx.recv_async().fuse() => { 
                            log::debug!("Future is canceled")
                        },
                        _ = fut.fuse() => {
                            log::debug!("Future is complete")
                        }
                    }
                });
            },
            ExecutionMessage::Result(msg) => {
                self.executions.remove(&msg.id);
                self.responder.do_send(msg)
                    .unwrap_or_else(|e| log::error!("{:?}", e));

            },
            ExecutionMessage::Cancel(id) => {
                log::debug!("Sending Cancel({})", &id);
                if let Some(exec) = self.executions.remove(&id) {
                    exec.send(Cancel(id))
                        .unwrap_or_else(|e| log::error!("{:?}", e));
                }
            },
            ExecutionMessage::Stop => {
                ctx.stop();
            }
        }
    }
}

impl ExecutionManager {
    // A separate implementation is required because actix-web still
    // depends on older version of tokio (actix-rt)
    async fn execute_timed_call(
        id: MessageId,
        duration: Duration,
        fut: impl Future<Output = HandlerResult>
    ) -> HandlerResult {
        match actix_rt::time::timeout(
            duration, 
            execute_call(id, fut)
        ).await {
            Ok(res) => res,
            Err(_) => Err(Error::Timeout(Some(id)))
        }
    }
}

// =============================================================================
// Integration
// =============================================================================
cfg_if! {
    if #[cfg(any(
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
    ))] {
        use crate::codec::{DefaultCodec, ConnTypePayload};
        use super::Server;
        
        async fn index(
            state: web::Data<Server>,
            req: HttpRequest,
            stream: web::Payload,
        ) -> Result<HttpResponse, actix_web::Error> {
            let services = state.services.clone();
            let ws_actor: WsMessageActor<DefaultCodec<Vec<u8>, Vec<u8>, ConnTypePayload>>
                = WsMessageActor {
                    services,
                    manager: None,
                    req_header: None,
                    marker: PhantomData,
                };
            ws::start(ws_actor, &req, stream)
        }

        impl Server {
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
            #[cfg(any(feature = "http_actix_web", feature = "docs"))]
            #[cfg_attr(feature = "docs", doc(cfg(feature = "http_actix_web")))]
            pub fn scope_config(cfg: &mut web::ServiceConfig) {
                cfg.service(
                    web::scope("/")
                        .service(
                            web::resource(crate::DEFAULT_RPC_PATH)
                                .route(web::get().to(index))
                        )
                );
            }

            /// A conevience function that calls the corresponding http handling
            /// function depending on the enabled feature flag
            ///
            /// | feature flag | function name  |
            /// | ------------ |---|
            /// | `http_tide`| [`into_endpoint`](#method.into_endpoint) |
            /// | `http_actix_web` | [`scope_config`](#method.scope_config) |
            /// | `http_warp` | [`into_boxed_filter`](#method.into_boxed_filter) |
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            #[cfg(any(all(feature = "http_actix_web", not(feature = "http_tide"),), feature = "docs"))]
            #[cfg_attr(
                feature = "docs",
                doc(cfg(all(feature = "http_actix_web", not(feature = "http_tide"))))
            )]
            pub fn handle_http() -> fn(&mut web::ServiceConfig) {
                Self::scope_config
            }
        }
    }
}