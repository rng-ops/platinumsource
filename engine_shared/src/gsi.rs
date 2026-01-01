//! Game State Integration (GSI) implementation.
//!
//! # Valve Documentation Reference
//! - [CS:GO Game State Integration](https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Game_State_Integration)
//! - [Dota 2 GSI](https://developer.valvesoftware.com/wiki/Dota_2_Game_State_Integration)
//!
//! # Overview
//! GSI sends game state as JSON payloads to configured HTTP endpoints.
//! This enables third-party tools like stream overlays, LED integration,
//! and analytics platforms.
//!
//! # Payload Structure
//! - **provider**: Game identity (name, appid, version, steamid)
//! - **map**: Current map state (name, phase, round, scores)
//! - **player**: Local player data (team, state, weapons)
//! - **round**: Round timing and phase
//! - **previously**: Changed values from last update
//! - **added**: New values since last update

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::steam_id::SteamId;

/// GSI Provider information.
/// 
/// Reference: Counter-Strike: Global Offensive Game State Integration docs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GsiProvider {
    /// Game name (e.g., "Counter-Strike 2").
    pub name: String,
    /// Steam App ID.
    pub appid: u32,
    /// Game version/build number.
    pub version: u32,
    /// Player's Steam ID.
    pub steamid: String,
    /// Unix timestamp.
    pub timestamp: u64,
}

impl GsiProvider {
    pub fn new(name: &str, appid: u32, version: u32, steam_id: SteamId) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        GsiProvider {
            name: name.to_string(),
            appid,
            version,
            steamid: steam_id.to_string(),
            timestamp,
        }
    }
}

/// Map phase states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MapPhase {
    Warmup,
    Live,
    Intermission,
    GameOver,
}

/// Game mode types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameMode {
    Competitive,
    Casual,
    Deathmatch,
    Custom,
    Coop,
    Survival,
}

/// Team score information.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TeamScore {
    pub score: u32,
    #[serde(default)]
    pub consecutive_round_losses: u32,
    #[serde(default)]
    pub timeouts_remaining: u32,
    #[serde(default)]
    pub matches_won_this_series: u32,
}

/// Map state information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GsiMap {
    /// Game mode.
    pub mode: GameMode,
    /// Map name (e.g., "de_dust2").
    pub name: String,
    /// Current phase.
    pub phase: MapPhase,
    /// Current round number (1-indexed).
    pub round: u32,
    /// Team CT scores.
    pub team_ct: TeamScore,
    /// Team T scores.
    pub team_t: TeamScore,
    /// Number of matches to win series.
    #[serde(default)]
    pub num_matches_to_win_series: u32,
}

impl GsiMap {
    pub fn new(name: &str, mode: GameMode) -> Self {
        GsiMap {
            mode,
            name: name.to_string(),
            phase: MapPhase::Warmup,
            round: 0,
            team_ct: TeamScore::default(),
            team_t: TeamScore::default(),
            num_matches_to_win_series: 0,
        }
    }
}

/// Player activity states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerActivity {
    Playing,
    Menu,
    TextInput,
}

/// Player team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerTeam {
    CT,
    T,
    #[serde(rename = "unassigned")]
    Unassigned,
    #[serde(rename = "spectator")]
    Spectator,
}

/// Player state (health, armor, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PlayerState {
    pub health: u32,
    pub armor: u32,
    pub helmet: bool,
    pub flashed: u32,
    pub smoked: u32,
    pub burning: u32,
    pub money: u32,
    pub round_kills: u32,
    pub round_killhs: u32,
    pub round_totaldmg: u32,
    pub equip_value: u32,
}

/// Player weapon information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerWeapon {
    pub name: String,
    #[serde(rename = "type")]
    pub weapon_type: String,
    #[serde(default)]
    pub ammo_clip: Option<u32>,
    #[serde(default)]
    pub ammo_clip_max: Option<u32>,
    #[serde(default)]
    pub ammo_reserve: Option<u32>,
    pub state: String, // "active", "holstered"
}

