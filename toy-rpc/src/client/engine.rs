use super::{broker::ClientBroker, reader::ClientReader, writer::ClientWriter};


pub struct ClientEngine<C, R, W> {
    broker: ClientBroker<C>,
    reader: ClientReader<R>,
    writer: ClientWriter<W>,
}