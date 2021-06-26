use std::{
    collections::HashMap, time::Duration, sync::Arc,
};
use brw::{Running, Broker};
use futures::sink::{Sink, SinkExt};

use crate::{
    message::{MessageId, ExecutionMessage, ExecutionResult},
    error::Error
};

use super::handle_request;

#[cfg(any(
    feature = "docs",
    all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
))]
use ::tokio::task::JoinHandle;
#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use ::async_std::task::JoinHandle;

pub(crate) struct ServerBroker { 
    executions: HashMap<MessageId, JoinHandle<()>>,
    durations: HashMap<MessageId, Duration>
}

impl ServerBroker {
    pub fn new() -> Self {
        Self {
            executions: HashMap::new(),
            durations: HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl Broker for ServerBroker {
    type Item = ExecutionMessage;
    type WriterItem = ExecutionResult;
    type Ok = ();
    type Error = Error;

    async fn op<W>(&mut self, ctx: & Arc<brw::Context<Self::Item>>, item: Self::Item, mut writer: W) -> Running<Result<Self::Ok, Self::Error>>
    where W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin {
        match item {
            ExecutionMessage::TimeoutInfo(id, duration) => {
                self.durations.insert(id, duration);
                Running::Continue(Ok(()))
            },
            ExecutionMessage::Request{
                call,
                id,
                method,
                deserializer
            } => {
                let fut = call(method, deserializer);
                let _broker = ctx.broker.clone();
                let handle = handle_request(_broker, &mut self.durations, id, fut);
                self.executions.insert(id, handle);
                Running::Continue(Ok(()))
            },
            ExecutionMessage::Result(result) => {
                self.executions.remove(&result.id);
                let res: Result<(), Error> = writer.send(result).await
                    .map_err(|err| err.into());
                Running::Continue(res)
            },
            ExecutionMessage::Cancel(id) => {
                if let Some(handle) = self.executions.remove(&id) {
                    #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                    handle.abort();
                    #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                    handle.cancel().await;
                }

                Running::Continue(Ok(()))
            },
            ExecutionMessage::Stop => {
                for (_, handle) in self.executions.drain() {
                    log::debug!("Stopping execution as client is disconnected");
                    #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                    handle.abort();
                    #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                    handle.cancel().await;
                }
                log::debug!("Client connection is closed");
                Running::Stop
            }
        }
    }
}