/// Player information in GSI payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GsiPlayer {
    /// Player's Steam ID.
    pub steamid: String,
    /// Clan tag.
    #[serde(default)]
    pub clan: String,
    /// Display name.
    pub name: String,
    /// Observer slot (0-9).
    #[serde(default)]
    pub observer_slot: u32,
    /// Team.
    pub team: PlayerTeam,
    /// Current activity.
    pub activity: PlayerActivity,
    /// Player state (health, armor, etc.).
    pub state: PlayerState,
    /// Equipped weapons.
    #[serde(default)]
    pub weapons: HashMap<String, PlayerWeapon>,
    /// Match stats.
    #[serde(default)]
    pub match_stats: HashMap<String, Value>,
}

impl GsiPlayer {
    pub fn new(steam_id: SteamId, name: &str, team: PlayerTeam) -> Self {
        GsiPlayer {
            steamid: steam_id.to_string(),
            clan: String::new(),
            name: name.to_string(),
            observer_slot: 0,
            team,
            activity: PlayerActivity::Playing,
            state: PlayerState::default(),
            weapons: HashMap::new(),
            match_stats: HashMap::new(),
        }
    }
}

/// Round phase states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoundPhase {
    Freezetime,
    Live,
    Over,
}

/// Round information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GsiRound {
    pub phase: RoundPhase,
    #[serde(default)]
    pub bomb: Option<String>,
    #[serde(default)]
    pub win_team: Option<PlayerTeam>,
}

impl Default for GsiRound {
    fn default() -> Self {
        GsiRound {
            phase: RoundPhase::Freezetime,
            bomb: None,
            win_team: None,
        }
    }
}

/// Phase countdown timers.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GsiPhaseCountdowns {
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub phase_ends_in: Option<f32>,
}

/// Authentication block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GsiAuth {
    pub token: String,
}

/// Full GSI payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GsiPayload {
    pub provider: GsiProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<GsiMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player: Option<GsiPlayer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<GsiRound>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_countdowns: Option<GsiPhaseCountdowns>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previously: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<GsiAuth>,
}

impl GsiPayload {
    pub fn new(provider: GsiProvider) -> Self {
        GsiPayload {
            provider,
            map: None,
            player: None,
            round: None,
            phase_countdowns: None,
            previously: None,
            added: None,
            auth: None,
        }
    }

