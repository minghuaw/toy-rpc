#[cfg(any(
    feature = "serde_bincode",
    feature = "serde_cbor",
    feature = "serde_rmp"
))]
pub(crate) mod frame;

pub(crate) mod ws;