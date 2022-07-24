use crate::{codec::{CodecRead, CodecWrite}, util::Running};

use super::{broker::ServerBroker, reader::ServerReader, writer::ServerWriter};

async fn event_loop<R, W>(
    broker: ServerBroker,
    reader: ServerReader<R>, 
    writer: ServerWriter<W>,
) 
where
    R: CodecRead,
    W: CodecWrite,
{
    loop {
        let running = futures::select! {
            running = reader.op() => {
                running
            }
        };

        match running {
            Running::Continue(_) => {},
            Running::Stop(_) => break
        }
    }
}