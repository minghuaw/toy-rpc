// use async_std::net::TcpStream;
use futures::{Stream, AsyncRead, AsyncWrite};

use crate::error::Error;
/// Websocket transport
/// 
/// A WsMessage type is probably needed to wrap different implementation of 
/// seems like `warp`, `tide`'s implementations all originated from `tungstenite`.
/// 
/// - `tide-websocket`: `Message` is a re-export of `tungstenite`'s `Message` 
/// - 'warp`: uses `tokio-tungstenite`, 
/// 

pub(crate) struct WebSocketConn<T>(T);

impl<S> Stream for WebSocketConn<async_tungstenite::WebSocketStream<S>> 
where 
    S: AsyncRead + AsyncWrite + Unpin
{
    type Item = Result<Vec<u8>, Error>;

    
}







#[cfg(test)]
mod tests {
    use async_std::net::{TcpListener, TcpStream};
    use async_std::task;

    #[test]
    fn async_tungestenite_handshake() {
        println!("Start testing");
        task::block_on(test_handshake());
        println!("end testing");
    }

    async fn test_handshake() {
        let (tx, rx) = futures::channel::oneshot::channel();

        let f = async move {
            let listener = TcpListener::bind("0.0.0.0:12345").await.unwrap();
            tx.send(()).unwrap();
            while let Ok((connection, _)) = listener.accept().await {
                let stream = async_tungstenite::accept_async(connection).await;
                stream.expect("Failed to handshake with connection");
            }
        };

        task::spawn(f);

        rx.await.expect("Failed to wait for server to be ready");
        let tcp = TcpStream::connect("0.0.0.0:12345")
            .await
            .expect("Failed to connect");
        let url = url::Url::parse("ws://localhost:12345/").unwrap();
        let _stream = async_tungstenite::client_async(url, tcp)
            .await
            .expect("Client failed to connect");
    }
}