//! Builder of the Server

use erased_serde as erased;
use std::{collections::HashMap, sync::Arc};

use super::Server;
use toy_rpc_core::{
    service::{build_service, AsyncServiceMap, HandleService, HandlerResultFut, Service},
    util::RegisterService,
};

/// Server builder
pub struct ServerBuilder {
    services: AsyncServiceMap,
}

impl ServerBuilder {
    /// Creates a new `ServerBuilder`
    pub fn new() -> Self {
        ServerBuilder {
            services: HashMap::new(),
        }
    }

    /// Registers a new service to the `Server` with the default name.
    ///
    /// Internally the `Service` object will be built using the supplied `service`
    /// , which is the state of the `Service` object
    ///
    /// # Example
    ///
    /// ```rust
    /// let foo = Arc::new(Foo { });
    /// // construct server
    /// let server = Server::builder()
    ///     .register(foo) // this will register `foo` with the default service name `Foo`
    ///     .build();
    /// ```
    pub fn register<S>(self, service: Arc<S>) -> Self
    where
        S: RegisterService + Send + Sync + 'static,
    {
        self.register_with_name(S::default_name(), service)
    }

    /// Register a a service with a name. This allows registering multiple instances
    /// of the same type on the server.
    ///
    /// # Example
    ///
    /// ```rust
    /// let foo1 = Arc::new(Foo { });
    /// let foo2 = Arc::new(Foo { });
    /// // construct server
    /// let server = Server::builder()
    ///     .register(foo1) // this will register `foo1` with the default service name `Foo`
    ///     .register_with_name("Foo2", foo2) // this will register `foo2` with the service name `Foo2`
    ///     .build();
    /// ```
    pub fn register_with_name<S>(self, name: &'static str, service: Arc<S>) -> Self
    where
        S: RegisterService + Send + Sync + 'static,
    {
        let service = build_service(service, S::handlers());
        self.register_service(name, service)
    }

    /// Register a `Service` instance. This allows registering multiple instances
    /// of the same type on the server.
    /// 
    /// # Example
    ///
    /// ```rust
    /// let foo1 = Arc::new(Foo { });
    /// let foo2 = Arc::new(Foo { });
    ///
    /// // construct server
    /// let server = Server::builder()
    ///     .register(foo1) // this will register `foo1` with the default service name `Foo`
    ///     .register_service("Foo2", foo2) // this will register `foo2` with the service name `Foo2`
    ///     .build();
    /// ```
    fn register_service<S>(self, name: &'static str, service: Service<S>) -> Self
    where
        S: Send + Sync + 'static,
    {
        let call = move |method_name: String,
                         _deserializer: Box<(dyn erased::Deserializer<'static> + Send)>|
              -> HandlerResultFut { service.call(&method_name, _deserializer) };

        log::debug!("Registering service: {}", name);
        let mut builder = self;
        builder.services.insert(name, Arc::new(call));
        builder
    }

    /// Builds an RPC `Server`
    /// 
    /// # Example
    ///
    /// ```
    /// let echo_service = Arc::new(EchoService { });
    /// let builder: ServerBuilder = Server::builder()
    ///     .register(echo_service);
    /// let server: Server = builder.build();        
    /// ```
    pub fn build(self) -> Server {
        Server {
            services: Arc::new(self.services),
        }
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
