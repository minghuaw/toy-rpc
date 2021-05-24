use actix::{Actor, AsyncContext, ContextFutureSpawner, Recipient, StreamHandler, WrapFuture};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::{marker::PhantomData, sync::Arc};

use crate::{codec::{Marshal, Unmarshal}, error::Error, message::{ExecutionMessage, ExecutionResult, MessageId, RequestHeader}, service::AsyncServiceMap};

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

    }
}

impl<C> actix::Handler<ExecutionResult> for WsMessageActor<C>
where 
    C: Marshal + Unmarshal + Unpin + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: ExecutionResult, ctx: &mut Self::Context) -> Self::Result {
        
    }
}

/// The `ExecutionManager` will manage spawning and stopping of new 
/// `ExecutionActor` 
struct ExecutionManager {
    responder: Recipient<ExecutionResult>
}

/// 
struct ExecutionActor { }

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
            Ok(body) => {}
            Err(err) => {}
        };

        Ok(())
    }
}
