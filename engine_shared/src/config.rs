//! Configuration system.
//!
//! Loads engine configuration from JSON strings/files (file IO left to app).

use serde::{Deserialize, Serialize};

/// Root configuration shared by client/server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Server listen address, e.g. `127.0.0.1:40000`.
    pub server_addr: String,
    /// Fixed simulation tick rate.
    pub tick_hz: u32,
    /// Path to maps directory.
    #[serde(default = "default_maps_dir")]
    pub maps_dir: String,
    /// Player name (client only).
    #[serde(default = "default_player_name")]
    pub player_name: String,
}

fn default_maps_dir() -> String {
    "maps".to_string()
}

fn default_player_name() -> String {
    "Player".to_string()
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:40000".to_string(),
            tick_hz: 64,
            maps_dir: default_maps_dir(),
            player_name: default_player_name(),
        }
    }
}

impl EngineConfig {
    /// Parses config from JSON.
    pub fn from_json_str(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}
