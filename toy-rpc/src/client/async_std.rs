//! Client implementation with `async_std` runtime

use ::async_std::task;
use futures::{AsyncRead, AsyncWrite};
use std::sync::atomic::Ordering;

use crate::{codec::split::SplittableClientCodec};

use super::*;

cfg_if! {
    if #[cfg(any(
        any(feature = "docs", doc),
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
        use ::async_std::net::{TcpStream, ToSocketAddrs};
        use async_tungstenite::async_std::connect_async;
        use crate::DEFAULT_RPC_PATH;
        use crate::transport::ws::WebSocketConn;

        /// The following impl block is controlled by feature flag. It is enabled
        /// if and only if **exactly one** of the the following feature flag is turned on
        /// - `serde_bincode`
        /// - `serde_json`
        /// - `serde_cbor`
        /// - `serde_rmp`
        impl Client<NotConnected> {
            /// Connects the an RPC server over socket at the specified network address
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
            pub async fn dial(addr: impl ToSocketAddrs)
                -> Result<Client<Connected>, Error>
            {
                let stream = TcpStream::connect(addr).await?;
                Ok(Self::with_stream(stream))
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
            pub async fn dial_http(addr: &str) -> Result<Client<Connected>, Error> {
                let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
                url.set_scheme("ws").expect("Failed to change scheme to ws");

                Self::dial_websocket_url(url).await
            }

            /// Similar to `dial`, this connects to an WebSocket RPC server at the specified network address using the defatul codec
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
            /// let client = Client::dial_http(addr).await.unwrap();
            /// ```
            ///
            pub async fn dial_websocket(addr: &str) -> Result<Client<Connected>, Error> {
                let url = url::Url::parse(addr)?;
                Self::dial_websocket_url(url).await
            }

            async fn dial_websocket_url(url: url::Url) -> Result<Client<Connected>, Error> {
                let (ws_stream, _) = connect_async(&url).await?;
                let ws_stream = WebSocketConn::new(ws_stream);
                let codec = DefaultCodec::with_websocket(ws_stream);
                Ok(Self::with_codec(codec))
            }

            /// Creates an RPC `Client` over socket with a specified `async_std::net::TcpStream` and the default codec
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
            pub fn with_stream<T>(stream: T) -> Client<Connected>
            where
                T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
            {
                let codec = DefaultCodec::new(stream);
                Self::with_codec(codec)
            }
        }
    }
}

impl Client<NotConnected> {
    /// Creates an RPC 'Client` over socket with a specified codec
    ///
    /// Example
    ///
    /// ```rust
    /// let addr = "127.0.0.1:8080";
    /// let stream = TcpStream::connect(addr).await.unwrap();
    /// let codec = Codec::new(stream);
    /// let client = Client::with_codec(codec);
    /// ```
    pub fn with_codec<C>(codec: C) -> Client<Connected>
    where
        C: SplittableClientCodec + Send + Sync + 'static,
    {
        let (writer, reader) = codec.split();
        let (req_sender, req_recver) = flume::unbounded();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (reader_stop, stop) = flume::bounded(1);
        task::spawn(reader_loop(reader, pending.clone(), stop));

        let (writer_stop, stop) = flume::bounded(1);
        task::spawn(writer_loop(writer, req_recver, stop));

        Client::<Connected> {
            count: AtomicMessageId::new(0),
            pending,
            requests: req_sender,
            reader_stop,
            writer_stop,

            marker: PhantomData,
        }
    }
}

impl Client<Connected> {
    /// Invokes the named function and wait synchronously in a blocking manner.
    ///
    /// This function internally calls `task::block_on` to wait for the response.
    /// Do NOT use this function inside another `task::block_on`.async_std
    ///
    /// Example
    ///
    /// ```rust
    /// let args = "arguments";
    /// let reply: Result<String, Error> = client.call("EchoService.echo", &args);
    /// println!("{:?}", reply);
    /// ```
    pub fn call_blocking<Req, Res>(
        &self,
        service_method: impl ToString,
        args: Req,
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static,
    {
        let call = self.call(service_method, args);
        task::block_on(call)
    }

    /// Invokes the named RPC function call asynchronously and returns a cancellation `Call`
    /// 
    /// The `Call<Res>` type takes one type argument `Res` which is the type of successful RPC execution.
    /// The result can be obtained by `.await`ing on the `Call`, which returns type `Result<Res, toy_rpc::Error>`
    /// `Call` can be cancelled by calling the `cancel()` function. 
    /// The request will be sent in a background task. 
    /// 
    /// Example 
    /// 
    /// ```rust
    /// // Get the result by `.await`ing on the `Call`
    /// let call: Call<i32> = client.call("SomeService.echo_i32", 7i32);
    /// let reply: Result<i32, toy_rpc::Error> = call.await;
    /// 
    /// // Cancel the call
    /// let call: Call<()> = client.call("SomeService.infinite_loop", ());
    /// call.cancel();
    /// ```
    pub fn call<Req, Res>(&self, service_method: impl ToString, args: Req) -> Call<Res>
    where
        Req: serde::Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static,
    {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        let service_method = service_method.to_string();
        let header = RequestHeader { id, service_method };
        let body = Box::new(args) as ClientRequestBody;

        // create oneshot channel
        let (done_tx, done_rx) = oneshot::channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let pending = self.pending.clone();
        let request_tx = self.requests.clone();
        let handle = task::spawn(handle_call(
            pending, header, body, request_tx, cancel_rx, done_tx,
        ));

        // create Call
        Call::<Res> {
            id,
            cancel: cancel_tx,
            done: done_rx,
            handle: Box::new(handle),
        }
    }
}
