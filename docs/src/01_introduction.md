# Introduction

`toy-rpc` aims to be an easy-to-use `async` RPC tool that is inspired by golang's `net/rpc`'s API. 
It supports both `async_std` and `tokio` runtimes and provides integration with `actix_web`, `warp` and `tide`
HTTP server frameworks.

The overall usage and API should feel similar to that of the golang's `net/rpc` package. Some of the names are changed 
to make them sound more "rusty". Because rust does not come with runtime reflection, attribute macros `#[export_impl]`
and `#[export_trait]` (WIP), and attribute `#[export_method]` are used to mark functions "exported" in golang's 
`net/rpc` perspective.

## Why?

While there are grpc implementations like `grpc-rs` and `tonic` as well as schema-free crates like `tarpc`, I didn't find 
a crate that offers the same level of ease-of-use as that of the golang's `net/rpc` package. Other than the ease-of-use,
not many async RPC crates work with both `async_std` and `tokio` runtime and could be difficult to integrate with the common
async HTTP crates (`actix_web`, `warp`, and `tide`). Thus I started working on this crate to bring something that is 
easy-to-use and supports both `async_std` and `tokio` runtimes.

## Feature flags

The feature flags can be put into three categories.

Choice of serialization/deserialzation (only one should be enabled at a time)

- `serde_bincode`: the default codec will use `bincode`
    for serialization/deserialization
- `serde_json`: the default codec will use `serde_json`
    for `json` serialization/deserialization
- `serde_cbor`: the default codec will use `serde_cbor`
    for serialization/deserialization
- `serde_rmp`: the default codec will use `rmp-serde`
    for serialization/deserialization

Choice of runtime and HTTP framework integration

- `async_std_runtime`: supports usage with `async-std`
- `tokio_runtime`: supports usage with `tokio`
- `http_tide`: enables `tide` integration on the server side. This also enables `async_std_runtime`
- `http_actix_web`: enables `actix-web` integration on the server side. This also enables `tokio_runtime`
- `http_warp`: enables integration with `warp` on the server side. This also enables `tokio_runtime`

Choice of RPC server or client (both can be enabled at the same time)

- `server`: enables RPC server
- `client`: enables RPC client 

Other trivial feature flags are listed below, and they are likely of no actual usage for you.
- `docs`
- `std`: `serde/std`. There is no actual usage right now.

By default, only `serde_bincode` feature is enabled. 
You must enable at least one runtime feature flag and the `server` and/or `client` to have something usable.

## Default features

```toml
default = [
    "serde_bincode",
]
```