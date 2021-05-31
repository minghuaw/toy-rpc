#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! # An async RPC crate that mimics the `golang`'s `net/rpc` package and supports both `async-std` and `tokio`.
//!
//! <div align="center">
//! <!-- Crates version -->
//! <a href="https://crates.io/crates/toy-rpc">
//! <img src="https://img.shields.io/crates/v/toy-rpc.svg?style=flat"
//! alt="Crates.io version" />
//! </a>
//! <!-- docs.rs docs -->
//! <a href="https://docs.rs/toy-rpc">
//! <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat"
//! alt="docs.rs docs" />
//! </a>
//! <!-- Downloads -->
//! <a href="https://crates.io/crates/toy-rpc">
//! <img src="https://img.shields.io/crates/d/toy-rpc.svg?style=flat"
//! alt="Download" />
//! </a>
//! <a href="https://github.com/rust-secure-code/safety-dance/">
//! <img src="https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat"
//! alt="Unsafe Rust forbidden" />
//! </a>
//! </div>
//!
//! `toy-rpc` aims to be an easy-to-use async RPC tool that is inspired by golang's `net/rpc`'s API. 
//! It supports both `async_std` and `tokio` runtimes and provides integration with `actix_web`, 
//! `warp` and `tide` HTTP server frameworks.
//! 
//! The overall usage and API should feel similar to that of the golang's `net/rpc` package. Some of the names are changed 
//! to make them sound more "rusty". Because rust does not come with runtime reflection, attribute macros `#[export_impl]`
//! and `#[export_trait]` / `#[export_trait_impl]`, and attribute `#[export_method]` are used to mark functions "exported" in golang's 
//! `net/rpc` perspective. 
//!
//! More detailed usage can be found in the book and documentation.
//! 
//! - [Book](https://minghuaw.github.io/toy-rpc/01_introduction.html)
//! - [Documentation](https://docs.rs/toy-rpc/0.7.0-alpha.0/toy_rpc/)
//! - [GitHub repository](https://github.com/minghuaw/toy-rpc)
//! - [crate.io](https://crates.io/crates/toy-rpc)
//! 
//! This crate uses `#![forbid(unsafe_code)]` to ensure no usage of `unsafe` in the crate.
//! 
//! # Feature flags
//! 
//! The feature flags can be put into three categories.
//! 
//! Choice of serialization/deserialzation (only one should be enabled at a time)
//! 
//! - `serde_bincode`: the default codec will use `bincode`
//!     for serialization/deserialization
//! - `serde_json`: the default codec will use `serde_json`
//!     for `json` serialization/deserialization
//! - `serde_cbor`: the default codec will use `serde_cbor`
//!     for serialization/deserialization
//! - `serde_rmp`: the default codec will use `rmp-serde`
//!     for serialization/deserialization
//! 
//! Choice of runtime and HTTP framework integration
//! 
//! - `async_std_runtime`: supports usage with `async-std`
//! - `tokio_runtime`: supports usage with `tokio`
//! - `http_tide`: enables `tide` integration on the server side. This also enables `async_std_runtime`
//! - `http_actix_web`: enables `actix-web` integration on the server side. This also enables `tokio_runtime`
//! - `http_warp`: enables integration with `warp` on the server side. This also enables `tokio_runtime`
//! 
//! Choice of RPC server or client (both can be enabled at the same time)
//! 
//! - `server`: enables RPC server
//! - `client`: enables RPC client 
//! 
//! Other trivial feature flags are listed below, and they are likely of no actual usage for you.
//! - `docs`
//! - `std`: `serde/std`. There is no actual usage right now.
//! 
//! By default, only `serde_bincode` feature is enabled. 
//! You must enable at least one runtime feature flag and the `server` and/or `client` to have something usable.
//! 
//! ## Default features
//! 
//! ```toml
//! default = ["serde_bincode"]
//! ```
//! 
//! # Integration
//! 
//! HTTP integration is provided for `actix-web`, `tide`, and `warp`. More details can be found
//! in the [Book/Integrations](https://minghuaw.github.io/toy-rpc/05_integration.html) and in 
//! [examples](https://github.com/minghuaw/toy-rpc/tree/main/examples).
//! 
//! # Quickstart Example
//! 
//! A quickstart example with `tokio` runtime is provided in the [Book/Quickstart](https://minghuaw.github.io/toy-rpc/02_quickstart.html).
//! 

pub mod codec;
pub mod error;
pub mod macros;
pub mod message;
pub mod service;
pub mod transport;
pub mod util;

/// The default path added to the HTTP url 
#[cfg(any(
    feature = "http_actix_web",
    feature = "http_warp",
    feature = "http_tide",
    feature = "client"
))]
pub const DEFAULT_RPC_PATH: &str = "_rpc_";

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub use server::{Server, ServerBuilder};

pub use error::Error;

// re-export
pub use erased_serde;
pub use serde;
