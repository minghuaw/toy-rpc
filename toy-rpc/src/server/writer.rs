use brw::{Running, Writer};

use crate::{codec::split::ServerCodecWrite, error::Error, message::{ErrorMessage, ExecutionResult, ResponseHeader}};

pub(crate) struct ServerWriter<W> {
    writer: W,
}
impl<W: ServerCodecWrite> ServerWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer
        }
    }

    async fn write_one_message(&mut self, result: ExecutionResult) -> Result<(), Error> {
        let ExecutionResult { id, result } = result;

        match result {
            Ok(body) => {
                log::trace!("Message {} Success", &id);
                let header = ResponseHeader {
                    id,
                    is_error: false,
                };
                self.writer.write_response(header, &body).await?;
            }
            Err(err) => {
                log::trace!("Message {} Error", &id);
                let header = ResponseHeader { id, is_error: true };
                let msg = ErrorMessage::from_err(err)?;
                self.writer.write_response(header, &msg).await?;
            }
        };
        Ok(())
    }
}

#[async_trait::async_trait]
impl<W: ServerCodecWrite> Writer for ServerWriter<W> {
    type Item = ExecutionResult;
    type Ok = ();
    type Error = Error;

    async fn op(&mut self, item: Self::Item) -> Running<Result<Self::Ok, Self::Error>> {
        let res = self.write_one_message(item).await;
        Running::Continue(res)
    }

    async fn handle_result(res: Result<Self::Ok, Self::Error>) -> Running<()> {
        if let Err(err) = res {
            log::error!("{:?}", err);
        }
        Running::Continue(())
    }
}