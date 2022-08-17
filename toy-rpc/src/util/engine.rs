use flume::{Sender, Receiver};
use futures::FutureExt;

use crate::Error;

use super::{Broker, Reader, Writer, Running};


pub(crate) struct Engine<B, R, W, BI> {
    broker: B,
    reader: R,
    writer: W,
    pending_tx: Sender<BI>,
    pending_rx: Receiver<BI>,
}

impl<B, R, W, BI> Engine<B, R, W, BI>
where
    B: Broker<Item = BI, WriterItem = W::Item> + Send,
    R: Reader<BrokerItem = B::Item> + Send,
    W: Writer + Send,
{
    pub(crate) fn new(broker: B, reader: R, writer: W) -> Self {
        let (pending_tx, pending_rx) = flume::unbounded();

        Self {
            broker,
            reader,
            writer,
            pending_tx,
            pending_rx,
        }
    }

    async fn handle_broker_item(&mut self, item: BI) -> Result<Running, Error> {
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

    async fn event_loop_inner(&mut self) -> Result<Running, Error> {
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

    pub(crate) async fn event_loop(&mut self) -> Result<(), Error> {
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
