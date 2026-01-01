//! `engine_server`
//!
//! Server-side systems:
//! - Fixed timestep simulation loop
//! - Entity management
//! - Receives `PlayerCommand`s
//! - Sends `Snapshot`s
//!
//! Networking model:
//! - TCP: handshake/control plane
//! - UDP: gameplay plane (commands/snapshots)

pub mod server;

pub use server::GameServer;
