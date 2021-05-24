use cfg_if::cfg_if;
use actix::{Actor, ActorContext, AsyncContext, Context, ContextFutureSpawner, Recipient, Running, StreamHandler, WrapFuture};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::{collections::HashMap, future::Future, marker::PhantomData, sync::Arc};

use crate::{codec::{EraseDeserializer, Marshal, Unmarshal}, error::Error, message::{ErrorMessage, ExecutionMessage, ExecutionResult, MessageId, RequestHeader, ResponseHeader}, service::{ArcAsyncServiceCall, AsyncServiceMap, HandlerResult}};

use super::preprocess_service_method;

// =============================================================================
// `WsMessageActor`
// =============================================================================

/// Parse incoming and outgoing websocket messages and look up services
///
/// In the "Started" state, it will start a new `ExecutionManager`
/// actor. Upon reception of a request, the 
pub struct WsMessageActor<C> {
    pub services: Arc<AsyncServiceMap>,
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
            executions: HashMap::new(),
        };
        let addr = manager.start();

        self.manager = Some(addr.recipient());
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        log::debug!("WsMessageActor is stopping");
        if let Some(ref manager) = self.manager {
            match manager.do_send(ExecutionMessage::Stop) {
                Ok(_) => { },
                Err(err) => {
                    log::error!("{:?}", err);
                }
            }
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
                        let RequestHeader{ id, service_method } = header;

                        let (service, method) = match preprocess_service_method(id, &service_method) {
                            Ok(pair) => pair,
                            Err(err) => {
                                match self.handle_preprocess_error(err, id, deserializer, ctx) {
                                    Ok(_) => { },
                                    Err(err) => {
                                        log::error!("{:?}", err)
                                    }
                                }
                                return;
                            }
                        };

                        let call: ArcAsyncServiceCall = match HashMap::get(&self.services, service) {
                            Some(serv_call) => serv_call.clone(),
                            None => {
                                let err = ExecutionResult{
                                    id,
                                    result: Err(Error::ServiceNotFound)
                                };
                                match Self::send_response_via_context(err, ctx) {
                                    Ok(_) => { },
                                    Err(err) => {
                                        log::error!("Error encountered sending response via context: {:?}", err)
                                    }
                                }
                                log::error!("Service not found: '{}'", service);
                                return;
                            }
                        };

                        // Send to ExecutorManager
                        if let Some(ref manager) = self.manager {
                            let msg = ExecutionMessage::Request{
                                call,
                                id,
                                method: method.into(),
                                deserializer
                            };
                            match manager.do_send(msg) {
                                Ok(_) => { },
                                Err(err) => {
                                    log::error!("{:?}", err)
                                }
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
        match Self::send_response_via_context(msg, ctx) {
            Ok(_) => { },
            Err(err ) => {
                log::error!("Error encountered sending response via context: {:?}", err);
            }
        }
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

    fn handle_preprocess_error<'de>(
        &mut self, 
        err: Error, 
        id: MessageId, 
        mut deserializer: Box<dyn erased_serde::Deserializer<'static> + Send>,
        ctx: &mut <Self as Actor>::Context
    ) -> Result<(), Error> {
        match err {
            Error::Canceled(_) => {
                let token: String = erased_serde::deserialize(&mut deserializer)?;
                if super::is_correct_cancellation_token(id, &token) {
                    if let Some(ref manager) = self.manager {
                        let msg = ExecutionMessage::Cancel(id);
                        match manager.do_send(msg) {
                            Ok(_) => { },
                            Err(err) => {
                                log::error!("{:?}", err)
                            }
                        } 
                    }
                }
            },
            Error::MethodNotFound => {
                let err = ExecutionResult {
                    id, 
                    result: Err(err)
                };
                Self::send_response_via_context(err, ctx)?;
            },

            // Note: not using `_` in case of mishanlding of new additions of Error types
            Error::IoError(_) => {},
            Error::ParseError(_) => {},
            Error::Internal(_) => {},
            Error::InvalidArgument => {},
            Error::ServiceNotFound => {},
            Error::ExecutionError(_) => {},
        }

        Ok(())
    }
}

// =============================================================================
// `ExecutionManager`
// =============================================================================

#[derive(Debug, actix::Message)]
#[rtype(result = "()")]
struct Cancel(MessageId);

/// The `ExecutionManager` will manage spawning and stopping of new 
/// `ExecutionActor` 
struct ExecutionManager {
    responder: Recipient<ExecutionResult>,
    executions: HashMap<MessageId, Recipient<Cancel>>
}

impl Actor for ExecutionManager {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        log::debug!("ExecutionManager is started");
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        log::debug!("ExecutionManager is stopping");
        for (id, exec) in self.executions.drain() {
            match exec.do_send(Cancel(id)) {
                Ok(_) => { },
                Err(err) => {
                    log::error!("{:?}", err);
                }
            }
        }

        Running::Stop
    }
}

