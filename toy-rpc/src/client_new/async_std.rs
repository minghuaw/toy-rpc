use std::sync::atomic::Ordering;

use ::async_std::task;
use futures::{AsyncRead, AsyncWrite};

use super::*;

impl Client<NotConnected> {
    pub fn with_codec<C>(codec: C) -> Client<Connected> 
    where 
        C: ClientCodec + Send + Sync + 'static
    {
        let codec: Box<dyn ClientCodec> = Box::new(codec);

        Client::<Connected> {
            count: AtomicMessageId::new(0),
            inner_codec: Arc::new(Mutex::new(codec)),
            pending: Arc::new(Mutex::new(HashMap::new())),

            marker: PhantomData
        }
    }
}

impl Client<Connected> {
    pub fn call_blocking() {
        unimplemented!()
    }

    pub async fn call<Req, Res>(
        &self,
        service_method: impl ToString,
        args: Req
    ) -> Result<Call<Res>, Error>
    where 
        Req: serde::Serialize + Send + Sync,
        Res: serde::de::DeserializeOwned,
    {
        unimplemented!()
    }
}