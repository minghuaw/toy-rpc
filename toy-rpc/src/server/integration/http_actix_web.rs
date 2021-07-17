//! Implements integration with `actix_web`

use actix::{Actor, ActorContext, AsyncContext, Context, Recipient, Running, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use flume::Sender;
use cfg_if::cfg_if;
use futures::FutureExt;
use std::{collections::HashMap, future::Future, marker::PhantomData, pin::Pin, sync::{Arc, atomic::Ordering}, time::Duration};

use crate::{codec::{EraseDeserializer, Marshal, Unmarshal}, error::Error, message::{ErrorMessage, MessageId}, protocol::{Header}, server::{ClientId, broker::{ServerBrokerItem}, pubsub::PubSubItem, reader::{get_service, handle_cancel}, writer::ServerWriterItem}, service::{AsyncServiceMap, HandlerResult}};

use crate::server::{broker::execute_call};

// =============================================================================
// `WsMessageActor`
// =============================================================================

/// Parse incoming and outgoing websocket messages and look up services
///
/// In the "Started" state, it will start a new `ExecutionManager`
/// actor. Upon reception of a request, the
pub struct WsMessageActor<C> {
    client_id: ClientId,
    _pubsub_broker: Sender<PubSubItem>,
    services: Arc<AsyncServiceMap>,
    manager: Option<Recipient<ServerBrokerItem>>,
    req_header: Option<Header>,
    marker: PhantomData<C>,
}

impl<C> WsMessageActor<C> {
    fn send_to_manager(&self, item: ServerBrokerItem) {
        if let Some(ref manager) = self.manager {
            manager
                .do_send(item)
                .unwrap_or_else(|err| log::error!("{}", err));
        }
    }
}

impl<C> Actor for WsMessageActor<C>
where
    C: Marshal + Unmarshal + Unpin + 'static,
{
    type Context = ws::WebsocketContext<Self>;

    /// Start a new `ExecutionManager`
    fn started(&mut self, ctx: &mut Self::Context) {
        let responder: Recipient<ServerWriterItem> = ctx.address().recipient();
        let manager = ExecutionBroker {
            client_id: self.client_id,
            responder,
            executions: HashMap::new(),
        };
        let addr = manager.start();

        self.manager = Some(addr.recipient());
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.send_to_manager(ServerBrokerItem::Stop);
        Running::Stop
    }
}

impl<C> StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsMessageActor<C>
where
    C: Marshal + Unmarshal + EraseDeserializer + Unpin + 'static,
{
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Text(text)) => {
                log::error!(
                    "Received Text message: {} while expecting a binary message",
                    text
                );
            }
            Ok(ws::Message::Continuation(_)) => {}
            Ok(ws::Message::Nop) => {}
            Ok(ws::Message::Close(_)) => {
                log::debug!("Received closing message");
                ctx.stop();
            }
            Ok(ws::Message::Binary(buf)) => {
                match self.req_header.take() {
                    None => match C::unmarshal(&buf) {
                        Ok(h) => {
                            self.req_header.get_or_insert(h);
                        }
                        Err(err) => {
                            log::error!("Failed to unmarshal request header: {}", err);
                        }
                    },
                    Some(header) => {
                        // let deserializer = C::from_bytes(buf.to_vec());

                        // match preprocess_header(&header) {
                        //     Ok(req_type) => {
                        //         match preprocess_request(&self.services, req_type, deserializer) {
                        //             Ok(msg) => {
                        //                 if let Some(ref manager) = self.manager {
                        //                     manager
                        //                         .do_send(msg)
                        //                         .unwrap_or_else(|e| log::error!("{}", e));
                        //                 }
                        //             }
                        //             Err(err) => {
                        //                 log::error!("{}", err);
                        //                 match err {
                        //                     Error::ServiceNotFound => {
                        //                         Self::send_response_via_context(
                        //                             ExecutionResult {
                        //                                 id: header.id,
                        //                                 result: Err(Error::ServiceNotFound),
                        //                             },
                        //                             ctx,
                        //                         )
                        //                         .unwrap_or_else(|e| log::error!("{}", e));
                        //                     }
                        //                     _ => {}
                        //                 }
                        //             }
                        //         }
                        //     }
                        //     Err(err) => {
                        //         // the only error returned is MethodNotFound,
                        //         // which should be sent back to client
                        //         let err = ExecutionResult {
                        //             id: header.id,
                        //             result: Err(err),
                        //         };
                        //         Self::send_response_via_context(err, ctx)
                        //             .unwrap_or_else(|e| log::error!("{}", e));
                        //     }
                        // }

                        match header {
                            Header::Request{id, service_method, timeout} => {
                                let deserializer = C::from_bytes(buf.to_vec());
                                match get_service(&self.services, service_method) {
                                    Ok((call, method)) => {
                                        let item = ServerBrokerItem::Request {
                                            call,
                                            id,
                                            method,
                                            duration: timeout,
                                            deserializer
                                        };
                                        self.send_to_manager(item);
                                    },
                                    Err(err) => {
                                        log::error!("{}", &err);
                                        let item = ServerWriterItem::Response {id, result: Err(err) };
                                        Self::send_via_context(item, ctx)
                                            .unwrap_or_else(|err| log::error!("{}", err));
                                    }
                                }
                            },
                            Header::Response{id, is_ok } => {
                                log::error!("Server received Response {{id: {}, is_ok: {}}}", id, is_ok);
                            },
                            Header::Cancel(id) => {
                                let deserializer = C::from_bytes(buf.to_vec());
                                match handle_cancel(id, deserializer) {
                                    Ok(_) => {
                                        let item = ServerBrokerItem::Cancel(id);
                                        self.send_to_manager(item);
                                    },
                                    Err(err) => {
                                        let item = ServerWriterItem::Response{id, result: Err(err)};
                                        Self::send_via_context(item, ctx)
                                            .unwrap_or_else(|err| log::error!("{}", err));
                                    }
                                }
                            },
                            Header::Publish{id, topic} => {
                                let content = buf.to_vec();
                                self.send_to_manager(ServerBrokerItem::Publish{id, topic, content});
                            },
                            Header::Subscribe{id, topic} => {
                                self.send_to_manager(ServerBrokerItem::Subscribe{id, topic});
                            },
                            Header::Unsubscribe{id, topic} => {
                                self.send_to_manager(ServerBrokerItem::Unsubscribe{id, topic});
                            },
                            Header::Ack(_) => { },
                            Header::Produce{ .. } => { },
                            Header::Consume{ .. } => { },
                            Header::Ext { .. } => { },
                        }
                    }
                }
            }
            Err(err) => {
                log::error!("{}", err);
            }
        }
    }
}

