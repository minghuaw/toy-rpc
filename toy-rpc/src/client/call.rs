//! RPC Call

use std::{marker::PhantomData, pin::Pin, task::{Context, Poll}};

use flume::Sender;
use futures::{Future, channel::oneshot};

use crate::{Error, message::MessageId, protocol::InboundBody};

use super::broker;

/// Call of a RPC request. The result can be obtained by `.await`ing the `Call`.
/// The call can be cancelled with `cancel()` method.
///
/// The type parameter `Res` is the `Ok` type of the result. `.await`ing on the `Call<Res>`
/// will yield a `Result<Res, toy_rpc::Error>`.
///
/// # Example
///
/// ```rust
/// // `.await` to wait for the response
/// let call: Call<i32> = client.call("Arith.add", (1i32, 6i32));
/// let result = call.await;
///
/// // cancel the call regardless of whether the response is received or not
/// let call: Call<()> = client.call("Arith.infinite_loop", ());
/// call.cancel();
/// // You can still .await on the canceled `Call` but will get an error
/// let result = call.await; // Err(Error::Canceled(Some(id)))
/// ```
#[pin_project::pin_project]
pub struct Call<Res> {
    pub(crate) id: MessageId,
    pub(crate) cancel: Sender<broker::ClientBrokerItem>,
    #[pin]
    pub(crate) done: oneshot::Receiver<Result<Result<Box<InboundBody>, Box<InboundBody>>, Error>>,
    pub(crate) marker: PhantomData<Res>,
}

impl<Res> Call<Res>
where
    Res: serde::de::DeserializeOwned,
{
    /// Cancel the RPC call
    ///
    pub fn cancel(&self) {
        match self.cancel.send(broker::ClientBrokerItem::Cancel(self.id)) {
            Ok(_) => {
                log::info!("Call is canceled");
            }
            Err(_) => {
                log::error!("Failed to cancel")
            }
        }
    }

    /// Gets the ID number of the call
    ///
    /// Each client RPC call has a monotonically increasing ID number of type `u16`
    pub fn get_id(&self) -> MessageId {
        self.id
    }
}

impl<Res> Future for Call<Res>
where
    Res: serde::de::DeserializeOwned,
{
    type Output = Result<Res, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let done: Pin<&mut oneshot::Receiver<Result<Result<Box<InboundBody>, Box<InboundBody>>, Error>>> = this.done;

        match done.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => {
                let res = match res {
                    Ok(val) => val,
                    Err(_canceled) => return Poll::Ready(Err(Error::Canceled(Some(*this.id)))),
                };

                let res = match res {
                    Ok(val) => val,
                    Err(err) => return Poll::Ready(Err(err))
                };

                let res = match res {
                    Ok(mut resp_body) => erased_serde::deserialize(&mut resp_body)
                        .map_err(|err| Error::ParseError(Box::new(err))),
                    Err(mut err_body) => erased_serde::deserialize(&mut err_body).map_or_else(
                        |err| Err(Error::ParseError(Box::new(err))),
                        |msg| Err(Error::from_err_msg(msg)),
                    ),
                };
                Poll::Ready(res)
            },
        }
    }
}
