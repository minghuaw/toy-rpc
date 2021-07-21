//! Client builder

use std::marker::PhantomData;

use cfg_if::cfg_if;

use crate::pubsub::{AckModeAuto, AckModeManual, AckModeNone};

/// Client builder
pub struct ClientBuilder<AckMode> {
    ack_mode: PhantomData<AckMode>
}

impl Default for ClientBuilder<AckModeNone> {
    fn default() -> Self {
        Self {
            ack_mode: PhantomData
        }
    }
}

impl<AckMode> ClientBuilder<AckMode> {
    /// Set the AckMode to None
    pub fn set_ack_mode_none(self) -> ClientBuilder<AckModeNone> {
        ClientBuilder::<AckModeNone> {
            ack_mode: PhantomData
        }
    }

    /// Set the AckMode to Auto
    pub fn set_ack_mode_auto(self) -> ClientBuilder<AckModeAuto> {
        ClientBuilder::<AckModeAuto> {
            ack_mode: PhantomData
        }
    }

    /// Set the AckMode to Manual
    pub fn set_ack_mode_manual(self) -> ClientBuilder<AckModeManual> {
        ClientBuilder::<AckModeManual> {
            ack_mode: PhantomData
        }
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(
            feature = "serde_bincode",
            not(feature = "serde_json"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_cbor",
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_json",
            not(feature = "serde_bincode"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_rmp",
            not(feature = "serde_cbor"),
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
        )
    ))] {
        #[cfg(feature = "tls")]
        use rustls::ClientConfig;

        use crate::{
            DEFAULT_RPC_PATH,
            client::Client,
            error::Error,
        };

        #[cfg(feature = "tls")]
        use crate::client::{tcp_client_with_tls_config, websocket_client_with_tls_config}; 

        #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
        use tokio::net::{ToSocketAddrs, TcpStream};

        #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
        use async_std::net::{TcpStream, ToSocketAddrs};

        impl<AckMode> ClientBuilder<AckMode> {
            /// Connects to an RPC server over socket at the specified network address
            pub async fn dial(self, addr: impl ToSocketAddrs) -> Result<Client<AckMode>, Error> {
                let stream = TcpStream::connect(addr).await?;
                Ok(Client::with_stream(stream))
            }
        
            /// Connects to an RPC server with TLS enabled
            #[cfg(feature = "tls")]
            pub async fn dial_with_tls_config(
                self,
                addr: impl ToSocketAddrs,
                domain: &str,
                config: ClientConfig
            ) -> Result<Client<AckMode>, Error> {
                tcp_client_with_tls_config(addr, domain, config).await
            }
        
            /// Connects to an HTTP RPC server at the specified network address using WebSocket and the defatul codec.
            pub async fn dial_http(self, addr: &str) -> Result<Client<AckMode>, Error> {
                let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
                url.set_scheme("ws").expect("Failed to change scheme to ws");
        
                Client::dial_websocket_url(url).await
            }
        
            /// Connects to an HTTP RPC server with TLS enabled
            #[cfg(feature = "tls")]
            pub async fn dial_http_with_tls_config(
                self,
                addr: &str,
                domain: &str,
                config: ClientConfig,
            ) -> Result<Client<AckMode>, Error> {
                let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
                url.set_scheme("ws").expect("Failed to change scheme to ws");
        
                websocket_client_with_tls_config(url, domain, config).await
            }
        
            /// Similar to `dial`, this connects to an WebSocket RPC server at the specified network address using the defatul codec
            ///
            /// The difference between `dial_websocket` and `dial_http` is that, `dial_websocket` does not
            /// append `DEFAULT_RPC_PATH="_rpc"` to the end of the addr.
            pub async fn dial_websocket(self, addr: &str) -> Result<Client<AckMode>, Error> {
                let url = url::Url::parse(addr)?;
                Client::dial_websocket_url(url).await
            }
        
            /// Similar to `dial_websocket` but with TLS enabled
            #[cfg(feature = "tls")]
            pub async fn dial_websocket_with_tls_config(
                addr: &str,
                domain: &str,
                config: ClientConfig,
            ) -> Result<Client<AckMode>, Error> {
                let url = url::Url::parse(addr)?;
                websocket_client_with_tls_config(url, domain, config).await
            }
        }
    }
}