    /// Set auth token.
    pub fn with_auth(mut self, token: &str) -> Self {
        self.auth = Some(GsiAuth {
            token: token.to_string(),
        });
        self
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// GSI configuration parsed from gamestate_integration_*.cfg files.
#[derive(Debug, Clone)]
pub struct GsiConfig {
    pub uri: String,
    pub timeout: Duration,
    pub buffer: Duration,
    pub throttle: Duration,
    pub heartbeat: Duration,
    pub auth_token: Option<String>,
    pub data_subscriptions: GsiDataSubscriptions,
}

/// Which data blocks to include in GSI payloads.
#[derive(Debug, Clone, Default)]
pub struct GsiDataSubscriptions {
    pub provider: bool,
    pub map: bool,
    pub round: bool,
    pub player_id: bool,
    pub player_state: bool,
    pub player_weapons: bool,
    pub player_match_stats: bool,
    pub allplayers_id: bool,
    pub allplayers_state: bool,
    pub allplayers_match_stats: bool,
    pub allplayers_weapons: bool,
    pub allplayers_position: bool,
    pub phase_countdowns: bool,
    pub allgrenades: bool,
    pub bomb: bool,
}

impl Default for GsiConfig {
    fn default() -> Self {
        GsiConfig {
            uri: "http://127.0.0.1:3000".to_string(),
            timeout: Duration::from_secs(1),
            buffer: Duration::from_millis(100),
            throttle: Duration::from_millis(100),
            heartbeat: Duration::from_secs(60),
            auth_token: None,
            data_subscriptions: GsiDataSubscriptions::default(),
        }
    }
}

/// GSI receiver for accepting payloads.
pub struct GsiReceiver {
    expected_token: Option<String>,
    last_payload: Option<GsiPayload>,
    payload_count: u64,
}

impl GsiReceiver {
    pub fn new(expected_token: Option<String>) -> Self {
        GsiReceiver {
            expected_token,
            last_payload: None,
            payload_count: 0,
        }
    }

    /// Process a received payload.
    pub fn process(&mut self, json: &str) -> Result<&GsiPayload, GsiError> {
        let payload: GsiPayload = serde_json::from_str(json)
            .map_err(|e| GsiError::ParseError(e.to_string()))?;

        // Validate auth token if required
        if let Some(ref expected) = self.expected_token {
            match &payload.auth {
                Some(auth) if &auth.token == expected => {}
                Some(_) => return Err(GsiError::InvalidToken),
                None => return Err(GsiError::MissingToken),
            }
        }

        self.payload_count += 1;
        self.last_payload = Some(payload);
        Ok(self.last_payload.as_ref().unwrap())
    }

    /// Get the last received payload.
    pub fn last_payload(&self) -> Option<&GsiPayload> {
        self.last_payload.as_ref()
    }

    /// Get total payload count.
    pub fn payload_count(&self) -> u64 {
        self.payload_count
    }
}

/// GSI errors.
#[derive(Debug, Clone, PartialEq)]
pub enum GsiError {
    ParseError(String),
    InvalidToken,
    MissingToken,
    Timeout,
    ConnectionFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_steam_id() -> SteamId {
        SteamId::from_account_id(12345678)
    }

    // =============================================================================
    // GSI-001: Provider Block
    // Reference: https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Game_State_Integration
    // =============================================================================

    #[test]
    fn gsi_001_provider_block() {
        let provider = GsiProvider::new("Counter-Strike 2", 730, 14000, test_steam_id());
        
        assert_eq!(provider.name, "Counter-Strike 2");
        assert_eq!(provider.appid, 730);
        assert_eq!(provider.version, 14000);
        assert!(!provider.steamid.is_empty());
        assert!(provider.timestamp > 0);
    }

    #[test]
    fn gsi_001_provider_serialization() {
        let provider = GsiProvider::new("Test Game", 440, 1000, test_steam_id());
        let json = serde_json::to_string(&provider).unwrap();
        
        assert!(json.contains("\"name\":\"Test Game\""));
        assert!(json.contains("\"appid\":440"));
    }

    // =============================================================================
    // GSI-002: Map Block
    // =============================================================================

    #[test]
    fn gsi_002_map_block() {
        let map = GsiMap::new("de_dust2", GameMode::Competitive);
        
        assert_eq!(map.name, "de_dust2");
        assert_eq!(map.mode, GameMode::Competitive);
        assert_eq!(map.phase, MapPhase::Warmup);
        assert_eq!(map.round, 0);
    }

    #[test]
    fn gsi_002_map_scores() {
        let mut map = GsiMap::new("de_mirage", GameMode::Competitive);
        map.team_ct.score = 10;
        map.team_t.score = 5;
        map.round = 16;
        map.phase = MapPhase::Live;
        
        let json = serde_json::to_string(&map).unwrap();
        let parsed: GsiMap = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.team_ct.score, 10);
        assert_eq!(parsed.team_t.score, 5);
        assert_eq!(parsed.round, 16);
    }

    // =============================================================================
    // GSI-003: Player Block
    // =============================================================================

    #[test]
    fn gsi_003_player_block() {
        let player = GsiPlayer::new(test_steam_id(), "TestPlayer", PlayerTeam::CT);
        
        assert_eq!(player.name, "TestPlayer");
        assert_eq!(player.team, PlayerTeam::CT);
        assert_eq!(player.activity, PlayerActivity::Playing);
    }

    #[test]
    fn gsi_003_player_state() {
        let mut player = GsiPlayer::new(test_steam_id(), "TestPlayer", PlayerTeam::T);
        player.state.health = 100;
        player.state.armor = 100;
        player.state.helmet = true;
        player.state.money = 4750;
        
        let json = serde_json::to_string(&player).unwrap();
        let parsed: GsiPlayer = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.state.health, 100);
        assert_eq!(parsed.state.armor, 100);
        assert!(parsed.state.helmet);
        assert_eq!(parsed.state.money, 4750);
    }

    // =============================================================================
    // GSI-005: Round Block
    // =============================================================================

