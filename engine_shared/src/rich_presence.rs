//! Rich Presence implementation.
//!
//! # Valve Documentation Reference
//! - [Steam Rich Presence](https://partner.steamgames.com/doc/features/enhancedrichpresence)
//! - [ISteamFriends Rich Presence](https://partner.steamgames.com/doc/api/ISteamFriends#SetRichPresence)
//!
//! # Standard Keys
//! - **status**: Display string in friends list
//! - **connect**: Server connection string (+connect ip:port)
//! - **steam_display**: Localization token for display
//! - **steam_player_group**: Group identifier for party display
//! - **steam_player_group_size**: Number of players in group

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::steam_id::SteamId;

/// Maximum number of rich presence keys per user.
pub const MAX_RICH_PRESENCE_KEYS: usize = 20;

/// Maximum length of a rich presence key.
pub const MAX_KEY_LENGTH: usize = 64;

/// Maximum length of a rich presence value.
pub const MAX_VALUE_LENGTH: usize = 256;

/// Standard rich presence keys.
pub mod keys {
    /// Display status in friends list.
    pub const STATUS: &str = "status";
    /// Server connection string.
    pub const CONNECT: &str = "connect";
    /// Localization token for display.
    pub const STEAM_DISPLAY: &str = "steam_display";
    /// Group identifier for party.
    pub const STEAM_PLAYER_GROUP: &str = "steam_player_group";
    /// Number of players in group.
    pub const STEAM_PLAYER_GROUP_SIZE: &str = "steam_player_group_size";
}

/// Rich presence data for a player.
#[derive(Debug, Clone, Default)]
pub struct RichPresenceData {
    /// Key-value pairs for rich presence.
    data: HashMap<String, String>,
    /// Last update time.
    last_update: Option<Instant>,
}

impl RichPresenceData {
    /// Create new empty rich presence data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a rich presence key-value pair.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), RichPresenceError> {
        if key.len() > MAX_KEY_LENGTH {
            return Err(RichPresenceError::KeyTooLong);
        }
        if value.len() > MAX_VALUE_LENGTH {
            return Err(RichPresenceError::ValueTooLong);
        }
        if self.data.len() >= MAX_RICH_PRESENCE_KEYS && !self.data.contains_key(key) {
            return Err(RichPresenceError::TooManyKeys);
        }

        self.data.insert(key.to_string(), value.to_string());
        self.last_update = Some(Instant::now());
        Ok(())
    }

    /// Get a rich presence value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Get the number of keys.
    pub fn key_count(&self) -> usize {
        self.data.len()
    }

    /// Get a key by index.
    pub fn get_key_by_index(&self, index: usize) -> Option<&str> {
        self.data.keys().nth(index).map(|s| s.as_str())
    }

    /// Clear all rich presence data.
    pub fn clear(&mut self) {
        self.data.clear();
        self.last_update = Some(Instant::now());
    }

    /// Check if data is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get last update time.
    pub fn last_update(&self) -> Option<Instant> {
        self.last_update
    }

    /// Get status string.
    pub fn status(&self) -> Option<&str> {
        self.get(keys::STATUS)
    }

    /// Get connect string.
    pub fn connect(&self) -> Option<&str> {
        self.get(keys::CONNECT)
    }

    /// Set status string.
    pub fn set_status(&mut self, status: &str) -> Result<(), RichPresenceError> {
        self.set(keys::STATUS, status)
    }

    /// Set connect string.
    pub fn set_connect(&mut self, connect: &str) -> Result<(), RichPresenceError> {
        self.set(keys::CONNECT, connect)
    }

    /// Set player group info.
    pub fn set_player_group(
        &mut self,
        group_id: &str,
        group_size: u32,
    ) -> Result<(), RichPresenceError> {
        self.set(keys::STEAM_PLAYER_GROUP, group_id)?;
        self.set(keys::STEAM_PLAYER_GROUP_SIZE, &group_size.to_string())
    }
}

/// Rich presence errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RichPresenceError {
    KeyTooLong,
    ValueTooLong,
    TooManyKeys,
    InvalidKey,
    NotFound,
}

/// Rich presence callback data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRichPresenceUpdate {
    /// Steam ID of the friend whose presence updated.
    pub steam_id: SteamId,
    /// App ID this update is for.
    pub app_id: u32,
}

/// Rate limiter for rich presence updates.
#[derive(Debug, Clone)]
pub struct RichPresenceRateLimiter {
    /// Last update time.
    last_update: Option<Instant>,
    /// Minimum interval between updates.
    min_interval: Duration,
}

impl RichPresenceRateLimiter {
    /// Create a new rate limiter with the given minimum interval.
    pub fn new(min_interval: Duration) -> Self {
        Self {
            last_update: None,
            min_interval,
        }
    }

    /// Check if an update is allowed.
    pub fn can_update(&self) -> bool {
        match self.last_update {
            None => true,
            Some(last) => last.elapsed() >= self.min_interval,
        }
    }

