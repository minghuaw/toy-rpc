//! Request

use std::{sync::Arc, time::Duration};

use futures::Sink;
#[cfg(all(
    feature = "tokio_runtime",
    not(feature = "async_std_runtime"),
    not(feature = "http_actix_web")
))]
use tokio::task::{spawn, JoinHandle};

use crate::{Error, context::Context, message::MessageId, protocol::InboundBody, server::broker::ServerBrokerItem, service::{ArcAsyncServiceCall, AsyncHandler, ServiceCallResult}};

pub struct Request {
    call: ArcAsyncServiceCall,
    method: String,
    arg: Box<InboundBody>,
    duration: Duration,
    context: Arc<Context>,
}

impl Request {
    async fn execute_with_timeout(self) -> ServiceCallResult {
        let Self {
            call,
            method,
            arg,
            duration,
            context,
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

    fn spawn_execution_with_timeout(self, responder: impl Sink<ServerBrokerItem>) -> JoinHandle<()> {
        todo!()
    }
}
