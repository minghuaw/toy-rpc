//!  Service builder and registration

use async_trait::async_trait;
use futures::future::Future;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::Error;
use crate::protocol::{InboundBody, OutboundBody};

/// Ok type of HandlerResult
// pub(crate) type Success = Box<dyn erased::Serialize + Send + Sync + 'static>;
// pub(crate) type Success = Box<OutboundBody>;


pub type Outcome = Box<OutboundBody>;


/// Future of RPC handler
pub type HandlerResultFut = Pin<Box<dyn Future<Output = Result<Outcome, Error>> + Send>>;

/// Async handler definition
pub type AsyncHandler<S> = fn(Arc<S>, Box<InboundBody>) -> HandlerResultFut;

/// Result of HandleService::call
pub type ServiceCallResult = Result<Outcome, Error>;

/// Future of RPC handler, this must be `.await`ed to obtain the result
pub type ServiceCallFut = Pin<Box<dyn Future<Output = ServiceCallResult> + Send>>;

/// Async trait objects to invoke a service
pub type AsyncServiceCall =
    dyn Fn(String, Box<InboundBody>) -> ServiceCallFut + Send + Sync + 'static;

/// Arc wrapper of `AsyncServiceCall`
pub type ArcAsyncServiceCall = Arc<AsyncServiceCall>;

/// Hashmap of services.
///
/// The keys are service names and the values are function trait objects `ArcAsyncServiceCall`
pub type AsyncServiceMap = HashMap<&'static str, ArcAsyncServiceCall>;

/// A RPC service that can hold an internal state
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
    /// Creates a `ServiceBuilder`
    pub fn builder() -> ServiceBuilder<State, BuilderUninitialized> {
        ServiceBuilder::new()
    }
}

/// The `HandleService` trait provides the method `call` which will execute the
/// RPC method
#[async_trait]
pub trait HandleService<State>
where
    State: Send + Sync + 'static,
{
    /// Returns a `Arc` of the internal state
    fn state(&self) -> Arc<State>;

    /// Returns a function pointer to the requested method
    fn method(&self, name: &str) -> Option<AsyncHandler<State>>;

    /// Returns a future that will execute the RPC method when `.await`ed.
    /// Returns `Error::MethodNotFound` if the requested method is not registered.
    fn call(&self, name: &str, deserializer: Box<InboundBody>) -> ServiceCallFut {
        let state = self.state();
        match self.method(name) {
            Some(m) => {
                m(state, deserializer)
            },
            None => Box::pin(async move { Err(Error::MethodNotFound) }),
        }
    }
}

impl<State> HandleService<State> for Service<State>
where
    State: Send + Sync + 'static,
{
    fn state(&self) -> Arc<State> {
        self.state.clone()
    }

    fn method(&self, name: &str) -> Option<AsyncHandler<State>> {
        self.handlers.get(name).cloned()
    }
}

/// Type state for the `ServiceBuilder` when the builder is NOT ready to build a `Service`
pub struct BuilderUninitialized;
/// Type state for the `ServiceBuilder` when the builder is ready to build a `Service`
pub struct BuilderReady;

/// Service builder. The builder uses type state to control whether the builder is ready
/// to build a `Service`.
///
/// A `Service` can be built without any handler but cannot be built without internal state.
pub struct ServiceBuilder<State, BuilderMode>
where
    State: Send + Sync + 'static,
{
    /// Internal state of the builder, which will be the internal state of the `Service`
    pub state: Option<Arc<State>>,

    /// RPC method handlers
    pub handlers: HashMap<&'static str, AsyncHandler<State>>,

    // helper members for TypeState only
    mode: PhantomData<BuilderMode>,
}

impl<State> ServiceBuilder<State, BuilderUninitialized>
where
    State: Send + Sync + 'static,
{
    /// Creates a new builder without any internal state.
    pub fn new() -> ServiceBuilder<State, BuilderUninitialized> {
        ServiceBuilder::<State, BuilderUninitialized> {
            state: None,
            handlers: HashMap::new(),

            mode: PhantomData,
        }
    }

    /// Creates a new builder with an internal state.
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
    /// Register the internal state
    pub fn register_state(self, s: Arc<State>) -> ServiceBuilder<State, BuilderReady> {
        ServiceBuilder::<State, BuilderReady> {
            state: Some(s),
            handlers: self.handlers,

            mode: PhantomData,
        }
    }

    /// Register a hashmap of RPC handlers
    pub fn register_handlers(self, map: HashMap<&'static str, AsyncHandler<State>>) -> Self {
        let mut builder = self;
        // for (key, val) in map.iter() {
        //     builder = builder.register_handler(key, *val);
        // }
        builder.handlers = map;

        builder
    }

    /// Register a handler for a service
    pub fn register_handler(self, method: &'static str, handler: AsyncHandler<State>) -> Self {
        let mut builder = self;
        builder.handlers.insert(method, handler);
        builder
    }
}

impl<State> ServiceBuilder<State, BuilderReady>
where
    State: Send + Sync + 'static,
{
    /// Build a `Service`
    pub fn build(mut self) -> Service<State> {
        let state = self.state.take().unwrap();
        let handlers = self.handlers;

        Service { state, handlers }
    }
}

/// Convenience function to build a `Service` with the internal state and the handlers
pub fn build_service<State>(
    state: Arc<State>,
    handlers: HashMap<&'static str, AsyncHandler<State>>,
) -> Service<State>
where
    State: Send + Sync + 'static,
{
    Service::builder()
        .register_state(state)
        .register_handlers(handlers)
        .build()
}
