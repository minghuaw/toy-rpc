use async_trait::async_trait;
use cfg_if::cfg_if;
use erased_serde as erased;
use serde::de::Visitor;
use std::io::Cursor; // serde doesn't support AsyncRead

use super::{
    Codec, CodecRead, CodecWrite, DeserializerOwned, EraseDeserializer, Marshal, Unmarshal,
};
use crate::error::Error;
use crate::macros::impl_inner_deserializer;
use crate::message::{MessageId, Metadata};

use super::ConnTypeReadWrite;

cfg_if! {
    if #[cfg(any(
        feature = "async_std_runtime",
        feature = "http_tide"
    ))] {
        mod async_std;
    } else if #[cfg(any(
        feature = "tokio_runtime",
        feature = "http_warp",
        feature = "http_actix_web"
    ))] {
        mod tokio;
    }
}

// #[cfg(any(
//     all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
//     all(feature = "http_tide", not(feature="http_actix_web"), not(feature = "http_warp"))
// ))]
// mod async_std;

// #[cfg(any(
//     all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
//     all(
//         any(feature = "http_warp", feature = "http_actix_web"),
//         not(feature = "http_tide")
//     )
// ))]
// mod tokio;

impl<'de, R> serde::Deserializer<'de> for DeserializerOwned<serde_json::Deserializer<R>>
where
    R: serde_json::de::Read<'de>,
{
    type Error = <&'de mut serde_json::Deserializer<R> as serde::Deserializer<'de>>::Error;

    // the rest is simply calling self.inner.deserialize_xxx()
    // use a macro to generate the code
    impl_inner_deserializer!();
}

impl<R, W, C> Marshal for Codec<R, W, C> {
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(val)
            .map(|mut v| {
                v.push(b'\n');
                v
            })
            .map_err(|e| e.into())
    }
}

impl<R, W, C> Unmarshal for Codec<R, W, C> {
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        serde_json::from_slice(buf).map_err(|e| e.into())
    }
}

impl<R, W, C> EraseDeserializer for Codec<R, W, C> {
    fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send> {
        let de = serde_json::Deserializer::from_reader(Cursor::new(buf));

        let de_owned = DeserializerOwned::new(de);
        Box::new(erased::Deserializer::erase(de_owned))
    }
}

#[cfg(test)]
mod tests {
    use crate::message::RequestHeader;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct FooRequest {
        pub a: u32,
        pub b: u32,
    }

    #[test]
    fn json_request() {
        let header = RequestHeader {
            id: 0,
            service_method: "service.method".to_string(),
        };
        let body = FooRequest { a: 3, b: 6 };

        let header_buf = serde_json::to_string(&header).unwrap();
        let body_buf = serde_json::to_string(&body).unwrap();

        println!("header:\n{}", header_buf);
        println!("body:\n{}", body_buf);
    }
}
