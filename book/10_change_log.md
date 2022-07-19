# Change Log

## 0.8.6

- Updated serde_rmp, tungstenite, and async-tungstenite to the latest version

## 0.8.5

- Moved accepting incoming websocket connection to main task to allow propagating error back to the user

## 0.8.4

- Unified all connection related error (read/write) to `Error::IoError(_)`

## 0.8.3

- Reverting back to 2018 edition

## 0.8.2

- Fixed a bug where attribute macros doesn't parse trait path and type path correctly

## 0.8.1

- Updated dependencies and corresponding examples
  - `axum` to the latest version
  - WebSocket dependencies like `tungstenite`, etc
  - TLS dependencies like `rustls`, etc.
- Switched from `async-rustls` to `futures-rustls`
- The error message of sending on a closed channel upon ending a client or server 
becomes a debug message now.

## 0.8.0

- Finalized features

## 0.8.0-beta

- Added AckMode impl
- Added attribute #[topic()] for derive macro #[derive(Topic)]
- Added integration with `axum`

## 0.8.0-alpha

### Breaking Changes

- Communication protocol is changed

### Non-breaking Changes

- Added pubsub support
- Added RPC trait implementation generation for client

## 0.7.4

- Fixed wrong documentation for `Client::with_stream<T>(stream: T)`

## ~~0.7.3 (yanked)~~

## 0.7.2

- Relaxed trait bounds on generic type `T` in `Client::with_stream<T>(stream: T)`

## 0.7.1

### Bug fix

- Fixed a bug where large payload is not written entirely with `tokio` runtime

## 0.7.0

### Breaking Changes

- The blocking RPC call on the client side is renamed to `call_blocking`
- The asynchronous RPC call on the client side is renamed to `call`
- The `call` method returns a `Call<Res>` type where `Res` represents the `Ok` type of 
result. The request is sent by a background task and thus the new `call` method is 
similar to the old `spawn_task` in terms of usage.

### New Features

- Cancellation. The `Call<Res>` type returned by the `call` method can be canceled by 
using the `cancel()` method.
- Timeout. A timeout can be set for the next request by calling `client.timeout(duration)`. 
only one request after setting the timeout is going to run with a timeout. If you want to set timeout
for multiple requests, you need to set the timeout for each of them.

## 0.6.1

- Multiple objects of the same types can be registered on the same server again, but you will need to
use the `ServerBuilder::register_with_name` method as opposed to the regular `ServerBuilder::register`.
More details can be found in `ServerBuilder::register_with_name`'s documentation.

## 0.6.0

### Breaking Changes

- In short, this update makes the crate resemble closer to the usage of `go`'s `net/rpc` package
- Service registration is simplified to `Server::builder().register(foo_service).build()`. The examples will be
updated accordingly. Thus
    - `service!()` macro will be deprecated
    - `register` function now takes only one argument, which is the instance of the service
    - on the client side, the service name will just be the name of the struct. for example,
        to call a RPC method on `struct Foo { }` service, the client simply uses
        `.async_call("Foo.<method>").await` where `<method>` should be replaced with the RPC method
    - you can still register multiple services on the same server. However, only one object of the same type
        can be registered on the same server. Multiple servers are needed to have multiple objects of the same type.
- Re-defined the custom `Error` type

### Non-breaking changes

- Fixed bug where client does not interpret error message correctly
- Fixed bug with `accept_websocket` crashes with incorrect protocol

## 0.5.4

- Handlers are now stored as a `fn` pointer as opposed to a trait object.

## 0.5.3

- The `#[export_impl]` macro now generates client stub functions by generating a new trait for `toy_rpc::Client`.

## 0.5.0

**Breaking changes**

- HTTP integration is now accomplished using WebSocket with `async_tungstenite`, and thus HTTP connections
of versions <0.5.0 are not compatible with versions >=0.5.0.
- The custom binary transport protocol now includes a magic byte at the beginning, making
versions <0.5.0 **NOT** compatible with versions >= 0.5.0;
- `toy_rpc::error::Error` changed from struct-like variants to simple enum variants
- Changes to feature flags
    - "logging" feature flag is removed
    - "surf" feature flag is removed
    - "tide" is changed to "http_tide"
    - "actix-web" is changed to "http_actix_web"
    - added "http_warp" feature flag
    - added "async_std_runtime"
    - added "tokio_runtime"

Non-breaking changes

- Removed `Stream` and `Sink` impl from the custom binary transport protocol `Frame`

## 0.4.5

- Added `Sink` implementation for the custom binary transport protocol `Frame`

## 0.4.4

- Modified traits `CodecRead`, `CodecWrite`, `ServerCodec`, `ClientCodec` to no longer
return number of bytes written
- The number of bytes written for header and body will be logged separately

## 0.4.3

- Removed previously unused NoneError
- Unified `call`, `async_call` and `spawn_task` for socket client
and HTTP client. The `call_http`, `async_call_http`, and `spawn_task_http`
methods are kept for compatibility.

## 0.4.2

- Temporary fix of `spawn_task()` and `spawn_task_http()` with `Arc<Mutex<_>>` until
lifetime with async task is figured out. As a result, `Client` no longer needs to be declared `mut`.

## 0.4.1

- Updated documentation

## 0.4.0

- Added `actix-web` feature flag to support integration with `actix-web`

## 0.3.1

- Added `serde_rmp` features flag
- Updated and corrected examples in the documentation

## 0.3.0

- Added `serde_cbor` feature flag
- Changed `bincode` feature flag to `serde_bincode`