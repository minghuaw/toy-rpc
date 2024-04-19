#[cfg(feature = "http_tide")]
#[cfg_attr(doc, doc(cfg(feature = "http_tide")))]
mod http_tide;

#[cfg(all(feature = "http_warp"))]
#[cfg_attr(doc, doc(cfg(feature = "http_warp")))]
mod http_warp;

#[cfg(all(feature = "http_axum"))]
#[cfg_attr(doc, doc(cfg(feature = "http_axum")))]
mod http_axum;
