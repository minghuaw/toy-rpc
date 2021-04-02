//!  Service builder and registration

use async_trait::async_trait;
use erased_serde as erased;
use futures::future::Future;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::Error;

// async versions of handlers
pub(crate) type Success = Box<dyn erased::Serialize + Send + Sync + 'static>;
pub type HandlerResult = Result<Success, Error>;
pub type HandlerResultFut = Pin<Box<dyn Future<Output = HandlerResult> + Send>>;
pub type AsyncHandler<S> = fn(Arc<S>, Box<dyn erased::Deserializer<'static> + Send>) -> HandlerResultFut;
// pub type ArcAsyncHandler<S> = Arc<AsyncHandler<S>>;

// trait objects to invoke a service
pub type AsyncServiceCall = dyn Fn(String, Box<dyn erased::Deserializer<'static> + Send>) -> HandlerResultFut
    + Send
    + Sync
    + 'static;
pub type ArcAsyncServiceCall = Arc<AsyncServiceCall>;
pub type AsyncServiceMap = HashMap<&'static str, ArcAsyncServiceCall>;

pub struct Service<State>
where
    State: Send + Sync + 'static,
{
    state: Arc<State>,
    handlers: HashMap<&'static str, AsyncHandler<State>>,
}

impl<State> Service<State>
where
    State: Send + Sync + 'static,
{
    pub fn builder() -> ServiceBuilder<State, BuilderUninitialized> {
        ServiceBuilder::new()
    }
}

#[async_trait]
pub trait HandleService<State>
where
    State: Send + Sync + 'static,
{
    fn get_state(&self) -> Arc<State>;
    fn get_method(&self, name: &str) -> Option<AsyncHandler<State>>;

    fn call(
        &self,
        name: &str,
        deserializer: Box<dyn erased::Deserializer<'static> + Send>,
    ) -> HandlerResultFut {
        let _state = self.get_state();
        match self.get_method(name) {
            Some(m) => m(_state, deserializer),
            None => Box::pin(async move { Err(Error::MethodNotFound) }),
        }
    }
}

impl<State> HandleService<State> for Service<State>
where
    State: Send + Sync + 'static,
{
    fn get_state(&self) -> Arc<State> {
        self.state.clone()
    }

    fn get_method(&self, name: &str) -> Option<AsyncHandler<State>> {
        // self.handlers.get(name).map(|m| m.clone())
        self.handlers.get(name).cloned()
    }
}

#[allow(dead_code)]
pub struct BuilderUninitialized;
pub struct BuilderReady;

pub struct ServiceBuilder<State, BuilderMode>
where
    State: Send + Sync + 'static,
{
    pub state: Option<Arc<State>>,
    pub handlers: HashMap<&'static str, AsyncHandler<State>>,

    // helper members for TypeState only
    mode: PhantomData<BuilderMode>,
}

impl<State> ServiceBuilder<State, BuilderUninitialized>
where
    State: Send + Sync + 'static,
{
    pub fn new() -> ServiceBuilder<State, BuilderUninitialized> {
        ServiceBuilder::<State, BuilderUninitialized> {
            state: None,
            handlers: HashMap::new(),

            mode: PhantomData,
        }
    }

    pub fn with_state(s: Arc<State>) -> ServiceBuilder<State, BuilderReady> {
        ServiceBuilder::<State, BuilderReady> {
            state: Some(s),
            handlers: HashMap::new(),

            mode: PhantomData,
        }
    }
}

impl<State> Default for ServiceBuilder<State, BuilderUninitialized>
where
    State: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<State, BuilderMode> ServiceBuilder<State, BuilderMode>
where
    State: Send + Sync + 'static,
{
    pub fn register_state(self, s: Arc<State>) -> ServiceBuilder<State, BuilderReady> {
        ServiceBuilder::<State, BuilderReady> {
            state: Some(s),
            handlers: self.handlers,

            mode: PhantomData,
        }
    }

    pub fn register_handlers(
        self,
        map: &'static HashMap<&'static str, AsyncHandler<State>>,
    ) -> Self {
        let mut builder = self;
        for (key, val) in map.iter() {
            builder.handlers.insert(key, val.clone());
        }

        builder
    }
}

impl<State> ServiceBuilder<State, BuilderReady>
where
    State: Send + Sync + 'static,
{
    pub fn build(mut self) -> Service<State> {
        let state = self.state.take().unwrap();
        let handlers = self.handlers;

        Service { state, handlers }
    }
}

pub fn build_service<State>(
    state: Arc<State>,
    handlers: &'static HashMap<&'static str, AsyncHandler<State>>,
) -> Service<State>
where
    State: Send + Sync + 'static,
{
    Service::builder()
        .register_state(state)
        .register_handlers(handlers)
        .build()
}
