//! Impplementation of `CodecRead`, `CodecWrite`, `Marshal`, `Unmarshal` and `EraseDeserializer` traits with `serde_json`

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "async-std")] {
        mod async_std;
    } else if #[cfg(feature = "tokio")] {
        mod tokio;
    }
}

cfg_if! {
    if #[cfg(feature = "serde_bincode")] {

    } else if #[cfg(feature = "serde_cbor")] {

    } else if #[cfg(feature = "serde_rmp")] {

    } else {
        use erased_serde as erased;
        use serde::de::Visitor;
        use std::io::Cursor; // serde doesn't support AsyncRead

        use super::{
            Codec, CodecRead, CodecWrite, DeserializerOwned, EraseDeserializer, Marshal, Unmarshal,
        };
        use crate::error::Error;
        use toy_rpc_macros::impl_inner_deserializer;
        use crate::message::{MessageId, Metadata};

        use super::ConnTypeReadWrite;

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
                Box::new(<dyn erased::Deserializer>::erase(de_owned))
            }
        }
    }
}
