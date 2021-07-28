# Introduction

<div align="center">
<!-- Crates version -->
<a href="https://crates.io/crates/toy-rpc">
<img src="https://img.shields.io/crates/v/toy-rpc.svg?style=flat" alt="Crates.io version" />
</a>
<!-- docs.rs docs -->
<a href="https://docs.rs/toy-rpc">
<img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat" alt="docs.rs docs" />
</a>
<!-- Downloads -->
<a href="https://crates.io/crates/toy-rpc">
<img src="https://img.shields.io/crates/d/toy-rpc.svg?style=flat" alt="Download" />
</a>
<a href="https://github.com/rust-secure-code/safety-dance/">
<img src="https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat" alt="Unsafe Rust forbidden" />
</a>
</div>

`toy-rpc` aims to be an easy-to-use `async` RPC tool that is inspired by golang's `net/rpc`'s API. 
It supports both `async_std` and `tokio` runtimes over either TCP or TLS. Integration with common HTTP server frameworks such as `actix_web`, `warp` and `tide`
are provided.

The overall usage and API should feel similar to that of the golang's `net/rpc` package. Some of the names are changed 
to make them sound more "rusty". Because rust does not come with runtime reflection, attribute macros `#[export_impl]`
and `#[export_trait]` / `#[export_trait_impl]`, and attribute `#[export_method]` are used to mark functions "exported" in golang's 
`net/rpc` perspective.

Minimum supported Rust version: 1.53 or later

## Why?

While there are grpc implementations like `grpc-rs` and `tonic` as well as schema-free crates like `tarpc`, I didn't find 
a crate that offers the same level of ease-of-use as that of the golang's `net/rpc` package. Other than the ease-of-use,
not many async RPC crates work with both `async_std` and `tokio` runtime and could be difficult to integrate with the common
async HTTP crates (`actix_web`, `warp`, and `tide`). Thus I started working on this crate to bring something that is 
easy-to-use and supports both `async_std` and `tokio` runtimes.

## Feature flags

Most of the feature flags can be put into three categories.

Choice of runtime and HTTP framework integration

- `async_std_runtime`: supports usage with `async-std`
- `tokio_runtime`: supports usage with `tokio`
- `http_tide`: enables `tide` integration on the server side. This also enables `async_std_runtime`
- `http_actix_web`: enables `actix-web` integration on the server side. This also enables `tokio_runtime`
- `http_warp`: enables integration with `warp` on the server side. This also enables `tokio_runtime`

Choice of RPC server or client (both can be enabled at the same time)

- `server`: enables RPC server
- `client`: enables RPC client 

Choice of serialization/deserialzation (only one should be enabled at a time)

- `serde_bincode`: (default) the default codec will use `bincode`
    for serialization/deserialization
- `serde_json`: the default codec will use `serde_json`
    for `json` serialization/deserialization
- `serde_cbor`: the default codec will use `serde_cbor`
    for serialization/deserialization
- `serde_rmp`: the default codec will use `rmp-serde`
    for serialization/deserialization

TLS support

- `tls`: enables TLS support

Convenience conversion to `anyhow::Error`

- `anyhow`: enables using `anyhow::Error` in RPC methods

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