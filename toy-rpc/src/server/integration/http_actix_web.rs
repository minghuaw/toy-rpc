//! Implements integration with `actix_web`

use actix::{Actor, ActorContext, AsyncContext, Context, Recipient, Running, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use cfg_if::cfg_if;
use flume::Sender;
use futures::FutureExt;
use std::{
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use crate::{
    codec::{EraseDeserializer, Marshal, Unmarshal},
    error::Error,
    message::{ErrorMessage, MessageId},
    protocol::{Header, InboundBody},
    pubsub::SeqId,
    server::{
        broker::ServerBrokerItem,
        pubsub::{PubSubItem, PubSubResponder},
        reader::{handle_cancel, service},
        writer::ServerWriterItem,
        ClientId,
    },
    service::{ArcAsyncServiceCall, AsyncServiceMap, HandlerResult},
};

use crate::server::broker::execute_call;

// =============================================================================
// `WsMessageActor`
// =============================================================================

/// Parse incoming and outgoing websocket messages and look up services
///
/// In the "Started" state, it will start a new `ExecutionManager`
/// actor. Upon reception of a request, the
pub struct WsMessageActor<C> {
    client_id: ClientId,
    pubsub_broker: Sender<PubSubItem>,
    services: Arc<AsyncServiceMap>,
    manager: Option<Recipient<ServerBrokerItem>>,
    req_header: Option<Header>,
    marker: PhantomData<C>,
    // ack_mode: PhantomData<AckMode>,
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

// macro_rules! impl_ws_message_actor_for_ack_modes {
//     ($($ack_mode:ty),*) => {
//         $(
//             impl<C> Actor for WsMessageActor<C, $ack_mode>
//             where
//                 C: Marshal + Unmarshal + Unpin + 'static,
//             {
//                 type Context = ws::WebsocketContext<Self>;

//                 /// Start a new `ExecutionManager`
//                 fn started(&mut self, ctx: &mut Self::Context) {
//                     let responder: Recipient<ServerWriterItem> = ctx.address().recipient();
//                     let manager = ExecutionBroker::<$ack_mode> {
//                         client_id: self.client_id,
//                         responder,
//                         pubsub_broker: self.pubsub_broker.clone(),
//                         executions: HashMap::new(),

//                         ack_mode: PhantomData
//                     };
//                     let addr = manager.start();

//                     self.manager = Some(addr.recipient());
//                 }

//                 fn stopping(&mut self, _: &mut Self::Context) -> Running {
//                     self.send_to_manager(ServerBrokerItem::Stop);
//                     Running::Stop
//                 }
//             }

//             impl<C> StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsMessageActor<C, $ack_mode>
//             where
//                 C: Marshal + Unmarshal + EraseDeserializer + Unpin + 'static,
//             {
//                 fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
//                     match item {
//                         Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
//                         Ok(ws::Message::Pong(_)) => {}
//                         Ok(ws::Message::Text(text)) => {
//                             log::error!(
//                                 "Received Text message: {} while expecting a binary message",
//                                 text
//                             );
//                         }
//                         Ok(ws::Message::Continuation(_)) => {}
//                         Ok(ws::Message::Nop) => {}
//                         Ok(ws::Message::Close(_)) => {
//                             log::debug!("Received closing message");
//                             ctx.stop();
//                         }
//                         Ok(ws::Message::Binary(buf)) => match self.req_header.take() {
//                             None => match C::unmarshal(&buf) {
//                                 Ok(h) => {
//                                     self.req_header.get_or_insert(h);
//                                 }
//                                 Err(err) => {
//                                     log::error!("Failed to unmarshal request header: {}", err);
//                                 }
//                             },
//                             Some(header) => match header {
//                                 Header::Request {
//                                     id,
//                                     service_method,
//                                     timeout,
//                                 } => {
//                                     let deserializer = C::from_bytes(buf.to_vec());
//                                     match service(&self.services, service_method) {
//                                         Ok((call, method)) => {
//                                             let item = ServerBrokerItem::Request {
//                                                 call,
//                                                 id,
//                                                 method,
//                                                 duration: timeout,
//                                                 deserializer,
//                                             };
//                                             self.send_to_manager(item);
//                                         }
//                                         Err(err) => {
//                                             log::error!("{}", &err);
//                                             let item = ServerWriterItem::Response {
//                                                 id,
//                                                 result: Err(err),
//                                             };
//                                             Self::send_via_context(item, ctx)
//                                                 .unwrap_or_else(|err| log::error!("{}", err));
//                                         }
//                                     }
//                                 }
//                                 Header::Response { id, is_ok } => {
//                                     log::error!("Server received Response {{id: {}, is_ok: {}}}", id, is_ok);
//                                 }
//                                 Header::Cancel(id) => {
//                                     let deserializer = C::from_bytes(buf.to_vec());
//                                     match handle_cancel(id, deserializer) {
//                                         Ok(_) => {
//                                             let item = ServerBrokerItem::Cancel(id);
//                                             self.send_to_manager(item);
//                                         }
//                                         Err(err) => {
//                                             let item = ServerWriterItem::Response {
//                                                 id,
//                                                 result: Err(err),
//                                             };
//                                             Self::send_via_context(item, ctx)
//                                                 .unwrap_or_else(|err| log::error!("{}", err));
//                                         }
//                                     }
//                                 }
//                                 Header::Publish { id, topic } => {
//                                     let content = buf.to_vec();
//                                     self.send_to_manager(ServerBrokerItem::Publish { id, topic, content });
//                                 }
//                                 Header::Subscribe { id, topic } => {
//                                     self.send_to_manager(ServerBrokerItem::Subscribe { id, topic });
//                                 }
//                                 Header::Unsubscribe { id, topic } => {
//                                     self.send_to_manager(ServerBrokerItem::Unsubscribe { id, topic });
//                                 }
//                                 Header::Ack(_) => {}
//                                 Header::Produce { .. } => {}
//                                 Header::Consume { .. } => {}
//                                 Header::Ext { .. } => {}
//                             },
//                         },
//                         Err(err) => {
//                             log::error!("{}", err);
//                         }
//                     }
//                 }
//             }

//             impl<C> actix::Handler<ServerWriterItem> for WsMessageActor<C, $ack_mode>
//             where
//                 C: Marshal + Unmarshal + Unpin + 'static,
//             {
//                 type Result = ();

//                 fn handle(&mut self, msg: ServerWriterItem, ctx: &mut Self::Context) -> Self::Result {
//                     Self::send_via_context(msg, ctx).unwrap_or_else(|err| log::error!("{}", err));
//                 }
//             }

//             impl<C> WsMessageActor<C, $ack_mode>
//             where
//                 C: Marshal + Unmarshal + Unpin + 'static,
//             {
//                 fn send_via_context(
//                     item: ServerWriterItem,
//                     ctx: &mut <Self as Actor>::Context,
//                 ) -> Result<(), Error> {
//                     match item {
//                         ServerWriterItem::Response { id, result } => {
//                             match result {
//                                 Ok(body) => {
//                                     log::trace!("Message {} Success", &id);
//                                     let header = Header::Response { id, is_ok: true };
//                                     let buf = C::marshal(&header)?;
//                                     ctx.binary(buf);

//                                     let buf = C::marshal(&body)?;
//                                     ctx.binary(buf);
//                                 }
//                                 Err(err) => {
//                                     log::trace!("Message {} Error", id.clone());
//                                     let header = Header::Response { id, is_ok: false };
//                                     let msg = ErrorMessage::from_err(err)?;

//                                     // compose error response header
//                                     let buf = C::marshal(&header)?;
//                                     ctx.binary(buf);
//                                     let buf = C::marshal(&msg)?;
//                                     ctx.binary(buf);
//                                 }
//                             };
//                         }
//                         ServerWriterItem::Publication {
//                             seq_id,
//                             topic,
//                             content,
//                         } => {
//                             let id = seq_id.0;
//                             let header = Header::Publish { id, topic };
//                             let buf = C::marshal(&header)?;
//                             ctx.binary(buf);
//                             ctx.binary(content.to_vec());
//                         }
//                         ServerWriterItem::Ack { id } => {
//                             let header = Header::Ack(id);
//                             let buf = C::marshal(&header)?;
//                             // There is no body frame for Ack message
//                             ctx.binary(buf);
//                         }
//                         ServerWriterItem::Stopping => {
//                             ctx.close(None);
//                         },
//                         ServerWriterItem::Stop => {
//                             ctx.stop();
//                         }
//                     }

//                     Ok(())
//                 }
//             }
//         )*
//     }
// }

// impl_ws_message_actor_for_ack_modes!(AckModeNone, AckModeAuto);

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
            pubsub_broker: self.pubsub_broker.clone(),
            executions: HashMap::new(),
            // ack_mode: PhantomData
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
            Ok(ws::Message::Binary(buf)) => match self.req_header.take() {
                None => match C::unmarshal(&buf) {
                    Ok(h) => {
                        self.req_header.get_or_insert(h);
                    }
                    Err(err) => {
                        log::error!("Failed to unmarshal request header: {}", err);
                    }
                },
                Some(header) => match header {
                    Header::Request {
                        id,
                        service_method,
                        timeout,
                    } => {
                        let deserializer = C::from_bytes(buf.to_vec());
                        match service(&self.services, service_method) {
                            Ok((call, method)) => {
                                let item = ServerBrokerItem::Request {
                                    call,
                                    id,
                                    method,
                                    duration: timeout,
                                    deserializer,
                                };
                                self.send_to_manager(item);
                            }
                            Err(err) => {
                                log::error!("{}", &err);
                                let item = ServerWriterItem::Response {
                                    id,
                                    result: Err(err),
                                };
                                Self::send_via_context(item, ctx)
                                    .unwrap_or_else(|err| log::error!("{}", err));
                            }
                        }
                    }
                    Header::Response { id, is_ok } => {
                        log::error!("Server received Response {{id: {}, is_ok: {}}}", id, is_ok);
                    }
                    Header::Cancel(id) => {
                        let deserializer = C::from_bytes(buf.to_vec());
                        match handle_cancel(id, deserializer) {
                            Ok(_) => {
                                let item = ServerBrokerItem::Cancel(id);
                                self.send_to_manager(item);
                            }
                            Err(err) => {
                                let item = ServerWriterItem::Response {
                                    id,
                                    result: Err(err),
                                };
                                Self::send_via_context(item, ctx)
                                    .unwrap_or_else(|err| log::error!("{}", err));
                            }
                        }
                    }
                    Header::Publish { id, topic } => {
                        let content = buf.to_vec();
                        self.send_to_manager(ServerBrokerItem::Publish { id, topic, content });
                    }
                    Header::Subscribe { id, topic } => {
                        self.send_to_manager(ServerBrokerItem::Subscribe { id, topic });
                    }
                    Header::Unsubscribe { id, topic } => {
                        self.send_to_manager(ServerBrokerItem::Unsubscribe { id, topic });
                    }
                    Header::Ack(_) => {}
                    Header::Produce { .. } => {}
                    Header::Consume { .. } => {}
                    Header::Ext { .. } => {}
                },
            },
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
            ServerWriterItem::Response { id, result } => {
                match result {
                    Ok(body) => {
                        log::trace!("Message {} Success", &id);
                        let header = Header::Response { id, is_ok: true };
                        let buf = C::marshal(&header)?;
                        ctx.binary(buf);

                        let buf = C::marshal(&body)?;
                        ctx.binary(buf);
                    }
                    Err(err) => {
                        log::trace!("Message {} Error", id.clone());
                        let header = Header::Response { id, is_ok: false };
                        let msg = ErrorMessage::from_err(err)?;

                        // compose error response header
                        let buf = C::marshal(&header)?;
                        ctx.binary(buf);
                        let buf = C::marshal(&msg)?;
                        ctx.binary(buf);
                    }
                };
            }
            ServerWriterItem::Publication {
                seq_id,
                topic,
                content,
            } => {
                let id = seq_id.0;
                let header = Header::Publish { id, topic };
                let buf = C::marshal(&header)?;
                ctx.binary(buf);
                ctx.binary(content.to_vec());
            }
            // ServerWriterItem::Ack { id } => {
            //     let header = Header::Ack(id);
            //     let buf = C::marshal(&header)?;
            //     // There is no body frame for Ack message
            //     ctx.binary(buf);
            // }
            ServerWriterItem::Stopping => {
                ctx.close(None);
            }
            ServerWriterItem::Stop => {
                ctx.stop();
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
    pubsub_broker: Sender<PubSubItem>,
    executions: HashMap<MessageId, Sender<()>>,
    // ack_mode: PhantomData<AckMode>,
}

impl ExecutionBroker {
    fn handle_response(&mut self, id: MessageId, result: HandlerResult) -> Result<(), Error> {
        self.executions.remove(&id);
        let msg = ServerWriterItem::Response { id, result };
        self.responder.do_send(msg).map_err(|err| err.into())
    }

    fn handle_cancel(&mut self, id: MessageId) -> Result<(), Error> {
        if let Some(exec) = self.executions.remove(&id) {
            exec.send(())?
        }
        Ok(())
    }

    fn handle_publish_inner(
        &mut self,
        id: MessageId,
        topic: String,
        content: Vec<u8>,
    ) -> Result<(), Error> {
        let content = Arc::new(content);
        let msg = PubSubItem::Publish {
            client_id: self.client_id,
            msg_id: id,
            topic,
            content,
        };
        self.pubsub_broker.send(msg).map_err(|err| err.into())
    }

    fn handle_unsubscribe(&mut self, id: MessageId, topic: String) -> Result<(), Error> {
        log::debug!("Message ID: {}, Unsubscribe from topic: {}", &id, &topic);
        let msg = PubSubItem::Unsubscribe {
            client_id: self.client_id,
            topic,
        };
        self.pubsub_broker.send(msg).map_err(|err| err.into())
    }

    fn handle_publication(
        &mut self,
        seq_id: SeqId,
        topic: String,
        content: Arc<Vec<u8>>,
    ) -> Result<(), Error> {
        let msg = ServerWriterItem::Publication {
            seq_id,
            topic,
            content,
        };
        self.responder.do_send(msg).map_err(|err| err.into())
    }

    fn handle_inbound_ack(&mut self, seq_id: SeqId) -> Result<(), Error> {
        let msg = PubSubItem::Ack {
            seq_id,
            client_id: self.client_id,
        };
        self.pubsub_broker.send(msg).map_err(|err| err.into())
    }
}

impl ExecutionBroker {
    // Publish is the PubSub message from client to server
    fn handle_publish(
        &mut self,
        id: MessageId,
        topic: String,
        content: Vec<u8>,
    ) -> Result<(), Error> {
        self.handle_publish_inner(id, topic, content)
    }
}

// impl ExecutionBroker<AckModeAuto> {
//     // Publish is the PubSub message from client to server
//     fn handle_publish(
//         &mut self,
//         id: MessageId,
//         topic: String,
//         content: Vec<u8>,
//     ) -> Result<(), Error> {
//         self.handle_publish_inner(id, topic, content)?;
//         self.auto_ack(id)
//     }

//     fn auto_ack(&mut self, id: MessageId) -> Result<(), Error> {
//         self.responder
//             .do_send(ServerWriterItem::Ack { id })
//             .map_err(|err| err.into())
//     }
// }

impl Actor for ExecutionBroker {
    type Context = Context<Self>;

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        for (_, tx) in self.executions.drain() {
            tx.send(()).unwrap_or_else(|err| log::error!("{}", err));
        }

        Running::Stop
    }
}

// macro_rules! impl_execution_broker_for_ack_modes {
//     ($($ack_mode:ty),*) => {
//         $(
//             impl ExecutionBroker<$ack_mode> {
//                 fn handle_request(
//                     &mut self,
//                     ctx: &mut actix::Context<Self>,
//                     call: ArcAsyncServiceCall,
//                     id: MessageId,
//                     method: String,
//                     duration: Duration,
//                     deserializer: Box<InboundBody>,
//                 ) -> Result<(), Error> {
//                     let call_fut = call(method, deserializer);
//                     let broker = ctx.address().recipient();

//                     let fut: Pin<Box<dyn Future<Output = ()>>> = Box::pin(async move {
//                         let result = execute_timed_call(id, duration, call_fut).await;
//                         let item = ServerBrokerItem::Response { id, result };
//                         broker.do_send(item)
//                             .unwrap_or_else(|e| log::error!("{}", e));
//                     });
//                     let (tx, rx) = flume::bounded(1);
//                     self.executions.insert(id, tx);

//                     actix::spawn(async move {
//                         futures::select! {
//                             _ = rx.recv_async().fuse() => {
//                                 // log::debug!("Future is canceled")
//                             },
//                             _ = fut.fuse() => {
//                                 // log::debug!("Future is complete")
//                             }
//                         }
//                     });
//                     Ok(())
//                 }

//                 fn handle_subscribe(
//                     &mut self,
//                     ctx: &mut actix::Context<Self>,
//                     id: MessageId,
//                     topic: String,
//                 ) -> Result<(), Error> {
//                     log::debug!("Message ID: {}, Subscribe to topic: {}", &id, &topic);
//                     let sender = PubSubResponder::Recipient(ctx.address().recipient());
//                     let msg = PubSubItem::Subscribe {
//                         client_id: self.client_id,
//                         topic,
//                         sender,
//                     };
//                     self.pubsub_broker
//                         .send(msg)
//                         .map_err(|err| err.into())
//                 }
//             }

//             impl actix::Handler<ServerBrokerItem> for ExecutionBroker<$ack_mode> {
//                 type Result = ();

//                 fn handle(&mut self, msg: ServerBrokerItem, ctx: &mut Self::Context) -> Self::Result {
//                     let result = match msg {
//                         ServerBrokerItem::Request {
//                             call,
//                             id,
//                             method,
//                             duration,
//                             deserializer,
//                         } => {
//                             self.handle_request(ctx, call, id, method, duration, deserializer)
//                         }
//                         ServerBrokerItem::Response { id, result } => {
//                             self.handle_response(id, result)
//                         }
//                         ServerBrokerItem::Cancel(id) => {
//                             self.handle_cancel(id)
//                         }
//                         ServerBrokerItem::Publish { id, topic, content } => {
//                             self.handle_publish(id, topic, content)
//                         }
//                         ServerBrokerItem::Subscribe { id, topic } => {
//                             self.handle_subscribe(ctx, id, topic)
//                         }
//                         ServerBrokerItem::Unsubscribe { id, topic } => {
//                             self.handle_unsubscribe(id, topic)
//                         }
//                         ServerBrokerItem::Publication {
//                             seq_id,
//                             topic,
//                             content,
//                         } => {
//                             self.handle_publication(seq_id, topic, content)
//                         },
//                         ServerBrokerItem::InboundAck{seq_id} => {
//                             self.handle_inbound_ack(seq_id)
//                         },
//                         ServerBrokerItem::Stopping => {
//                             let msg = ServerWriterItem::Stopping;
//                             self.responder.do_send(msg).map_err(Into::into)
//                         },
//                         ServerBrokerItem::Stop => {
//                             let msg = ServerWriterItem::Stop;
//                             if let Err(err) = self.responder.do_send(msg) {
//                                 log::error!("{}", err);
//                             }
//                             ctx.stop();
//                             Ok(())
//                         }
//                     };

//                     if let Err(err) = result {
//                         log::error!("{}", err);
//                     }
//                 }
//             }

//         )*
//     };
// }

// impl_execution_broker_for_ack_modes!(AckModeNone, AckModeAuto);

impl ExecutionBroker {
    fn handle_request(
        &mut self,
        ctx: &mut actix::Context<Self>,
        call: ArcAsyncServiceCall,
        id: MessageId,
        method: String,
        duration: Duration,
        deserializer: Box<InboundBody>,
    ) -> Result<(), Error> {
        let call_fut = call(method, deserializer);
        let broker = ctx.address().recipient();

        let fut: Pin<Box<dyn Future<Output = ()>>> = Box::pin(async move {
            let result = execute_timed_call(id, duration, call_fut).await;
            let item = ServerBrokerItem::Response { id, result };
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
        Ok(())
    }

    fn handle_subscribe(
        &mut self,
        ctx: &mut actix::Context<Self>,
        id: MessageId,
        topic: String,
    ) -> Result<(), Error> {
        log::debug!("Message ID: {}, Subscribe to topic: {}", &id, &topic);
        let sender = PubSubResponder::Recipient(ctx.address().recipient());
        let msg = PubSubItem::Subscribe {
            client_id: self.client_id,
            topic,
            sender,
        };
        self.pubsub_broker.send(msg).map_err(|err| err.into())
    }
}

impl actix::Handler<ServerBrokerItem> for ExecutionBroker {
    type Result = ();

    fn handle(&mut self, msg: ServerBrokerItem, ctx: &mut Self::Context) -> Self::Result {
        let result = match msg {
            ServerBrokerItem::Request {
                call,
                id,
                method,
                duration,
                deserializer,
            } => self.handle_request(ctx, call, id, method, duration, deserializer),
            ServerBrokerItem::Response { id, result } => self.handle_response(id, result),
            ServerBrokerItem::Cancel(id) => self.handle_cancel(id),
            ServerBrokerItem::Publish { id, topic, content } => {
                self.handle_publish(id, topic, content)
            }
            ServerBrokerItem::Subscribe { id, topic } => self.handle_subscribe(ctx, id, topic),
            ServerBrokerItem::Unsubscribe { id, topic } => self.handle_unsubscribe(id, topic),
            ServerBrokerItem::Publication {
                seq_id,
                topic,
                content,
            } => self.handle_publication(seq_id, topic, content),
            ServerBrokerItem::InboundAck { seq_id } => self.handle_inbound_ack(seq_id),
            ServerBrokerItem::Stopping => {
                let msg = ServerWriterItem::Stopping;
                self.responder.do_send(msg).map_err(Into::into)
            }
            ServerBrokerItem::Stop => {
                let msg = ServerWriterItem::Stop;
                if let Err(err) = self.responder.do_send(msg) {
                    log::error!("{}", err);
                }
                ctx.stop();
                Ok(())
            }
        };

        if let Err(err) = result {
            log::error!("{}", err);
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
        Err(_) => Err(Error::Timeout(id)),
    }
}

// =============================================================================
// Integration
// =============================================================================

use crate::codec::{ConnTypePayload, DefaultCodec};
use crate::server::Server;

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
        // macro_rules! impl_actix_web_integration_for_ack_modes {
        //     ($($ack_mode:ty),*) => {
        //         $(
        //             impl Server<$ack_mode> {
        //                 async fn index(
        //                     state: web::Data<Server<$ack_mode>>,
        //                     req: HttpRequest,
        //                     stream: web::Payload,
        //                 ) -> Result<HttpResponse, actix_web::Error> {
        //                     let services = state.services.clone();
        //                     let client_id = state.client_counter.fetch_add(1, Ordering::Relaxed);
        //                     let pubsub_broker = state.pubsub_tx.clone();
        //                     let ws_actor: WsMessageActor<DefaultCodec<Vec<u8>, Vec<u8>, ConnTypePayload>, $ack_mode>
        //                         = WsMessageActor {
        //                             client_id,
        //                             pubsub_broker,
        //                             services,
        //                             manager: None,
        //                             req_header: None,
        //                             marker: PhantomData,
        //                             ack_mode: PhantomData,
        //                         };
        //                     ws::start(ws_actor, &req, stream)
        //                 }

        //                 /// Configuration for integration with an actix-web scope.
        //                 /// A convenient funciont "handle_http" may be used to achieve the same thing
        //                 /// with the `actix-web` feature turned on.
        //                 ///
        //                 /// The `DEFAULT_RPC_PATH` will be appended to the end of the scope's path.
        //                 ///
        //                 /// This is enabled
        //                 /// if and only if **exactly one** of the the following feature flag is turned on
        //                 /// - `serde_bincode`
        //                 /// - `serde_json`
        //                 /// - `serde_cbor`
        //                 /// - `serde_rmp`
        //                 ///
        //                 /// # Example
        //                 ///
        //                 /// ```
        //                 /// let example_service = Arc::new(Example { });
        //                 /// let server = Server::builder()
        //                 ///     .register(example_service)
        //                 ///     .build();
        //                 /// let app_data = web::Data::new(server);
        //                 ///
        //                 /// HttpServer::new(
        //                 ///     move || {
        //                 ///         App::new()
        //                 ///             .service(
        //                 ///                 web::scope("/rpc/")
        //                 ///                     .app_data(app_data.clone())
        //                 ///                     .configure(toy_rpc::Server::scope_config)
        //                 ///             )
        //                 ///     }
        //                 /// )
        //                 /// ```
        //                 #[cfg(any(feature = "http_actix_web", feature = "docs"))]
        //                 #[cfg_attr(feature = "docs", doc(cfg(feature = "http_actix_web")))]
        //                 pub fn scope_config(cfg: &mut web::ServiceConfig) {
        //                     cfg.service(
        //                         web::scope("/")
        //                             .service(
        //                                 web::resource(crate::DEFAULT_RPC_PATH)
        //                                     .route(web::get().to(Self::index))
        //                             )
        //                     );
        //                 }

        //                 /// A conevience function that calls the corresponding http handling
        //                 /// function depending on the enabled feature flag
        //                 ///
        //                 /// | feature flag | function name  |
        //                 /// | ------------ |---|
        //                 /// | `http_tide`| [`into_endpoint`](#method.into_endpoint) |
        //                 /// | `http_actix_web` | [`scope_config`](#method.scope_config) |
        //                 /// | `http_warp` | [`into_boxed_filter`](#method.into_boxed_filter) |
        //                 /// | `http_axum` | [`into_boxed_route`](#method.into_boxed_route) |
        //                 ///
        //                 /// This is enabled
        //                 /// if and only if **exactly one** of the the following feature flag is turned on
        //                 /// - `serde_bincode`
        //                 /// - `serde_json`
        //                 /// - `serde_cbor`
        //                 /// - `serde_rmp`
        //                 ///
        //                 /// # Example
        //                 ///
        //                 /// ```
        //                 /// let example_service = Arc::new(Example { });
        //                 /// let server = Server::builder()
        //                 ///     .register(example_service)
        //                 ///     .build();
        //                 /// let app_data = web::Data::new(server);
        //                 ///
        //                 /// HttpServer::new(
        //                 ///     move || {
        //                 ///         App::new()
        //                 ///             .service(
        //                 ///                 web::scope("/rpc/")
        //                 ///                     .app_data(app_data.clone())
        //                 ///                     .configure(toy_rpc::Server::handle_http())
        //                 ///             )
        //                 ///     }
        //                 /// )
        //                 /// ```
        //                 #[cfg(any(all(
        //                     feature = "http_actix_web",
        //                     not(feature = "http_tide"),
        //                     not(feature = "http_warp"),
        //                     not(feature = "http_axum"),
        //                 ), feature = "docs"))]
        //                 #[cfg_attr(
        //                     feature = "docs",
        //                     doc(cfg(all(
        //                         feature = "http_actix_web",
        //                         not(feature = "http_tide"),
        //                         not(feature = "http_warp"),
        //                         not(feature = "http_axum"),
        //                         not(feature = "http_warp"))))
        //                 )]
        //                 pub fn handle_http() -> fn(&mut web::ServiceConfig) {
        //                     Self::scope_config
        //                 }
        //             }
        //         )*
        //     }
        // }

        // impl_actix_web_integration_for_ack_modes!(AckModeNone);

        // #[cfg(not(feature = "docs"))]
        // impl_actix_web_integration_for_ack_modes!(AckModeAuto);

        impl Server {
            async fn index(
                state: web::Data<Server>,
                req: HttpRequest,
                stream: web::Payload,
            ) -> Result<HttpResponse, actix_web::Error> {
                let services = state.services.clone();
                let client_id = state.client_counter.fetch_add(1, Ordering::Relaxed);
                let pubsub_broker = state.pubsub_tx.clone();
                let ws_actor: WsMessageActor<DefaultCodec<Vec<u8>, Vec<u8>, ConnTypePayload>>
                    = WsMessageActor {
                        client_id,
                        pubsub_broker,
                        services,
                        manager: None,
                        req_header: None,
                        marker: PhantomData,
                        // ack_mode: PhantomData,
                    };
                ws::start(ws_actor, &req, stream)
            }

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
                                .route(web::get().to(Self::index))
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
            /// | `http_axum` | [`into_boxed_route`](#method.into_boxed_route) |
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
            #[cfg(any(all(
                feature = "http_actix_web",
                not(feature = "http_tide"),
                not(feature = "http_warp"),
                not(feature = "http_axum"),
            ), feature = "docs"))]
            #[cfg_attr(
                feature = "docs",
                doc(cfg(all(
                    feature = "http_actix_web",
                    not(feature = "http_tide"),
                    not(feature = "http_warp"),
                    not(feature = "http_axum"),
                    not(feature = "http_warp"))))
            )]
            pub fn handle_http() -> fn(&mut web::ServiceConfig) {
                Self::scope_config
            }
        }
    }
}
