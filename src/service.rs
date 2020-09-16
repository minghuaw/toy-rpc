use std::collections::HashMap;
use std::marker::PhantomData;
use async_std::sync::Arc;
// use async_std::task;
use serde;
use erased_serde as erased;
use async_trait::async_trait;

use crate::error::{Error, RpcError};
// use crate::codec::ServerCodec;

pub(crate) type HandlerResult = Result<Box<dyn erased::Serialize + Send + Sync + 'static>, Error>;
pub type Handler<S> = dyn Fn(Arc<S>, &mut dyn erased::Deserializer) -> HandlerResult + Send + Sync + 'static;

pub struct Service<State> where State: Send + Sync + 'static {
    state: Arc<State>,
    handlers: HashMap<&'static str, Arc<Handler<State>>>
}

impl<State> Service<State> where State: Send + Sync {
    pub fn builder() -> ServiceBuilder<State, BuilderNotReady> {
        ServiceBuilder::new()
    }
}

#[async_trait]
pub trait HandleService<State> {
    fn get_state(&self) -> Arc<State>;
    fn get_method(&self, name: &str) -> Option<Arc<Handler<State>>>;

    fn call<'a>(&'a self, name: &str, _deserializer: &'a mut dyn erased::Deserializer) -> HandlerResult {
        let _state = self.get_state();
        let _method = match self.get_method(name) {
            Some(m) => m.clone(),
            None => {
                return Err(
                            Error::RpcError(
                                RpcError::MethodNotFound
                            )
                        )
            }
        };
            
        // execute hanlder method
        let res = _method(_state, _deserializer);
        
        // return handler result
        return res

    }
}

impl<State> HandleService<State> for Service<State> where State: Send + Sync + 'static {
    fn get_state(&self) -> Arc<State> {
        self.state.clone()
    }

    fn get_method(&self, name: &str) -> Option<Arc<Handler<State>>> {
        self.handlers.get(name)
            .map(|m| m.clone())
    }
}



/// Type state for the service builder when state is set
#[allow(dead_code)]
pub struct BuilderIsReady;
/// Type state for the service builder when state is NOT set
#[allow(dead_code)]
pub struct BuilderNotReady;

pub struct ServiceBuilder<State, BuilderMode>  where State: Send + Sync + 'static {
    pub state: Option<Arc<State>>,
    pub handlers: HashMap<&'static str, Arc<Handler<State>>>,

    // helper members for TypeState only
    // is_builder_ready: BuilderMode
    phantom: PhantomData<BuilderMode>
}

impl<State> ServiceBuilder<State, BuilderNotReady> where State: Send + Sync + 'static {
    pub fn new() -> ServiceBuilder<State, BuilderNotReady> {
        ServiceBuilder::<State, BuilderNotReady> {
            state: None,
            handlers: HashMap::new(),

            phantom: PhantomData
        }
    } 

    pub fn with_state(s: Arc<State>) -> ServiceBuilder<State, BuilderIsReady> {
        ServiceBuilder::<State, BuilderIsReady> {
            state: Some(s),
            handlers: HashMap::new(),

            phantom: PhantomData
        }
    }

    pub fn register_state(self, s: Arc<State>) -> ServiceBuilder<State, BuilderIsReady> {
        ServiceBuilder::<State, BuilderIsReady> {
            state: Some(s),
            handlers: self.handlers,

            phantom: PhantomData
        }
    }
}

impl<State, BuilderMode> ServiceBuilder<State, BuilderMode> where State: Send + Sync + 'static {
    pub fn register_method<'h, F, E, Req, Res>(self, name: &'static str, method: F) -> Self 
    where 
        F: Fn(&State, Req) -> Result<Res, E> + Send + Sync + 'static,
        E: ToString,
        Req: serde::de::DeserializeOwned,
        Res: serde::Serialize + Send + Sync + 'static
    {
        let handler = 
            move |_state: Arc<State>, _deserializer: &mut dyn erased::Deserializer| -> HandlerResult {                    
                let _req: Req = erased::deserialize(_deserializer)
                    .map_err(|_| Error::RpcError(RpcError::InvalidRequest))?;

                let res = method(&_state, _req)
                    .map(|r| Box::new(r) as Box<dyn erased::Serialize + Send + Sync + 'static>)
                    .map_err(|e| Error::RpcError(RpcError::ServerError(e.to_string())));

                return res
            };

        let mut handlers = self.handlers;
        handlers.insert(name, Arc::new(handler) );

        Self {
            handlers,
            ..self
        }
    }

    pub fn register_handlers(self, map: &'static HashMap<&'static str, Arc<Handler<State>>>) -> Self {
        let mut builder  = self;
        for (key, val) in map.iter() {
            builder.handlers.insert(key, val.clone());
        }

        builder
    }
}

impl<State> ServiceBuilder<State, BuilderIsReady> where State: Send + Sync + 'static {
    pub fn build(mut self) -> Service<State> {
        let handlers = self.handlers;
        let state = self.state.take().unwrap();

        Service {
            state,
            handlers
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    
    // #[derive(serde::Serialize, serde::Deserialize)]
    // struct Foo {
    //     id: u32,
    //     content: String
    // }

    // #[derive(serde::Serialize, serde::Deserialize)]
    // struct Bar {
    //     id: u32,
    //     content: String
    // }

    // struct FooBarState { }

    // impl FooBarState {
    //     fn foo_to_bar(&self, f: Foo) -> Result<Bar, String> {
    //         Ok(
    //             Bar {
    //                 id: f.id,
    //                 content: f.content
    //             }
    //         )
    //     }
    // }

    // #[test]
    // fn map_handler() {
    //     let map: HashMap<&str, Box<Handler<i32>>> = HashMap::new();

    // }

    // #[test]
    // fn type_state_builder() {
    //     let builder = ServiceBuilder::new();
    //     let builder = builder.register_state(Arc::new(FooBarState{ }))
    //         .register_method("f2b", FooBarState::foo_to_bar);
        
    //     let serv = builder.build();
    // }
}

