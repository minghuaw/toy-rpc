use std::{marker::PhantomData, pin::Pin, task::{Context, Poll}};
use pin_project::pin_project;
use futures::{Future, channel::oneshot};

use crate::{Error};

mod tokio;
mod async_std;

/// Type state for creating `Client`
pub struct NotConnected {}
/// Type state for creating `Client`
pub struct Connected {}

type ResponseBody = Box<dyn erased_serde::Deserializer<'static> + Send>;

#[pin_project]
pub struct Call<Res> {
    cancel: oneshot::Sender<()>,
    #[pin]
    done: oneshot::Receiver<Result<ResponseBody, ResponseBody>>,
    
    // return type
    marker: PhantomData<Res>
}

impl<Res> Call<Res> 
where 
    Res: serde::de::DeserializeOwned
{
    pub fn cancel(self) {
        unimplemented!()
    }
}

impl<Res> Future for Call<Res> 
where 
    Res: serde::de::DeserializeOwned
{
    type Output = Result<Res, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let done: Pin<&mut oneshot::Receiver<Result<ResponseBody, ResponseBody>>> = this.done;
        // let done = Pin::new(&mut self.done);

        match done.poll(cx) {
            Poll::Ready(result) => {
                match result {
                    Ok(val) => {
                        match val {
                            Ok(mut resp_body) => {
                                let resp = erased_serde::deserialize(&mut resp_body)
                                    .map_err(|e| Error::ParseError(Box::new(e)));
                                Poll::Ready(resp)
                            },
                            Err(mut err_body) => {
                                let err = erased_serde::deserialize(&mut err_body)
                                    .map_or_else(
                                        |e| Err(Error::ParseError(Box::new(e))), 
                                        |msg| Err(Error::from_err_msg(msg))
                                    );
                                Poll::Ready(err)
                            }
                        }
                    },
                    Err(canceled) => {
                        let err = Err(Error::Internal(
                            format!("Failed to read from done channel: {:?}", canceled).into()
                        ));
                        Poll::Ready(err)
                    }
                }
            },
            Poll::Pending => {
                Poll::Pending
            }
        }
    }
}

