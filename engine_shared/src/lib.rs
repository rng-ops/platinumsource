//! `engine_shared`
//!
//! Shared libraries used by both client and server.
//!
//! Design goals:
//! - Deterministic and modular where practical.
//! - Clear separation of concerns (net, ecs, math, events, resources).
//! - Traits for abstraction and dependency injection.
//! - No `unsafe`.

pub mod auth;
pub mod bsp;
pub mod chat;
pub mod config;
pub mod console;
pub mod ecs;
pub mod event;
pub mod gsi;
pub mod lobby;
pub mod math;
pub mod net;
pub mod physics;
pub mod render;
pub mod resources;
pub mod steam_id;
pub mod test_report;

pub mod prelude {
    //! Commonly used exports.

    pub use crate::config::*;
    pub use crate::ecs::*;
    pub use crate::event::*;
    pub use crate::math::*;
    pub use crate::net::*;
    pub use crate::steam_id::*;
}