    /// Record an update.
    pub fn record_update(&mut self) {
        self.last_update = Some(Instant::now());
    }

    /// Get time until next update is allowed.
    pub fn time_until_allowed(&self) -> Duration {
        match self.last_update {
            None => Duration::ZERO,
            Some(last) => {
                let elapsed = last.elapsed();
                if elapsed >= self.min_interval {
                    Duration::ZERO
                } else {
                    self.min_interval - elapsed
                }
            }
        }
    }
}

impl Default for RichPresenceRateLimiter {
    fn default() -> Self {
        // Default: max one update per second
        Self::new(Duration::from_secs(1))
    }
}

/// Rich presence manager for tracking multiple players.
#[derive(Default)]
pub struct RichPresenceManager {
    /// Local player's rich presence.
    local_presence: RichPresenceData,
    /// Friend rich presence data.
    friend_presence: HashMap<SteamId, RichPresenceData>,
    /// Rate limiter for updates.
    rate_limiter: RichPresenceRateLimiter,
    /// Pending callbacks.
    pending_callbacks: Vec<FriendRichPresenceUpdate>,
}

impl RichPresenceManager {
    /// Create a new rich presence manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set local rich presence.
    pub fn set_rich_presence(&mut self, key: &str, value: &str) -> Result<(), RichPresenceError> {
        self.local_presence.set(key, value)
    }

    /// Clear local rich presence.
    pub fn clear_rich_presence(&mut self) {
        self.local_presence.clear();
    }

    /// Get local rich presence value.
    pub fn get_local_presence(&self, key: &str) -> Option<&str> {
        self.local_presence.get(key)
    }

    /// Get friend's rich presence.
    pub fn get_friend_rich_presence(&self, friend: SteamId, key: &str) -> Option<&str> {
        self.friend_presence.get(&friend).and_then(|p| p.get(key))
    }

    /// Get friend's rich presence key count.
    pub fn get_friend_rich_presence_key_count(&self, friend: SteamId) -> usize {
        self.friend_presence
            .get(&friend)
            .map(|p| p.key_count())
            .unwrap_or(0)
    }

    /// Get friend's rich presence key by index.
    pub fn get_friend_rich_presence_key_by_index(
        &self,
        friend: SteamId,
        index: usize,
    ) -> Option<&str> {
        self.friend_presence
            .get(&friend)
            .and_then(|p| p.get_key_by_index(index))
    }

    /// Update friend's rich presence (called when receiving callback).
    pub fn update_friend_presence(
        &mut self,
        friend: SteamId,
        app_id: u32,
        data: RichPresenceData,
    ) {
        self.friend_presence.insert(friend, data);
        self.pending_callbacks.push(FriendRichPresenceUpdate {
            steam_id: friend,
            app_id,
        });
    }

    /// Get and clear pending callbacks.
    pub fn drain_callbacks(&mut self) -> Vec<FriendRichPresenceUpdate> {
        std::mem::take(&mut self.pending_callbacks)
    }

    /// Check if rate limiter allows update.
    pub fn can_update(&self) -> bool {
        self.rate_limiter.can_update()
    }

