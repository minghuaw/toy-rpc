//! RPC Call

use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use flume::Sender;
use futures::{channel::oneshot, Future};
use serde::de::DeserializeOwned;

use crate::{message::MessageId, protocol::InboundBody, Error};

use super::{broker, ResponseResult};

enum CallStatus {
    Pending,
    Canceled,
    Received,
    Dropped,
}

/// Call of a RPC request. The result can be obtained by `.await`ing the `Call`.
/// The call can be cancelled with `cancel()` method.
///
/// The type parameter `Res` is the `Ok` type of the result. `.await`ing on the `Call<Res>`
/// will yield a `Result<Res, toy_rpc::Error>`. If a `Call` is dropped before the value is consumed
/// by `.await`ing, the call will be canceled.
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
#[pin_project::pin_project(PinnedDrop)]
pub struct Call<Res: DeserializeOwned> {
    status: CallStatus,
    id: MessageId,
    cancel: Sender<broker::ClientBrokerItem>,
    #[pin]
    done: oneshot::Receiver<Result<ResponseResult, Error>>,
    marker: PhantomData<Res>,
}

impl<Res: DeserializeOwned> Call<Res> {
    pub(crate) fn new(
        id: MessageId,
        cancel: Sender<broker::ClientBrokerItem>,
        done: oneshot::Receiver<Result<ResponseResult, Error>>,
    ) -> Self {
        Self {
            status: CallStatus::Pending,
            id,
            cancel,
            done,
            marker: PhantomData,
        }
    }
}

#[pin_project::pinned_drop]
impl<Res: DeserializeOwned> PinnedDrop for Call<Res> {
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();
        if let CallStatus::Pending = this.status {
            if let Err(_) = this.cancel.send(broker::ClientBrokerItem::Cancel(*this.id)) {
                log::error!("Failed to send cancellation message to client broker");
            }
        }
        *this.status = CallStatus::Dropped;
    }
}

impl<Res: DeserializeOwned> Call<Res> {
    /// Cancel the RPC call
    ///
    pub fn cancel(&mut self) {
        self.status = CallStatus::Canceled;
        if let Err(_) = self.cancel.send(broker::ClientBrokerItem::Cancel(self.id)) {
            log::error!("Failed to send cancellation message to client broker");
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
        let done: Pin<
            &mut oneshot::Receiver<Result<Result<Box<InboundBody>, Box<InboundBody>>, Error>>,
        > = this.done;

        match done.poll(cx) {
            Poll::Pending => match this.status {
                CallStatus::Canceled | CallStatus::Dropped => {
                    Poll::Ready(Err(Error::Canceled(*this.id)))
                }
                _ => Poll::Pending,
            },
            Poll::Ready(res) => {
                match this.status {
                    CallStatus::Canceled | CallStatus::Dropped => {
                        return Poll::Ready(Err(Error::Canceled(*this.id)))
                    }
                    _ => {}
                }

                let res = match res {
                    Ok(val) => val,
                    Err(_canceled) => return Poll::Ready(Err(Error::Canceled(*this.id))),
                };
                let res = match res {
                    Ok(val) => val,
                    Err(err) => return Poll::Ready(Err(err)),
                };
                let res = match res {
                    Ok(mut resp_body) => erased_serde::deserialize(&mut resp_body)
                        .map_err(|err| Error::ParseError(Box::new(err))),
                    Err(mut err_body) => erased_serde::deserialize(&mut err_body).map_or_else(
                        |err| Err(Error::ParseError(Box::new(err))),
                        |msg| Err(Error::from_err_msg(msg)),
                    ),
                };

                *this.status = CallStatus::Received;
                Poll::Ready(res)
            }
        }
    }
}