impl actix::Handler<ExecutionMessage> for ExecutionManager {
    type Result = ();

    fn handle(&mut self, msg: ExecutionMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ExecutionMessage::Request{
                call,
                id,
                method,
                deserializer
            } => {
                let exec = ExecutionActor {
                    id,
                    manager: ctx.address().recipient(),
                };
                let recp = exec.start();
                
                let fut = call(method, deserializer);
                recp.do_send(ExecFut(fut));
                self.executions.insert(id, recp.recipient());
            },
            ExecutionMessage::Result(msg) => {
                self.executions.remove(&msg.id);
                match self.responder.do_send(msg){
                    Ok(_) => { },
                    Err(err) => {
                        log::error!("{:?}", err)
                    }
                }
            },
            ExecutionMessage::Cancel(id) => {
                if let Some(exec) = self.executions.remove(&id) {
                    match exec.do_send(Cancel(id)) {
                        Ok(_) => { },
                        Err(err) => log::error!("{:?}", err)
                    }
                }
            },
            ExecutionMessage::Stop => {
                ctx.stop();
            }
        }
    }
}

// =============================================================================
// `ExecutionActor`
// =============================================================================

#[derive(actix::Message)]
#[rtype(result = "()")]
struct ExecFut<F: Future<Output=HandlerResult> + Send + 'static>(F);

/// 
struct ExecutionActor { 
    id: MessageId,
    manager: Recipient<ExecutionMessage>,
}

impl Actor for ExecutionActor {
    type Context = Context<Self>;
}

impl<F: Future<Output=HandlerResult> + Send + 'static> actix::Handler<ExecFut<F>> for ExecutionActor {
    type Result = ();

    fn handle(&mut self, msg: ExecFut<F>, ctx: &mut Self::Context) -> Self::Result {
        log::debug!("Handling ExecFut");
        let id = self.id;
        let manager = self.manager.clone();
        let fut = async move {
            let res: HandlerResult = msg.0.await
                .map_err(|err| {
                    log::error!(
                        "Error found executing request id: {}, error msg: {}",
                        &id,
                        &err
                    );
                    match err {
                        // if serde cannot parse request, the argument is likely mistaken
                        Error::ParseError(e) => {
                            log::error!("ParseError {:?}", e);
                            Error::InvalidArgument
                        }
                        e @ _ => e,
                    }
                });
            
            let exec_result = ExecutionResult{
                id: id,
                result: res
            };
            match manager.do_send(ExecutionMessage::Result(exec_result)) {
                Ok(_) => { },
                Err(err) => {
                    log::error!("Error encountered while sending message to actor. Error: {:?}", err);
                }
            }
        };

        fut.into_actor(self).wait(ctx);
    }
}

impl actix::Handler<Cancel> for ExecutionActor {
    type Result = ();

    fn handle(&mut self, msg: Cancel, ctx: &mut Self::Context) -> Self::Result {
        if msg.0 == self.id {
            ctx.stop();
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
            let resp = ws::start(ws_actor, &req, stream);
            resp
        }

        impl Server {
            #[cfg(any(feature = "http_actix_web", feature = "docs"))]
            #[cfg_attr(feature = "docs", doc(cfg(feature = "http_actix_web")))]
            pub fn scope_config(cfg: &mut web::ServiceConfig) {
                cfg.service(
                    web::scope("/")
                        .service(
                            web::resource(super::DEFAULT_RPC_PATH)
                                .route(web::get().to(index))
                        )
                );
            }

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