    /// Record an update for rate limiting.
    pub fn record_update(&mut self) {
        self.rate_limiter.record_update();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    fn test_steam_id(n: u32) -> SteamId {
        SteamId::from_account_id(n)
    }

    // =============================================================================
    // RP-001: Set Rich Presence
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#SetRichPresence
    // =============================================================================

    #[test]
    fn rp_001_set_rich_presence() {
        let mut manager = RichPresenceManager::new();

        manager.set_rich_presence("status", "In Match").unwrap();
        manager.set_rich_presence("map", "de_dust2").unwrap();

        assert_eq!(manager.get_local_presence("status"), Some("In Match"));
        assert_eq!(manager.get_local_presence("map"), Some("de_dust2"));
    }

    #[test]
    fn rp_001_overwrite_value() {
        let mut manager = RichPresenceManager::new();

        manager.set_rich_presence("status", "In Lobby").unwrap();
        manager.set_rich_presence("status", "In Match").unwrap();

        assert_eq!(manager.get_local_presence("status"), Some("In Match"));
    }

    // =============================================================================
    // RP-002: Clear Rich Presence
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#ClearRichPresence
    // =============================================================================

    #[test]
    fn rp_002_clear_rich_presence() {
        let mut manager = RichPresenceManager::new();

        manager.set_rich_presence("status", "Playing").unwrap();
        manager.set_rich_presence("map", "cs_office").unwrap();

        manager.clear_rich_presence();

        assert!(manager.get_local_presence("status").is_none());
        assert!(manager.get_local_presence("map").is_none());
    }

    // =============================================================================
    // RP-003: Get Friend Rich Presence
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetFriendRichPresence
    // =============================================================================

    #[test]
    fn rp_003_get_friend_presence() {
        let mut manager = RichPresenceManager::new();
        let friend = test_steam_id(12345);

        let mut friend_data = RichPresenceData::new();
        friend_data.set("status", "In Game").unwrap();
        friend_data.set("map", "de_mirage").unwrap();

        manager.update_friend_presence(friend, 730, friend_data);

        assert_eq!(
            manager.get_friend_rich_presence(friend, "status"),
            Some("In Game")
        );
        assert_eq!(
            manager.get_friend_rich_presence(friend, "map"),
            Some("de_mirage")
        );
    }

    // =============================================================================
    // RP-004: Rich Presence Key Count
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetFriendRichPresenceKeyCount
    // =============================================================================

    #[test]
    fn rp_004_key_count() {
        let mut manager = RichPresenceManager::new();
        let friend = test_steam_id(12345);

        let mut friend_data = RichPresenceData::new();
        friend_data.set("status", "Playing").unwrap();
        friend_data.set("map", "de_dust2").unwrap();
        friend_data.set("mode", "competitive").unwrap();

        manager.update_friend_presence(friend, 730, friend_data);

        assert_eq!(manager.get_friend_rich_presence_key_count(friend), 3);
    }

    #[test]
    fn rp_004_unknown_friend_zero_keys() {
        let manager = RichPresenceManager::new();
        let unknown = test_steam_id(99999);

        assert_eq!(manager.get_friend_rich_presence_key_count(unknown), 0);
    }

    // =============================================================================
    // RP-005: Steam Group Key
    // =============================================================================

    #[test]
    fn rp_005_player_group() {
        let mut data = RichPresenceData::new();
        data.set_player_group("group_12345", 4).unwrap();

        assert_eq!(data.get(keys::STEAM_PLAYER_GROUP), Some("group_12345"));
        assert_eq!(data.get(keys::STEAM_PLAYER_GROUP_SIZE), Some("4"));
    }

    // =============================================================================
    // RP-006: Connect String
    // Reference: https://partner.steamgames.com/doc/features/enhancedrichpresence
    // =============================================================================

    #[test]
    fn rp_006_connect_string() {
        let mut data = RichPresenceData::new();
        data.set_connect("+connect 192.168.1.100:27015").unwrap();

        assert_eq!(data.connect(), Some("+connect 192.168.1.100:27015"));
    }

    // =============================================================================
    // RP-007: Status String
    // =============================================================================

    #[test]
    fn rp_007_status_string() {
        let mut data = RichPresenceData::new();
        data.set_status("Playing Competitive on de_dust2").unwrap();

        assert_eq!(data.status(), Some("Playing Competitive on de_dust2"));
    }

    // =============================================================================
    // RP-009: Rich Presence Update Rate
    // =============================================================================

    #[test]
    fn rp_009_rate_limiting() {
        let mut limiter = RichPresenceRateLimiter::new(Duration::from_millis(50));

        assert!(limiter.can_update());
        limiter.record_update();
        assert!(!limiter.can_update());

        sleep(Duration::from_millis(60));
        assert!(limiter.can_update());
    }

    // =============================================================================
    // RP-010: Rich Presence Callback
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#FriendRichPresenceUpdate_t
    // =============================================================================

    #[test]
    fn rp_010_presence_update_callback() {
        let mut manager = RichPresenceManager::new();
        let friend = test_steam_id(12345);

        let mut friend_data = RichPresenceData::new();
        friend_data.set("status", "Online").unwrap();

        manager.update_friend_presence(friend, 730, friend_data);

        let callbacks = manager.drain_callbacks();
        assert_eq!(callbacks.len(), 1);
        assert_eq!(callbacks[0].steam_id, friend);
        assert_eq!(callbacks[0].app_id, 730);
    }

    // =============================================================================
    // Validation Tests
    // =============================================================================

    #[test]
    fn key_length_limit() {
        let mut data = RichPresenceData::new();
        let long_key = "k".repeat(MAX_KEY_LENGTH + 1);

        let result = data.set(&long_key, "value");
        assert_eq!(result, Err(RichPresenceError::KeyTooLong));
    }

    #[test]
    fn value_length_limit() {
        let mut data = RichPresenceData::new();
        let long_value = "v".repeat(MAX_VALUE_LENGTH + 1);

        let result = data.set("key", &long_value);
        assert_eq!(result, Err(RichPresenceError::ValueTooLong));
    }

    #[test]
    fn max_keys_limit() {
        let mut data = RichPresenceData::new();

        for i in 0..MAX_RICH_PRESENCE_KEYS {
            data.set(&format!("key{}", i), "value").unwrap();
        }

        let result = data.set("one_more", "value");
        assert_eq!(result, Err(RichPresenceError::TooManyKeys));
    }

    #[test]
    fn update_existing_key_within_limit() {
        let mut data = RichPresenceData::new();

        for i in 0..MAX_RICH_PRESENCE_KEYS {
            data.set(&format!("key{}", i), "value").unwrap();
        }

        // Updating existing key should work
        data.set("key0", "new_value").unwrap();
        assert_eq!(data.get("key0"), Some("new_value"));
    }
}
