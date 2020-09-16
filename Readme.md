
#

## Useful notes

1. Please refer to the way rocket handles endpoint handlers defined in separate mods (https://github.com/SergioBenitez/Rocket/issues/884)
2. The reason why only `&mut serde_json::Deserializer<_>` can be erased is that `impl serde::Deserializer for &'a mut serde_json::Deserializer`.
3. Probably can implement a wrapper type `DeserializerOwned<D>` that holds a `Deserializer`. Might not be able to use generics with trait bounds

##