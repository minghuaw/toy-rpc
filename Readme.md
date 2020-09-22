
#

## Useful notes

1. Please refer to the way rocket handles endpoint handlers defined in separate mods (https://github.com/SergioBenitez/Rocket/issues/884)
2. The reason why only `&mut serde_json::Deserializer<_>` can be erased is that `impl serde::Deserializer for &'a mut serde_json::Deserializer`.
3. Probably can implement a wrapper type `DeserializerOwned<D>` that holds a `Deserializer`. Might not be able to use generics with trait bounds

## What errors does golang's `net/rpc` have

Errors in server.go 
- "connection is shut down"
- "rpc.Register: no service name for type " + s.typ.String()
- "rpc.Register: type " + sname + " is not exported"
- "rpc.Register: type " + sname + " has no exported methods of suitable type (hint: pass a pointer to value of that type)"
- "rpc.Register: type " + sname + " has no exported methods of suitable type"
- "rpc: service already defined: " + sname
- "rpc: server cannot decode request: " + err.Error()
- "rpc: service/method request ill-formed: " + req.ServiceMethod
- "rpc: can't find service " + req.ServiceMethod
- "rpc: can't find method " + req.ServiceMethod
- "rpc.Serve: accept:", err.Error()