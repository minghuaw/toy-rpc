use flume::{Receiver, Sender};
use futures::FutureExt;

use crate::{
    codec::{CodecRead, CodecWrite},
    util::{GracefulShutdown, Running}, Error,
};

use super::{
    broker::{ServerBroker, ServerBrokerItem},
    reader::ServerReader,
    writer::ServerWriter,
};

pub struct ServerEngine<R, W> {
    broker: ServerBroker,
    reader: ServerReader<R>,
    writer: ServerWriter<W>,
    pending_tx: Sender<ServerBrokerItem>,
    pending_rx: Receiver<ServerBrokerItem>,
}

impl<R, W> ServerEngine<R, W>
where
    W: CodecWrite + GracefulShutdown,
{
    pub(crate) fn new(broker: ServerBroker, reader: ServerReader<R>, writer: ServerWriter<W>) -> Self {
        let (pending_tx, pending_rx) = flume::unbounded();

        Self {
            broker,
            reader,
            writer,
            pending_tx,
            pending_rx,
        }
    }

    async fn handle_broker_item(&mut self, item: ServerBrokerItem) -> Result<Running, Error> {
        match self.broker.op(item, &self.pending_tx).await {
            Err(err) => {
                log::error!("{:?}", err);
                Ok(Running::Stop)
            }
            Ok(Some(item)) => match self.writer.op(item).await {
                Ok(running) => Ok(running),
                Err(error) => self.writer.handle_error(error).await,
            },
            Ok(None) => Ok(Running::Continue),
        }
    }

    async fn event_loop_inner(&mut self) -> Result<Running, Error>
    where
        R: CodecRead,
        W: CodecWrite,
    {
        futures::select! {
            option = self.reader.op().fuse() => {
                match option {
                    Some(result) => match result {
                        Ok(item) => self.handle_broker_item(item).await,
                        Err(error) => self.reader.handle_error(error).await
                    },
                    None => Ok(Running::Stop),
                }
            },

            pending = self.pending_rx.recv_async() => {
                match pending {
                    Ok(item) => self.handle_broker_item(item).await,
                    Err(_) => {
                        // All sender must have been dropped which is impossible
                        Ok(Running::Stop)
                    },
                }
            }
        }
    }

    pub(crate) async fn event_loop(&mut self) -> Result<(), Error>
    where
        R: CodecRead,
        W: CodecWrite,
    {
        loop {
            let running = self.event_loop_inner().await?;

            match running {
                Running::Continue => {}
                Running::Stop => break,
            }
        }

        Ok(())
    }
}
