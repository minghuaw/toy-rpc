use futures::channel::mpsc::{Sender, Receiver};


pub struct BytesSender {
    inner: Sender<Vec<u8>>,
}

pub struct BytesReceiver {
    inner: Receiver<Vec<u8>>,
}