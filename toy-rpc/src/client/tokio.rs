//! Client implementation with `tokio` runtime

use super::*;

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
        use ::tokio::io::{AsyncRead, AsyncWrite};
        use ::tokio::net::{TcpStream, ToSocketAddrs};
        use async_tungstenite::tokio::connect_async;

        #[cfg(feature = "tls")]
        use rustls::{ClientConfig};

        use crate::transport::ws::WebSocketConn;
        use crate::DEFAULT_RPC_PATH;

        /// The following impl block is controlled by feature flag. It is enabled
        /// if and only if **exactly one** of the the following feature flag is turned on
        /// - `serde_bincode`
        /// - `serde_json`
        /// - `serde_cbor`
        /// - `serde_rmp`
        impl Client {
            /// Connects to an RPC server over socket at the specified network address
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// # Example
            ///
            /// ```rust
            /// let addr = "127.0.0.1:8080";
            /// let client = Client::dial(addr).await.unwrap();
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn dial(addr: impl ToSocketAddrs)
                -> Result<Client, Error>
            {
                let stream = TcpStream::connect(addr).await?;
                Ok(Self::with_stream(stream))
            }

            /// Connects to an RPC server with TLS enabled
            ///
            /// A more detailed example can be found in the 
            /// [GitHub repo](https://github.com/minghuaw/toy-rpc/blob/9793bf53909bd7ffa74967fae6267f973e03ec8a/examples/tokio_tls/src/bin/client.rs#L22)
            #[cfg(feature = "tls")]
            #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls", feature = "tokio_runtime"))))]
            pub async fn dial_with_tls_config(
                addr: impl ToSocketAddrs,
                domain: &str,
                config: ClientConfig
            ) -> Result<Client, Error> {
                super::tcp_client_with_tls_config(addr, domain, config).await
            }

            /// Connects to an HTTP RPC server at the specified network address using WebSocket and the defatul codec.
            ///
            /// It is recommended to use "ws://" as the url scheme as opposed to "http://"; however, internally the url scheme
            /// is changed to "ws://". Internally, `DEFAULT_RPC_PATH="_rpc"` is appended to the end of `addr`,
            /// and the rest is the same is calling `dial_websocket`.
            /// If a network path were to be supplpied, the network path must end with a slash "/".
            /// For example, a valid path could be "ws://127.0.0.1/rpc/".
            ///
            /// *Warning*: WebSocket is used as the underlying transport protocol starting from version "0.5.0-beta.0",
            /// and this will make client of versions later than "0.5.0-beta.0" incompatible with servers of versions
            /// earlier than "0.5.0-beta.0".
            ///
            /// This is enabled
            /// if and only if **only one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// # Example
            ///
            /// ```rust
            /// let addr = "ws://127.0.0.1:8080/rpc/";
            /// let client = Client::dial_http(addr).await.unwrap();
            /// ```
            ///
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn dial_http(addr: &str) -> Result<Client, Error> {
                let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
                url.set_scheme("ws").expect("Failed to change scheme to ws");

                Self::dial_websocket_url(url).await
            }

            /// Connects to an HTTP RPC server with TLS enabled
            ///
            /// An example with self-signed certificate can be found in the 
            /// [GitHub repo](https://github.com/minghuaw/toy-rpc/blob/9793bf53909bd7ffa74967fae6267f973e03ec8a/examples/warp_tls/src/bin/client.rs#L25)
            #[cfg(feature = "tls")]
            #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls", feature = "tokio_runtime"))))]
            pub async fn dial_http_with_tls_config(
                addr: &str,
                domain: &str,
                config: ClientConfig,
            ) -> Result<Client, Error> {
                let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
                url.set_scheme("ws").expect("Failed to change scheme to ws");

                super::websocket_client_with_tls_config(url, domain, config).await
            }

            /// Similar to `dial`, this connects to an WebSocket RPC server at the specified network address using the defatul codec
            ///
            /// The difference between `dial_websocket` and `dial_http` is that, `dial_websocket` does not 
            /// append `DEFAULT_RPC_PATH="_rpc"` to the end of the addr.
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// # Example
            ///
            /// ```rust
            /// let addr = "ws://127.0.0.1:8080";
            /// let client = Client::dial_websocket(addr).await.unwrap();
            /// ```
            ///
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn dial_websocket(addr: &str) -> Result<Client, Error> {
                let url = url::Url::parse(addr)?;
                Self::dial_websocket_url(url).await
            }

            async fn dial_websocket_url(url: url::Url) -> Result<Client, Error> {
                let (ws_stream, _) = connect_async(&url).await?;
                let ws_stream = WebSocketConn::new(ws_stream);
                let codec = DefaultCodec::with_websocket(ws_stream);
                Ok(Self::with_codec(codec))
            }

            /// Similar to `dial_websocket` but with TLS enabled
            #[cfg(feature = "tls")]
            #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls", feature = "tokio_runtime"))))]
            pub async fn dial_websocket_with_tls_config(
                addr: &str,
                domain: &str,
                config: ClientConfig,
            ) -> Result<Client, Error> {
                let url = url::Url::parse(addr)?;
                super::websocket_client_with_tls_config(url, domain, config).await
            }

            /// Creates an RPC `Client` over a stream that implements `tokio::io::AsyncRead`
            /// and `tokio::io::AsyncWrite`
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// # Example
            /// ```
            /// let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            /// let client = Client::with_stream(stream);
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub fn with_stream<T>(stream: T) -> Client
            where
                T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
            {
                let codec = DefaultCodec::new(stream);
                Self::with_codec(codec)
            }
        }
    }
}
