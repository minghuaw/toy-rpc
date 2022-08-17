use flume::{Sender, Receiver};
use futures::FutureExt;

use crate::{util::Running, Error, codec::{CodecRead, CodecWrite}};

use super::{broker::{ClientBroker, ClientBrokerItem}, reader::ClientReader, writer::ClientWriter};


// pub struct ClientEngine<C, R, W> {
//     broker: ClientBroker<C>,
//     reader: ClientReader<R>,
//     writer: ClientWriter<W>,

//     tx: Sender<ClientBrokerItem>,
//     rx: Receiver<ClientBrokerItem>,
// }

// impl<C, R, W> ClientEngine<C, R, W> 
// where
//     R: CodecRead,
//     W: CodecWrite,
// {
//     pub(crate) fn new(
//         broker: ClientBroker<C>,
//         reader: ClientReader<R>,
//         writer: ClientWriter<W>
//     ) -> Self {
//         let (tx, rx) = flume::unbounded();
//         Self {
//             broker,
//             reader,
//             writer,
//             tx,
//             rx,
//         }
//     }

//     async fn event_loop_inner(&mut self) -> Result<Running, Error> {
//         futures::select! {
//             option = self.reader.op().fuse() => {
//                 match option {
//                     Some(result) => match result {

//                     },
//                     None => Ok(Running::Stop),
//                 }
//             }
//         }
//     }

//     pub(crate) async fn event_loop(&mut self) -> Result<(), Error> {
//         loop {
//             let running = self.event_loop_inner().await?;

//             match running {
//                 Running::Continue => {},
//                 Running::Stop => break,
//             }
//         }

//         Ok(())
//     }
// }