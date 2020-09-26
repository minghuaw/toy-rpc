use async_std::sync::Arc;
use async_trait::async_trait;
use erased_serde as erased;
use serde;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use futures::future::Future;

use crate::{Error, RpcError};

// async versions of handlers
pub type HandlerResult = Result<Box<dyn erased::Serialize + Send + Sync + 'static>, Error>;
pub type HandlerResultFut = Pin<Box<dyn Future<Output=HandlerResult>>>;
pub type AsyncHandler<S> = 
    dyn Fn(&'static S, Box<dyn erased::Deserializer<'static>>) -> HandlerResultFut + Send + Sync + 'static;
pub type ArcAsyncHandler<S> = Arc<AsyncHandler<S>>;

// pub trait IntoAsyncHandler<State, Fut, Req, Res, E>
// where 
//     State: Send + Sync + 'static,
//     Fut: Future<Output=Result<Res, E>>,
//     Req: serde::de::DeserializeOwned,
//     Res: serde::Serialize + Send + Sync + 'static,
//     E: ToString,
//     Self: Fn(&State, Req) -> Fut + Send + Sync + Sized + 'static,
// {
//     fn into_async_handler(&'static self) -> AsyncHandler<State>;
// }

// impl<State, F, Fut, Req, Res, E> IntoAsyncHandler<State, Fut, Req, Res, E> for F 
// where 
//     State: Send + Sync + 'static,
//     F: Fn(&State, Req) -> Fut + Send + Sync + Sized + 'static,
//     Fut: Future<Output=Result<Res, E>>,
//     Req: serde::de::DeserializeOwned,
//     Res: serde::Serialize + Send + Sync + 'static,
//     E: ToString,
// {
//     fn into_async_handler(&'static self) -> AsyncHandler<State> {
//         let handler = 
//             move |state: &State, mut deserializer: Box<dyn erased::Deserializer>| 
//             -> Pin<Box<dyn Future<Output=HandlerResult>>>
//         {
//             Box::pin(
//                 async move {
//                     let _req: Req = erased::deserialize(deserializer.as_mut())
//                         .map_err(|_| Error::RpcError(RpcError::InvalidRequest))?;

//                     let res = self(&state, _req).await
//                         .map(|r| Box::new(r) as Box<dyn erased::Serialize + Send + Sync + 'static>)
//                         .map_err(|e| Error::RpcError(RpcError::ServerError(e.to_string())));
                    
//                     res
//                 }
//             )   
//         };

//         Arc::new(handler)
//     }
// }
