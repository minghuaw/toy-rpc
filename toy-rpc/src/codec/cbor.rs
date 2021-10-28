//! Impplementation of `CodecRead`, `CodecWrite`, `Marshal`, `Unmarshal` and `EraseDeserializer` traits with `serde_cbor`

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "serde_json")] {
        
    } else if #[cfg(feature = "serde_bincode")] {
        
    } else if #[cfg(feature = "serde_rmp")] {
        
    } else {
        use erased_serde as erased;
        use serde::de::Visitor;
        use std::io::Cursor; // serde doesn't support AsyncRead
        
        use super::{Codec, DeserializerOwned, EraseDeserializer, Marshal, Unmarshal};
        use crate::error::ParseError;
        use crate::macros::impl_inner_deserializer;

        impl<'de, R> serde::Deserializer<'de> for DeserializerOwned<serde_cbor::Deserializer<R>>
        where
            R: serde_cbor::de::Read<'de>,
        {
            type Error = <&'de mut serde_cbor::Deserializer<R> as serde::Deserializer<'de>>::Error;

            // use a macro to generate the code
            impl_inner_deserializer!();
        }

        impl<R, W, C> Marshal for Codec<R, W, C> {
            fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, ParseError> {
                serde_cbor::to_vec(val).map_err(|e| e.into())
            }
        }

        impl<R, W, C> Unmarshal for Codec<R, W, C> {
            fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, ParseError> {
                serde_cbor::from_slice(buf).map_err(|e| e.into())
            }
        }

        impl<R, W, C> EraseDeserializer for Codec<R, W, C> {
            fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send> {
                let de = serde_cbor::Deserializer::from_reader(Cursor::new(buf));

                let de_owned = DeserializerOwned::new(de);
                Box::new(<dyn erased::Deserializer>::erase(de_owned))
            }
        }
    }
}
