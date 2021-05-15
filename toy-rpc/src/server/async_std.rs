//! This modules implements `Server`'s methods that require `feature = "async_std_runtime"`
//! or `feature = "http_tide"`.

use cfg_if::cfg_if;

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
        ),
    ))] {
        use std::sync::Arc;
        use std::collections::HashMap;
        use ::async_std::net::{TcpListener, TcpStream};
        use ::async_std::task::{self, JoinHandle};
        use futures::{StreamExt, lock::Mutex};
        use flume::{Receiver, Sender};

        use crate::error::Error;
        use crate::transport::ws::WebSocketConn;
        use crate::codec::split::ServerCodecSplit;
        use crate::message::{ExecutionResult};
        use crate::{
            codec::{DefaultCodec, split::ServerCodecWrite},
            message::{ExecutionMessage, MessageId},
        };

        use super::{AsyncServiceMap, Server};

        /// The following impl block is controlled by feature flag. It is enabled
        /// if and only if **exactly one** of the the following feature flag is turned on
        /// - `serde_bincode`
        /// - `serde_json`
        /// - `serde_cbor`
        /// - `serde_rmp`
        impl Server {
            /// Accepts connections on an `async_std::net::TcpListner` and serves requests to default
            /// server for each incoming connection.
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
            /// use async_std::net::TcpListener;
            ///
            /// async fn main() {
            ///     // assume `ExampleService` exist
            ///     let example_service = ExampleService {};
            ///     let server = Server::builder()
            ///         .register(example_service)
            ///         .build();
            ///
            ///     let listener = TcpListener::bind(addr).await.unwrap();
            ///
            ///     let handle = task::spawn(async move {
            ///         server.accept(listener).await.unwrap();
            ///     });
            ///     handle.await;
            /// }
            /// ```
            ///
            /// See `toy-rpc/examples/rap_tcp/` for the example
            pub async fn accept(&self, listener: TcpListener) -> Result<(), Error> {
                let mut incoming = listener.incoming();

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;

                    log::trace!("Accepting incoming connection from {}", stream.peer_addr()?);

                    task::spawn(Self::serve_tcp_connection(stream, self.services.clone()));
                }

                Ok(())
            }

            /// Similar to `accept`. This will accept connections on an `async_std::net::TcpListner` and serves
            /// requests using WebSocket transport protocol and the default codec.
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
            /// use async_std::net::TcpListener;
            ///
            /// async fn main() {
            ///     // assume `ExampleService` exist
            ///     let example_service = ExampleService {};
            ///     let server = Server::builder()
            ///         .register(example_service)
            ///         .build();
            ///
            ///     let listener = TcpListener::bind(addr).await.unwrap();
            ///
            ///     let handle = task::spawn(async move {
            ///         server.accept_websocket(listener).await.unwrap();
            ///     });
            ///     handle.await;
            /// }
            /// ```
            pub async fn accept_websocket(&self, listener: TcpListener) -> Result<(), Error> {
                let mut incoming = listener.incoming();

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

                    task::spawn(Self::accept_ws_connection(stream, self.services.clone()));
                }

                Ok(())
            }

            async fn accept_ws_connection(stream: TcpStream, services: Arc<AsyncServiceMap>) {
                let ws_stream = async_tungstenite::accept_async(stream).await
                        .expect("Error during the websocket handshake occurred");
                    log::trace!("Established WebSocket connection.");

                match Self::serve_ws_connection(ws_stream, services).await {
                    Ok(_) => {},
                    Err(e) => log::error!("{}", e),
                };
            }

            /// Serves a single connection
            async fn serve_tcp_connection(stream: TcpStream, services: Arc<AsyncServiceMap>) -> Result<(), Error> {
                // let _stream = stream;
                let _peer_addr = stream.peer_addr()?;

                // using feature flag controlled default codec
                let codec = DefaultCodec::new(stream);

                // let fut = task::spawn_blocking(|| Self::_serve_codec(codec, services)).await;
                // let ret = Self::serve_codec_loop(codec, services).await;
                let ret = Self::serve_codec_setup(codec, services).await;
                log::trace!("Client disconnected from {}", _peer_addr);
                ret
            }

            /// Serves a single connection using the default codec
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// Example
            ///
            /// ```rust
            /// use async_std::net::TcpStream;
            ///
            /// async fn main() {
            ///     // assume `ExampleService` exist
            ///     let example_service = ExampleService {};
            ///     let server = Server::builder()
            ///         .register(example_service)
            ///         .build();
            ///
            ///     let conn = TcpStream::connect(addr).await.unwrap();
            ///
            ///     let handle = task::spawn(async move {
            ///         server.serve_conn(conn).await.unwrap();
            ///     });
            ///     handle.await;
            /// }
            /// ```
            pub async fn serve_conn(&self, stream: TcpStream) -> Result<(), Error> {
                Self::serve_tcp_connection(stream, self.services.clone()).await
            }

            async fn serve_ws_connection(
                ws_stream: async_tungstenite::WebSocketStream<TcpStream>,
                services: Arc<AsyncServiceMap>,
            ) -> Result<(), Error> {
                let ws_stream = WebSocketConn::new(ws_stream);
                let codec = DefaultCodec::with_websocket(ws_stream);

                let ret = Self::serve_codec_setup(codec, services).await;
                log::info!("Client disconnected from WebSocket connection");
                ret
            }

            // Spawn tasks for the reader/broker/writer loops
            pub(crate) async fn serve_codec_setup(
                codec: impl ServerCodecSplit + 'static,
                services: Arc<AsyncServiceMap>
            ) -> Result<(), Error> {
                let (exec_sender, exec_recver) = flume::unbounded();
                let (resp_sender, resp_recver) = flume::unbounded::<ExecutionResult>();
                let (codec_writer, codec_reader) = codec.split();
                let task_map = Arc::new(Mutex::new(HashMap::new()));

                let reader_handle = task::spawn(
                    super::serve_codec_reader_loop(codec_reader, services, exec_sender, resp_sender.clone())
                );

                let writer_handle = task::spawn(
                    serve_codec_writer_loop(codec_writer, resp_recver, task_map.clone())
                );

                let executor_handle = task::spawn(
                    serve_codec_executor_loop(exec_recver, resp_sender, task_map)
                );

                reader_handle.await?;
                executor_handle.await?;
                writer_handle.await?;
                Ok(())
            }
        }

        async fn serve_codec_executor_loop(
            reader: Receiver<ExecutionMessage>,
            writer: Sender<ExecutionResult>,
            task_map: Arc<Mutex<HashMap<MessageId, JoinHandle<()>>>>
        ) -> Result<(), Error> {
            while let Ok(msg) = reader.recv_async().await {
                match msg {
                    ExecutionMessage::Request{
                        call,
                        id,
                        method,
                        deserializer
                    } => {
                        let fut = call(method, deserializer);
                        let handle = task::spawn(super::serve_codec_execute_call(id, fut, writer.clone()));
                        {
                            let mut map = task_map.lock().await;
                            map.insert(id, handle);
                        }
                    },
                    ExecutionMessage::Cancel(id) => {
                        let mut map = task_map.lock().await;
                        match map.remove(&id) {
                            Some(handle) => {
                                handle.cancel().await;
                            },
                            None => { }
                        }
                    }
                }
            }

            Ok(())
        }

        async fn serve_codec_writer_loop(
            mut codec_writer: impl ServerCodecWrite,
            results: Receiver<ExecutionResult>,
            task_map: Arc<Mutex<HashMap<MessageId, JoinHandle<()>>>>
        ) -> Result<(), Error> {
            while let Ok(msg) = results.recv_async().await {
                {
                    let mut map = task_map.lock().await;
                    map.remove(&msg.id);
                }

                match super::serve_codec_write_once(&mut codec_writer, msg).await {
                    Ok(_) => { },
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            }
            Ok(())
        }
    }
}