impl<C> actix::Handler<ServerWriterItem> for WsMessageActor<C>
where
    C: Marshal + Unmarshal + Unpin + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: ServerWriterItem, ctx: &mut Self::Context) -> Self::Result {
        Self::send_via_context(msg, ctx).unwrap_or_else(|err| log::error!("{}", err));
    }
}

impl<C> WsMessageActor<C>
where
    C: Marshal + Unmarshal + Unpin + 'static,
{
    fn send_via_context(
        item: ServerWriterItem,
        ctx: &mut <Self as Actor>::Context,
    ) -> Result<(), Error> {
        match item {
            ServerWriterItem::Response{id, result} => {
                match result {
                    Ok(body) => {
                        log::trace!("Message {} Success", &id);
                        let header = Header::Response{id, is_ok: true};
                        let buf = C::marshal(&header)?;
                        ctx.binary(buf);
        
                        let buf = C::marshal(&body)?;
                        ctx.binary(buf);
                    }
                    Err(err) => {
                        log::trace!("Message {} Error", id.clone());
                        let header = Header::Response{id, is_ok: false};
                        let msg = ErrorMessage::from_err(err)?;
        
                        // compose error response header
                        let buf = C::marshal(&header)?;
                        ctx.binary(buf);
                        let buf = C::marshal(&msg)?;
                        ctx.binary(buf);
                    }
                };
            },
            ServerWriterItem::Publication{id, topic, content} => { 
                let header = Header::Publish{id, topic};
                let buf = C::marshal(&header)?;
                ctx.binary(buf);
                ctx.binary(content.to_vec());
            }
        }

        Ok(())
    }
}

// =============================================================================
// `ExecutionManager`
// =============================================================================

// struct Cancel(MessageId);

/// The `ExecutionManager` will manage spawning and stopping of new
/// `ExecutionActor`
struct ExecutionBroker {
    client_id: ClientId,
    responder: Recipient<ServerWriterItem>,
    // durations: HashMap<MessageId, Duration>,
    executions: HashMap<MessageId, Sender<()>>,
}

impl Actor for ExecutionBroker {
    type Context = Context<Self>;

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        for (_, tx) in self.executions.drain() {
            tx.send(())
                .unwrap_or_else(|err| log::error!("{}", err));
        }

        Running::Stop
    }
}

impl actix::Handler<ServerBrokerItem> for ExecutionBroker {
    type Result = ();

