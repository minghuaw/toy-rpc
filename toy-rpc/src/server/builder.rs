//! Builder of the Server

use erased_serde as erased;
use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Duration};

#[cfg(any(
    feature = "docs",
    all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
))]
use super::Server;

use crate::{pubsub::{AckModeAuto, AckModeNone, DEFAULT_PUB_RETRIES, DEFAULT_PUB_RETRY_TIMEOUT}, service::{build_service, AsyncServiceMap, HandleService, HandlerResultFut, Service}, util::RegisterService};

/// Server builder
pub struct ServerBuilder<AckMode> {
    /// Registered services
    pub services: AsyncServiceMap,
    /// Timeout for receiving the Ack from subscriber
    pub pub_retry_timeout: Duration,
    /// Max number of retries for publishing
    pub max_num_retries: u32,
    ack_mode: PhantomData<AckMode>,
}

impl<AckMode> ServerBuilder<AckMode> {
    /// Creates a new `ServerBuilder`
    pub fn new() -> Self {
        ServerBuilder {
            services: HashMap::new(),
            pub_retry_timeout: DEFAULT_PUB_RETRY_TIMEOUT,
            max_num_retries: DEFAULT_PUB_RETRIES,
            ack_mode: PhantomData,
        }
    }

    /// Sets the AckMode to None
    pub fn set_ack_mode_none(self) -> ServerBuilder<AckModeNone> {
        ServerBuilder::<AckModeNone> {
            services: self.services,
            pub_retry_timeout: self.pub_retry_timeout,
            max_num_retries: self.max_num_retries,
            ack_mode: PhantomData,
        }
    }

    /// Sets the AckMode to Auto
    pub fn set_ack_mode_auto(self) -> ServerBuilder<AckModeAuto> {
        ServerBuilder::<AckModeAuto> {
            services: self.services,
            pub_retry_timeout: self.pub_retry_timeout,
            max_num_retries: self.max_num_retries,
            ack_mode: PhantomData
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
}

impl Default for ServerBuilder<AckModeNone> {
    fn default() -> ServerBuilder<AckModeNone> {
        ServerBuilder::<AckModeNone>::new()
    }
}

macro_rules! impl_server_builder_for_ack_modes {
    ($($ack_mode:ty),*) => {
        $(
            #[cfg(any(
                feature = "docs",
                all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
                all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
            ))]
            impl ServerBuilder<$ack_mode> {
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
                pub fn build(self) -> Server<$ack_mode> {
                    use super::{AtomicClientId, RESERVED_CLIENT_ID, PubSubBroker};
            
                    let services = Arc::new(self.services);
            
                    let (pubsub_broker, pubsub_tx) = PubSubBroker::<$ack_mode>::new(self.pub_retry_timeout, self.max_num_retries);
                    pubsub_broker.spawn();
            
                    Server::<$ack_mode> {
                        client_counter: Arc::new(AtomicClientId::new(RESERVED_CLIENT_ID + 1)),
                        services,
                        pubsub_tx,
                        ack_mode: PhantomData,
                    }
                }
            }
        )*
    };
}

impl_server_builder_for_ack_modes!(AckModeNone, AckModeAuto);