//! `engine_client`
//!
//! Client-side systems:
//! - Connection management (reliable + unreliable channels)
//! - Input capture and command generation
//! - Prediction and reconciliation (placeholder)
//! - Interpolation for remote entity states
//! - Rendering abstraction wiring (placeholder)

pub mod client;
pub mod input;
pub mod interp;

pub use client::GameClient;
