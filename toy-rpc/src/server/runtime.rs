use flume::{Sender, Receiver};
use futures::FutureExt;

use crate::{codec::{CodecRead, CodecWrite}, util::{Running, GracefulShutdown}};

use super::{broker::{ServerBroker, ServerBrokerItem}, reader::ServerReader, writer::ServerWriter};

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
    async fn handle_broker_item(&mut self, item: ServerBrokerItem) -> Running {
        match self.broker.op(item, &self.pending_tx).await {
            Err(err) => {
                log::error!("{:?}", err);
                Running::Stop
            },
            Ok(Some(item)) => {
                match self.writer.op(item).await {
                    Ok(running) => running,
                    Err(error) => self.writer.handle_error(error).await,
                }
            }
            Ok(None) => Running::Continue,
        }
    }
    
    async fn event_loop_inner(
        &mut self,
    ) -> Running
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
                    None => Running::Stop,
                }
            },

            pending = self.pending_rx.recv_async() => {
                match pending {
                    Ok(item) => self.handle_broker_item(item).await,
                    Err(_) => {
                        // All sender must have been dropped which is impossible
                        Running::Stop
                    },
                }
            }
        }
    }
    
    async fn event_loop(
        &mut self
    ) 
    where
        R: CodecRead,
        W: CodecWrite,
    {
        loop {
            let running = self.event_loop_inner().await;
    
            match running {
                Running::Continue => {},
                Running::Stop => break
            }
        }
    }

}