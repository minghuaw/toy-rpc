//! Impplementation of `CodecRead`, `CodecWrite`, `Marshal`, `Unmarshal` and `EraseDeserializer` traits with `bincode`

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "serde_json")] {

    } else if #[cfg(feature = "serde_cbor")] {

    } else if #[cfg(feature = "serde_rmp")] {

    } else {
        use bincode::{DefaultOptions, Options};
        use erased_serde as erased;
        use serde::de::Visitor;
        use std::io::Cursor; // serde doesn't support AsyncRead

        use super::{Codec, DeserializerOwned, Marshal, Unmarshal};
        use crate::error::Error;
        use crate::macros::impl_inner_deserializer;

        use super::EraseDeserializer;

        impl<'de, R, O> serde::Deserializer<'de> for DeserializerOwned<bincode::Deserializer<R, O>>
        where
            R: bincode::BincodeRead<'de>,
            O: bincode::Options,
        {
            type Error = <&'de mut bincode::Deserializer<R, O> as serde::Deserializer<'de>>::Error;

            // use a macro to generate the code
            impl_inner_deserializer!();
        }

        impl<R, W, C> Marshal for Codec<R, W, C> {
            fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
                DefaultOptions::new()
                    // .with_fixint_encoding()
                    .with_varint_encoding() // FIXME: varint has problem with i16
                    .serialize(&val)
                    .map_err(|err| err.into())
            }
        }

        impl<R, W, C> Unmarshal for Codec<R, W, C> {
            fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
                DefaultOptions::new()
                    // .with_fixint_encoding()
                    .with_varint_encoding() // FIXME: varint has problem with i16
                    .deserialize(buf)
                    .map_err(|err| err.into())
            }
        }

        impl<R, W, C> EraseDeserializer for Codec<R, W, C> {
            fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send> {
                let de = bincode::Deserializer::with_reader(
                    Cursor::new(buf),
                    bincode::DefaultOptions::new()
                        // .with_fixint_encoding()
                        .with_varint_encoding() // FIXME: varint has problem with i16
                );

                let de_owned = DeserializerOwned::new(de);
                Box::new(<dyn erased::Deserializer>::erase(de_owned))
            }
        }
    }
}
