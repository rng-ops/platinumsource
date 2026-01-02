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
pub mod avatar;
pub mod bsp;
pub mod chat;
pub mod cloud;
pub mod config;
pub mod console;
pub mod dlc;
pub mod ecs;
pub mod event;
pub mod gsi;
pub mod leaderboard;
pub mod lobby;
pub mod matchmaking;
pub mod math;
pub mod net;
pub mod party;
pub mod physics;
pub mod render;
pub mod resources;
pub mod rich_presence;
pub mod social;
pub mod steam_id;
pub mod test_report;
pub mod voice;
pub mod workshop;

pub mod prelude {
    //! Commonly used exports.

    pub use crate::config::*;
    pub use crate::ecs::*;
    pub use crate::event::*;
    pub use crate::math::*;
    pub use crate::net::*;
    pub use crate::steam_id::*;
}