    #[test]
    fn gsi_005_round_block() {
        let round = GsiRound {
            phase: RoundPhase::Live,
            bomb: Some("planted".to_string()),
            win_team: None,
        };
        
        let json = serde_json::to_string(&round).unwrap();
        let parsed: GsiRound = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.phase, RoundPhase::Live);
        assert_eq!(parsed.bomb, Some("planted".to_string()));
    }

    // =============================================================================
    // GSI-009: Auth Token
    // =============================================================================

    #[test]
    fn gsi_009_auth_token() {
        let provider = GsiProvider::new("Test", 730, 1, test_steam_id());
        let payload = GsiPayload::new(provider).with_auth("secret_token_123");
        
        assert!(payload.auth.is_some());
        assert_eq!(payload.auth.unwrap().token, "secret_token_123");
    }

    #[test]
    fn gsi_009_auth_validation() {
        let mut receiver = GsiReceiver::new(Some("correct_token".to_string()));
        
        let provider = GsiProvider::new("Test", 730, 1, test_steam_id());
        let payload = GsiPayload::new(provider).with_auth("correct_token");
        let json = payload.to_json().unwrap();
        
        assert!(receiver.process(&json).is_ok());
    }

    #[test]
    fn gsi_009_invalid_token_rejected() {
        let mut receiver = GsiReceiver::new(Some("correct_token".to_string()));
        
        let provider = GsiProvider::new("Test", 730, 1, test_steam_id());
        let payload = GsiPayload::new(provider).with_auth("wrong_token");
        let json = payload.to_json().unwrap();
        
        let result = receiver.process(&json);
        assert_eq!(result, Err(GsiError::InvalidToken));
    }

    #[test]
    fn gsi_009_missing_token_rejected() {
        let mut receiver = GsiReceiver::new(Some("expected_token".to_string()));
        
        let provider = GsiProvider::new("Test", 730, 1, test_steam_id());
        let payload = GsiPayload::new(provider); // No auth
        let json = payload.to_json().unwrap();
        
        let result = receiver.process(&json);
        assert_eq!(result, Err(GsiError::MissingToken));
    }

    // =============================================================================
    // GSI-010: HTTP POST Delivery (parsing tests)
    // =============================================================================

    #[test]
    fn gsi_010_full_payload_roundtrip() {
        let provider = GsiProvider::new("Counter-Strike 2", 730, 14000, test_steam_id());
        let mut payload = GsiPayload::new(provider);
        
        payload.map = Some(GsiMap::new("de_dust2", GameMode::Competitive));
        payload.player = Some(GsiPlayer::new(test_steam_id(), "Player1", PlayerTeam::CT));
        payload.round = Some(GsiRound::default());
        payload = payload.with_auth("test_token");
        
        let json = payload.to_json_pretty().unwrap();
        let parsed = GsiPayload::from_json(&json).unwrap();
        
        assert_eq!(parsed.provider.appid, 730);
        assert!(parsed.map.is_some());
        assert!(parsed.player.is_some());
        assert!(parsed.round.is_some());
        assert!(parsed.auth.is_some());
    }

    #[test]
    fn gsi_010_minimal_payload() {
        let provider = GsiProvider::new("Test", 440, 1, test_steam_id());
        let payload = GsiPayload::new(provider);
        
        let json = payload.to_json().unwrap();
        
        // Should not contain optional fields
        assert!(!json.contains("\"map\""));
        assert!(!json.contains("\"player\""));
        assert!(!json.contains("\"auth\""));
    }

    // =============================================================================
    // Payload Counter Tests
    // =============================================================================

    #[test]
    fn receiver_counts_payloads() {
        let mut receiver = GsiReceiver::new(None);
        
        for i in 0..5 {
            let provider = GsiProvider::new("Test", 730, i, test_steam_id());
            let payload = GsiPayload::new(provider);
            receiver.process(&payload.to_json().unwrap()).unwrap();
        }
        
        assert_eq!(receiver.payload_count(), 5);
    }

    #[test]
    fn receiver_stores_last_payload() {
        let mut receiver = GsiReceiver::new(None);
        
        let provider = GsiProvider::new("Test", 730, 100, test_steam_id());
        let payload = GsiPayload::new(provider);
        receiver.process(&payload.to_json().unwrap()).unwrap();
        
        let last = receiver.last_payload().unwrap();
        assert_eq!(last.provider.version, 100);
    }

    // =============================================================================
    // Config Tests
    // =============================================================================

    #[test]
    fn default_config() {
        let config = GsiConfig::default();
        
        assert!(config.uri.starts_with("http"));
        assert!(config.timeout.as_secs() > 0);
    }
}