    fn handle(&mut self, msg: ServerBrokerItem, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ServerBrokerItem::Request {
                call,
                id,
                method,
                duration,
                deserializer,
            } => {
                let call_fut = call(method, deserializer);
                let broker = ctx.address().recipient();

                let fut: Pin<Box<dyn Future<Output = ()>>> = Box::pin(async move {
                        let result = execute_timed_call(id, duration, call_fut).await;
                        let item = ServerBrokerItem::Response{id, result};
                        broker
                            .do_send(item)
                            .unwrap_or_else(|e| log::error!("{}", e));
                    });
                let (tx, rx) = flume::bounded(1);
                self.executions.insert(id, tx);

                actix::spawn(async move {
                    futures::select! {
                        _ = rx.recv_async().fuse() => {
                            // log::debug!("Future is canceled")
                        },
                        _ = fut.fuse() => {
                            // log::debug!("Future is complete")
                        }
                    }
                });
            }
            ServerBrokerItem::Response{id, result} => {
                self.executions.remove(&id);
                let msg = ServerWriterItem::Response{id, result};
                self.responder
                    .do_send(msg)
                    .unwrap_or_else(|e| log::error!("{}", e));
            }
            ServerBrokerItem::Cancel(id) => {
                log::debug!("Sending Cancel({})", &id);
                if let Some(exec) = self.executions.remove(&id) {
                    exec.send(())
                        .unwrap_or_else(|e| log::error!("{}", e));
                }
            },
            ServerBrokerItem::Publish { .. } => {
                log::debug!("Publish client_id: {}", self.client_id);
                log::error!("PubSub on actix not implemented");
            },
            ServerBrokerItem::Subscribe { .. } => {
                log::error!("PubSub on actix not implemented");
            },
            ServerBrokerItem::Unsubscribe { .. } => {
                log::error!("PubSub on actix not implemented");
            },
            ServerBrokerItem::Publication{ .. } => {
                log::error!("PubSub on actix not implemented");
            },
            ServerBrokerItem::Stop => {
                ctx.stop();
            }
        }
    }
}


// A separate implementation is required because actix-web still
// depends on older version of tokio (actix-rt)
async fn execute_timed_call(
    id: MessageId,
    duration: Duration,
    fut: impl Future<Output = HandlerResult>,
) -> HandlerResult {
    match actix_rt::time::timeout(duration, execute_call(id, fut)).await {
        Ok(res) => res,
        Err(_) => Err(Error::Timeout(Some(id))),
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
        use crate::server::Server;

        async fn index(
            state: web::Data<Server>,
            req: HttpRequest,
            stream: web::Payload,
        ) -> Result<HttpResponse, actix_web::Error> {
            let services = state.services.clone();
            let client_id = state.client_counter.fetch_add(1, Ordering::Relaxed);
            let _pubsub_broker = state.pubsub_tx.clone();
            let ws_actor: WsMessageActor<DefaultCodec<Vec<u8>, Vec<u8>, ConnTypePayload>>
                = WsMessageActor {
                    client_id,
                    _pubsub_broker,
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
            ///
            /// # Example 
            ///
            /// ```
            /// let example_service = Arc::new(Example { });
            /// let server = Server::builder()
            ///     .register(example_service)
            ///     .build();
            /// let app_data = web::Data::new(server);
            /// 
            /// HttpServer::new(
            ///     move || {
            ///         App::new()
            ///             .service(
            ///                 web::scope("/rpc/")
            ///                     .app_data(app_data.clone())
            ///                     .configure(toy_rpc::Server::scope_config)
            ///             )
            ///     }
            /// )
            /// ```
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
            ///
            /// # Example 
            ///
            /// ```
            /// let example_service = Arc::new(Example { });
            /// let server = Server::builder()
            ///     .register(example_service)
            ///     .build();
            /// let app_data = web::Data::new(server);
            /// 
            /// HttpServer::new(
            ///     move || {
            ///         App::new()
            ///             .service(
            ///                 web::scope("/rpc/")
            ///                     .app_data(app_data.clone())
            ///                     .configure(toy_rpc::Server::handle_http())
            ///             )
            ///     }
            /// )
            /// ```
            #[cfg(any(all(feature = "http_actix_web", not(feature = "http_tide"),), feature = "docs"))]
            #[cfg_attr(
                feature = "docs",
                doc(cfg(all(feature = "http_actix_web", not(feature = "http_tide"), not(feature = "http_warp"))))
            )]
            pub fn handle_http() -> fn(&mut web::ServiceConfig) {
                Self::scope_config
            }
        }
    }
}
