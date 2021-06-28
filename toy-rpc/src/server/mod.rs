//! RPC server. There is only one `Server` defined, but some methods have
//! different implementations depending on the runtime feature flag
//!

use cfg_if::cfg_if;
use std::sync::Arc;

use crate::{service::AsyncServiceMap};


cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    ))] {
        mod integration;
        mod broker;
        mod reader;
        mod writer;
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
    all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
))]
mod tokio;

pub mod builder;
use builder::ServerBuilder;

/// RPC Server
///
/// ```
/// const DEFAULT_RPC_PATH: &str = "_rpc_";
/// ```
#[derive(Clone)]
pub struct Server {
    services: Arc<AsyncServiceMap>,
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
        use crate::error::Error;

        // Spawn tasks for the reader/broker/writer loops
        #[cfg(any(
            feature = "serde_bincode", 
            feature = "serde_json",
            feature = "serde_cbor",
            feature = "serde_rmp",
        ))]
        pub(crate) async fn start_broker_reader_writer(
            codec: impl crate::codec::split::SplittableCodec + 'static,
            services: Arc<AsyncServiceMap>
        ) -> Result<(), Error> {
            let (writer, reader) = codec.split();

            let reader = reader::ServerReader::new(reader, services);
            let writer = writer::ServerWriter::new(writer);
            let broker = broker::ServerBroker::new();

            let (mut broker_handle, _) = brw::spawn(broker, reader, writer);
            brw::util::Conclude::conclude(&mut broker_handle);
            Ok(())
        }
    }
}


