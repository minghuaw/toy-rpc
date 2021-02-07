//! Impplementation of `CodecRead`, `CodecWrite`, `Marshal`, `Unmarshal` and `EraseDeserializer` traits with `rmp-serde`

use cfg_if::cfg_if;

cfg_if!{
    if #[cfg(feature = "serde_json")] {

    } else if #[cfg(feature = "serde_cbor")] {

    } else if #[cfg(feature = "serde_bincode")] {

    } else { 
        use erased_serde as erased;
        use serde::de::Visitor;
        use std::io::Cursor; // serde doesn't support AsyncRead
        
        use super::{Codec, DeserializerOwned, EraseDeserializer, Marshal, Unmarshal};
        use crate::error::Error;
        use crate::macros::impl_inner_deserializer;
        
        impl<'de, R> serde::Deserializer<'de>
            for DeserializerOwned<rmp_serde::Deserializer<rmp_serde::decode::ReadReader<R>>>
        where
            R: std::io::Read,
        {
            type Error = <&'de mut rmp_serde::Deserializer<rmp_serde::decode::ReadReader<R>> as serde::Deserializer<'de>>::Error;
        
            // use a macro to generate the code
            impl_inner_deserializer!();
        }
        
        impl<R, W, C> Marshal for Codec<R, W, C> {
            fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
                let mut buf = Vec::new();
                match val.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
                    Ok(_) => Ok(buf),
                    Err(e) => Err(e.into()),
                }
            }
        }
        
        impl<R, W, C> Unmarshal for Codec<R, W, C> {
            fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
                let mut de = rmp_serde::Deserializer::new(buf);
                serde::Deserialize::deserialize(&mut de).map_err(|e| e.into())
            }
        }
        
        impl<R, W, C> EraseDeserializer for Codec<R, W, C> {
            fn from_bytes(buf: Vec<u8>) -> Box<dyn erased::Deserializer<'static> + Send> {
                let de = rmp_serde::Deserializer::new(Cursor::new(buf));
                let de_owned = DeserializerOwned::new(de);
                Box::new(erased::Deserializer::erase(de_owned))
            }
        }
    }
}

