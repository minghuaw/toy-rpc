//! RPC server. There is only one `Server` defined, but some methods have
//! different implementations depending on the runtime feature flag
//!

use cfg_if::cfg_if;
use std::sync::{atomic::AtomicU64, Arc};

use crate::service::AsyncServiceMap;

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    ))] {
        use flume::Sender;
        mod integration;
        mod broker;
        mod reader;
        mod writer;

        mod pubsub;
        use pubsub::{PubSubBroker, PubSubItem};
    }
}

#[cfg(any(
    feature = "docs",
    doc,
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"),)
))]
mod async_std;

#[cfg(any(
    feature = "docs",
    doc,
    all(
        feature = "tokio_runtime",
        not(feature = "async_std_runtime"),
        not(feature = "http_actix_web")
    )
))]
mod tokio;

pub mod builder;
use builder::ServerBuilder;

pub(crate) type ClientId = u64;
pub(crate) type AtomicClientId = AtomicU64;

/// Client ID 0 is reserved for publisher and subscriber on the server side.
/// Remote client have their ID starting from `RESERVED_CLIENT_ID + 1`
pub const RESERVED_CLIENT_ID: ClientId = 0;

/// RPC Server
///
/// ```
/// const DEFAULT_RPC_PATH: &str = "_rpc_";
/// ```
#[derive(Clone)]
pub struct Server {
    services: Arc<AsyncServiceMap>,
    client_counter: Arc<AtomicClientId>, // monotomically increase counter

    #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    ))]
    pubsub_tx: Sender<PubSubItem>,
}

#[cfg(any(
    feature = "docs",
    all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
))]
impl Drop for Server {
    fn drop(&mut self) {
        if let Err(err) = self.pubsub_tx.send(PubSubItem::Stop) {
            log::error!("{}", err);
        }
    }
}

impl Server {
    /// Creates a `ServerBuilder`
    ///
    /// Example
    ///
    /// ```rust
    /// use toy_rpc::server::{ServerBuilder, Server};
    ///
    /// let builder: ServerBuilder = Server::builder();
    /// ```
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    ))] {
        // use crate::error::Error;

        impl Server {
            /// Builds a Server from a ServerBuilder
            pub fn from_builder(builder: ServerBuilder) -> Self {
                let services = Arc::new(builder.services);
                let (tx, rx) = flume::unbounded();

                let pubsub_broker = PubSubBroker::new(rx);
                pubsub_broker.spawn();

                Self {
                    client_counter: Arc::new(AtomicClientId::new(RESERVED_CLIENT_ID + 1)),
                    services,
                    pubsub_tx: tx
                }
            }
        }

        // Spawn tasks for the reader/broker/writer loops
        #[cfg(any(
            feature = "docs",
            all(
                any(
                    feature = "serde_bincode",
                    feature = "serde_json",
                    feature = "serde_cbor",
                    feature = "serde_rmp",
                ),
                not(feature = "http_actix_web")
            )
        ))]
        pub(crate) async fn start_broker_reader_writer(
            codec: impl crate::codec::split::SplittableCodec + 'static,
            services: Arc<AsyncServiceMap>,
            client_id: ClientId,
            pubsub_tx: Sender<PubSubItem>,
        ) -> Result<(), crate::Error> {
            let (writer, reader) = codec.split();

            let reader = reader::ServerReader::new(reader, services);
            let writer = writer::ServerWriter::new(writer);
            let broker = broker::ServerBroker::new(client_id, pubsub_tx);

            let (broker_handle, _) = brw::spawn(broker, reader, writer);
            let _ = broker_handle.await;
            Ok(())
        }
    }
}
