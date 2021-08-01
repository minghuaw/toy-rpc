//! Integration with axum using WebSocket 
//! A separate implementation is required because `axum` has wrapped `tungstenite` types

use super::*;
use axum::ws::{Message, WebSocket};