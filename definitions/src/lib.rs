use std::collections::HashMap;
use async_std::sync::Arc;
use erased_serde as erased;
use serde;

mod error;

pub use error::{
    Error,
    RpcError
};

pub type HandlerResult = Result<Box<dyn erased::Serialize + Send + Sync + 'static>, Error>;
pub type Handler<S> =
    Arc<dyn Fn(Arc<S>, &mut dyn erased::Deserializer) -> HandlerResult + Send + Sync + 'static>;

pub type ServeRequest =
    Arc<dyn Fn(&str, &mut (dyn erased::Deserializer + Send + Sync)) -> HandlerResult + Send + Sync>;
pub type ServiceMap = HashMap<&'static str, ServeRequest>;

pub trait IntoHandler<State, E, Req, Res> 
where
    E: ToString,
    Req: serde::de::DeserializeOwned,
    Res: serde::Serialize + Send + Sync + 'static,
    Self: Fn(&State, Req) -> Result<Res, E> + Send + Sync + Sized + 'static,
{
    fn into_handler(self) -> Handler<State> {
        let handler = move |_state: Arc<State>,
                            _deserializer: &mut dyn erased::Deserializer|
            -> HandlerResult {
            let _req: Req = erased::deserialize(_deserializer)
                .map_err(|_| Error::RpcError(RpcError::InvalidRequest))?;

            let res = self(&_state, _req)
                .map(|r| Box::new(r) as Box<dyn erased::Serialize + Send + Sync + 'static>)
                .map_err(|e| Error::RpcError(RpcError::ServerError(e.to_string())));

            return res;
        };

        Arc::new(handler)
    }
}

impl<F, E, Req, Res, State> IntoHandler<State, E, Req, Res> for F 
where 
    F: Fn(&State, Req) -> Result<Res, E> + Send + Sync + Sized + 'static,
    E: ToString,
    Req: serde::de::DeserializeOwned,
    Res: serde::Serialize + Send + Sync + 'static,
{

}

pub fn wrap<State, F, E, Req, Res>(method: F) -> Handler<State>
where
    F: Fn(&State, Req) -> Result<Res, E> + Send + Sync + 'static,
    E: ToString,
    Req: serde::de::DeserializeOwned,
    Res: serde::Serialize + Send + Sync + 'static,
{
    method.into_handler()
}