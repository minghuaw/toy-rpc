//! Request

use std::{sync::Arc, time::Duration};

use crate::{Error, message::MessageId, protocol::InboundBody, service::{ArcAsyncServiceCall, AsyncHandler, HandlerResult}};

pub struct Context {

}

impl Context {
    pub fn id(&self) -> &MessageId {
        todo!()
    }
}

pub struct Request {
    call: ArcAsyncServiceCall,
    method: String,
    arg: Box<InboundBody>,
    duration: Duration,
    context: Arc<Context>,
}

impl Request {
    async fn execute_with_timeout(self) -> HandlerResult {
        let Self {
            call,
            method,
            arg,
            duration,
            context
        } = self;

        #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime"),))]
        match ::tokio::time::timeout(duration, call(method, arg)).await {
            Ok(res) => res,
            Err(err) => {
                let id = context.id();
                log::error!("Request {} reached timeout (err: {})", id, err);
                Err(Error::Timeout(*id))
            }
        }

        #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
        match ::async_std::future::timeout(duration, call(method, arg)).await {
            Ok(res) => res,
            Err(err) => {
                let id = context.id();
                log::error!("Request {} reached timeout (err: {})", id, err);
                Err(Error::Timeout(*id))
            }
        }
    }
}

