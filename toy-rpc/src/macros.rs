//! Re-export of proc_macros defined in `toy_rpc_macros`

pub use toy_rpc_macros::{export_impl, export_trait, export_trait_impl, Topic};

#[cfg(all(
    any(
        feature = "async_std_runtime",
        feature = "tokio_runtime",
        feature = "http_tide",
        feature = "http_warp",
        feature = "http_actix_web"
    ),
    any(
        all(
            feature = "serde_bincode",
            not(feature = "serde_json"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_cbor",
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_json",
            not(feature = "serde_bincode"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_rmp",
            not(feature = "serde_cbor"),
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
        )
    )
))]
pub(crate) use toy_rpc_macros::impl_inner_deserializer;
