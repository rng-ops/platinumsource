//! Leaderboards and statistics implementation.
//!
//! # Valve Documentation Reference
//! - [Steam Leaderboards](https://partner.steamgames.com/doc/features/leaderboards)
//! - [ISteamUserStats](https://partner.steamgames.com/doc/api/ISteamUserStats)
//!
//! # Features
//! - Leaderboard creation and management
//! - Score upload and retrieval
//! - Statistics tracking
//! - Achievement progress

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::steam_id::SteamId;

/// Leaderboard handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LeaderboardHandle(u64);

impl LeaderboardHandle {
    pub const INVALID: LeaderboardHandle = LeaderboardHandle(0);

    pub fn new(id: u64) -> Self {
        LeaderboardHandle(id)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Leaderboard sort method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaderboardSortMethod {
    /// Lower scores are better (times, golf scores).
    Ascending,
    /// Higher scores are better (points, kills).
    Descending,
}

/// Leaderboard display type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaderboardDisplayType {
    /// Raw numeric value.
    Numeric,
    /// Seconds (e.g., 123 = "2:03").
    TimeSeconds,
    /// Milliseconds (e.g., 12345 = "12.345").
    TimeMilliSeconds,
}

/// Leaderboard data request type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardDataRequest {
    /// Global entries starting from top.
    Global,
    /// Entries around the current user.
    GlobalAroundUser,
    /// Friends only.
    Friends,
}

/// Upload score method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardUploadScoreMethod {
    /// Always keep the new score.
    ForceUpdate,
    /// Only update if new score is better.
    KeepBest,
}

/// Leaderboard entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Player's Steam ID.
    pub steam_id: SteamId,
    /// Global rank (1-indexed).
    pub global_rank: u32,
    /// Score value.
    pub score: i32,
    /// Optional details (extra data).
    pub details: Vec<i32>,
}

/// Leaderboard definition.
#[derive(Debug, Clone)]
pub struct Leaderboard {
    /// Leaderboard handle.
    pub handle: LeaderboardHandle,
    /// Leaderboard name.
    pub name: String,
    /// Sort method.
    pub sort_method: LeaderboardSortMethod,
    /// Display type.
    pub display_type: LeaderboardDisplayType,
    /// All entries, sorted.
    entries: Vec<LeaderboardEntry>,
}

impl Leaderboard {
    /// Create a new leaderboard.
    pub fn new(
        handle: LeaderboardHandle,
        name: &str,
        sort_method: LeaderboardSortMethod,
        display_type: LeaderboardDisplayType,
    ) -> Self {
        Leaderboard {
            handle,
            name: name.to_string(),
            sort_method,
            display_type,
            entries: Vec::new(),
        }
    }

    /// Get entry count.
    pub fn entry_count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Upload a score.
    pub fn upload_score(
        &mut self,
        steam_id: SteamId,
        score: i32,
        method: LeaderboardUploadScoreMethod,
        details: Vec<i32>,
    ) -> bool {
        let existing = self.entries.iter_mut().find(|e| e.steam_id == steam_id);

        match existing {
            Some(entry) => {
                let should_update = match method {
                    LeaderboardUploadScoreMethod::ForceUpdate => true,
                    LeaderboardUploadScoreMethod::KeepBest => match self.sort_method {
                        LeaderboardSortMethod::Ascending => score < entry.score,
                        LeaderboardSortMethod::Descending => score > entry.score,
                    },
                };

                if should_update {
                    entry.score = score;
                    entry.details = details;
                    self.recalculate_ranks();
                    true
                } else {
                    false
                }
            }
            None => {
                self.entries.push(LeaderboardEntry {
                    steam_id,
                    global_rank: 0,
                    score,
                    details,
                });
                self.recalculate_ranks();
                true
            }
        }
    }

    /// Sort entries and recalculate ranks.
    fn recalculate_ranks(&mut self) {
        self.entries.sort_by(|a, b| match self.sort_method {
            LeaderboardSortMethod::Ascending => a.score.cmp(&b.score),
            LeaderboardSortMethod::Descending => b.score.cmp(&a.score),
        });

        for (i, entry) in self.entries.iter_mut().enumerate() {
            entry.global_rank = (i + 1) as u32;
        }
    }

    /// Get entries in range.
    pub fn get_entries(&self, start: u32, count: u32) -> Vec<&LeaderboardEntry> {
        let start = start.saturating_sub(1) as usize;
        let end = (start + count as usize).min(self.entries.len());
        self.entries[start..end].iter().collect()
    }

    /// Get entries around a user.
    pub fn get_entries_around_user(
        &self,
        steam_id: SteamId,
        range_before: u32,
        range_after: u32,
    ) -> Vec<&LeaderboardEntry> {
        let user_idx = self.entries.iter().position(|e| e.steam_id == steam_id);

        match user_idx {
            Some(idx) => {
                let start = idx.saturating_sub(range_before as usize);
                let end = (idx + range_after as usize + 1).min(self.entries.len());
                self.entries[start..end].iter().collect()
            }
            None => Vec::new(),
        }
    }

    /// Get user's entry.
    pub fn get_user_entry(&self, steam_id: SteamId) -> Option<&LeaderboardEntry> {
        self.entries.iter().find(|e| e.steam_id == steam_id)
    }
}

/// Stat value types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatValue {
    Int(i32),
    Float(f32),
}

impl StatValue {
    pub fn as_int(&self) -> Option<i32> {
        match self {
            StatValue::Int(v) => Some(*v),
            StatValue::Float(v) => Some(*v as i32),
        }
    }

    pub fn as_float(&self) -> Option<f32> {
        match self {
            StatValue::Int(v) => Some(*v as f32),
            StatValue::Float(v) => Some(*v),
        }
    }
}

/// User statistics.
#[derive(Debug, Clone, Default)]
pub struct UserStats {
    /// Stat name -> value.
    stats: HashMap<String, StatValue>,
    /// Whether stats are loaded.
    loaded: bool,
    /// Whether stats have unsaved changes.
    dirty: bool,
}

impl UserStats {
    /// Create new user stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark stats as loaded.
    pub fn set_loaded(&mut self, loaded: bool) {
        self.loaded = loaded;
    }

    /// Check if stats are loaded.
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Get an integer stat.
    pub fn get_stat_int(&self, name: &str) -> Option<i32> {
        self.stats.get(name).and_then(|v| v.as_int())
    }

    /// Get a float stat.
    pub fn get_stat_float(&self, name: &str) -> Option<f32> {
        self.stats.get(name).and_then(|v| v.as_float())
    }

    /// Set an integer stat.
    pub fn set_stat_int(&mut self, name: &str, value: i32) {
        self.stats.insert(name.to_string(), StatValue::Int(value));
        self.dirty = true;
    }

    /// Set a float stat.
    pub fn set_stat_float(&mut self, name: &str, value: f32) {
        self.stats.insert(name.to_string(), StatValue::Float(value));
        self.dirty = true;
    }

    /// Check if there are unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear dirty flag (after storing).
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Reset all stats.
    pub fn reset_all(&mut self) {
        self.stats.clear();
        self.dirty = true;
    }
}

/// Leaderboard and stats manager.
#[derive(Default)]
pub struct LeaderboardManager {
    /// Leaderboards by name.
    leaderboards: HashMap<String, Leaderboard>,
    /// User stats.
    user_stats: UserStats,
    /// Next handle ID.
    next_handle: u64,
}

impl LeaderboardManager {
    /// Create a new manager.
    pub fn new() -> Self {
        LeaderboardManager {
            leaderboards: HashMap::new(),
            user_stats: UserStats::new(),
            next_handle: 1,
        }
    }

    /// Find a leaderboard by name.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUserStats#FindLeaderboard>
    pub fn find_leaderboard(&self, name: &str) -> Option<LeaderboardHandle> {
        self.leaderboards.get(name).map(|l| l.handle)
    }

    /// Find or create a leaderboard.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUserStats#FindOrCreateLeaderboard>
    pub fn find_or_create_leaderboard(
        &mut self,
        name: &str,
        sort_method: LeaderboardSortMethod,
        display_type: LeaderboardDisplayType,
    ) -> LeaderboardHandle {
        if let Some(handle) = self.find_leaderboard(name) {
            return handle;
        }

        let handle = LeaderboardHandle::new(self.next_handle);
        self.next_handle += 1;

        let leaderboard = Leaderboard::new(handle, name, sort_method, display_type);
        self.leaderboards.insert(name.to_string(), leaderboard);
        handle
    }

    /// Get leaderboard by handle.
    pub fn get_leaderboard(&self, handle: LeaderboardHandle) -> Option<&Leaderboard> {
        self.leaderboards.values().find(|l| l.handle == handle)
    }

    /// Get mutable leaderboard by handle.
    pub fn get_leaderboard_mut(&mut self, handle: LeaderboardHandle) -> Option<&mut Leaderboard> {
        self.leaderboards.values_mut().find(|l| l.handle == handle)
    }

    /// Upload score to leaderboard.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUserStats#UploadLeaderboardScore>
    pub fn upload_score(
        &mut self,
        handle: LeaderboardHandle,
        steam_id: SteamId,
        score: i32,
        method: LeaderboardUploadScoreMethod,
    ) -> bool {
        if let Some(lb) = self.get_leaderboard_mut(handle) {
            lb.upload_score(steam_id, score, method, Vec::new())
        } else {
            false
        }
    }

    /// Download leaderboard entries.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUserStats#DownloadLeaderboardEntries>
    pub fn download_entries(
        &self,
        handle: LeaderboardHandle,
        data_request: LeaderboardDataRequest,
        start: u32,
        end: u32,
    ) -> Vec<LeaderboardEntry> {
        if let Some(lb) = self.get_leaderboard(handle) {
            lb.get_entries(start, end - start + 1)
                .into_iter()
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get leaderboard entry count.
    pub fn get_entry_count(&self, handle: LeaderboardHandle) -> u32 {
        self.get_leaderboard(handle)
            .map(|l| l.entry_count())
            .unwrap_or(0)
    }

    /// Request current user stats.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUserStats#RequestCurrentStats>
    pub fn request_current_stats(&mut self) -> bool {
        // In real implementation, this would be async.
        self.user_stats.set_loaded(true);
        true
    }

    /// Get user stats reference.
    pub fn user_stats(&self) -> &UserStats {
        &self.user_stats
    }

    /// Get mutable user stats.
    pub fn user_stats_mut(&mut self) -> &mut UserStats {
        &mut self.user_stats
    }

    /// Store stats.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUserStats#StoreStats>
    pub fn store_stats(&mut self) -> bool {
        if self.user_stats.is_dirty() {
            self.user_stats.clear_dirty();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_steam_id(n: u32) -> SteamId {
        SteamId::from_account_id(n)
    }

    // =============================================================================
    // LDB-001: Find Leaderboard
    // Reference: https://partner.steamgames.com/doc/api/ISteamUserStats#FindLeaderboard
    // =============================================================================

    #[test]
    fn ldb_001_find_leaderboard() {
        let mut manager = LeaderboardManager::new();

        manager.find_or_create_leaderboard(
            "High Scores",
            LeaderboardSortMethod::Descending,
            LeaderboardDisplayType::Numeric,
        );

        let handle = manager.find_leaderboard("High Scores");
        assert!(handle.is_some());
        assert!(handle.unwrap().is_valid());
    }

    #[test]
    fn ldb_001_find_nonexistent() {
        let manager = LeaderboardManager::new();

        let handle = manager.find_leaderboard("Missing");
        assert!(handle.is_none());
    }

    // =============================================================================
    // LDB-002: Find or Create Leaderboard
    // Reference: https://partner.steamgames.com/doc/api/ISteamUserStats#FindOrCreateLeaderboard
    // =============================================================================

    #[test]
    fn ldb_002_find_or_create() {
        let mut manager = LeaderboardManager::new();

        let handle1 = manager.find_or_create_leaderboard(
            "Speedrun",
            LeaderboardSortMethod::Ascending,
            LeaderboardDisplayType::TimeMilliSeconds,
        );

        let handle2 = manager.find_or_create_leaderboard(
            "Speedrun",
            LeaderboardSortMethod::Ascending,
            LeaderboardDisplayType::TimeMilliSeconds,
        );

        // Same leaderboard should return same handle
        assert_eq!(handle1, handle2);
    }

    // =============================================================================
    // LDB-003: Upload Score
    // Reference: https://partner.steamgames.com/doc/api/ISteamUserStats#UploadLeaderboardScore
    // =============================================================================

    #[test]
    fn ldb_003_upload_score() {
        let mut manager = LeaderboardManager::new();

        let handle = manager.find_or_create_leaderboard(
            "High Scores",
            LeaderboardSortMethod::Descending,
            LeaderboardDisplayType::Numeric,
        );

        let player = test_steam_id(12345);
        let result = manager.upload_score(handle, player, 1000, LeaderboardUploadScoreMethod::KeepBest);

        assert!(result);
        assert_eq!(manager.get_entry_count(handle), 1);
    }

    // =============================================================================
    // LDB-004: Download Entries
    // Reference: https://partner.steamgames.com/doc/api/ISteamUserStats#DownloadLeaderboardEntries
    // =============================================================================

    #[test]
    fn ldb_004_download_entries() {
        let mut manager = LeaderboardManager::new();

        let handle = manager.find_or_create_leaderboard(
            "High Scores",
            LeaderboardSortMethod::Descending,
            LeaderboardDisplayType::Numeric,
        );

        // Add some scores
        manager.upload_score(handle, test_steam_id(1), 100, LeaderboardUploadScoreMethod::ForceUpdate);
        manager.upload_score(handle, test_steam_id(2), 200, LeaderboardUploadScoreMethod::ForceUpdate);
        manager.upload_score(handle, test_steam_id(3), 150, LeaderboardUploadScoreMethod::ForceUpdate);

        let entries = manager.download_entries(handle, LeaderboardDataRequest::Global, 1, 10);

        assert_eq!(entries.len(), 3);
        // Should be sorted descending
        assert_eq!(entries[0].score, 200);
        assert_eq!(entries[1].score, 150);
        assert_eq!(entries[2].score, 100);
    }

    // =============================================================================
    // LDB-007: Leaderboard Entry Count
    // Reference: https://partner.steamgames.com/doc/api/ISteamUserStats#GetLeaderboardEntryCount
    // =============================================================================

    #[test]
    fn ldb_007_entry_count() {
        let mut manager = LeaderboardManager::new();

        let handle = manager.find_or_create_leaderboard(
            "Test",
            LeaderboardSortMethod::Descending,
            LeaderboardDisplayType::Numeric,
        );

        assert_eq!(manager.get_entry_count(handle), 0);

        for i in 0..5 {
            manager.upload_score(handle, test_steam_id(i), i as i32 * 100, LeaderboardUploadScoreMethod::ForceUpdate);
        }

        assert_eq!(manager.get_entry_count(handle), 5);
    }

    // =============================================================================
    // LDB-008: Sort Method
    // =============================================================================

    #[test]
    fn ldb_008_ascending_sort() {
        let mut manager = LeaderboardManager::new();

        let handle = manager.find_or_create_leaderboard(
            "Speedrun",
            LeaderboardSortMethod::Ascending,
            LeaderboardDisplayType::TimeMilliSeconds,
        );

        manager.upload_score(handle, test_steam_id(1), 30000, LeaderboardUploadScoreMethod::ForceUpdate);
        manager.upload_score(handle, test_steam_id(2), 25000, LeaderboardUploadScoreMethod::ForceUpdate);
        manager.upload_score(handle, test_steam_id(3), 35000, LeaderboardUploadScoreMethod::ForceUpdate);

        let entries = manager.download_entries(handle, LeaderboardDataRequest::Global, 1, 10);

        // Lower is better for ascending
        assert_eq!(entries[0].score, 25000);
        assert_eq!(entries[0].global_rank, 1);
    }

    // =============================================================================
    // LDB-010: Keep Best
    // =============================================================================

    #[test]
    fn ldb_010_keep_best_descending() {
        let mut manager = LeaderboardManager::new();

        let handle = manager.find_or_create_leaderboard(
            "High Scores",
            LeaderboardSortMethod::Descending,
            LeaderboardDisplayType::Numeric,
        );

        let player = test_steam_id(1);

        manager.upload_score(handle, player, 100, LeaderboardUploadScoreMethod::KeepBest);
        manager.upload_score(handle, player, 50, LeaderboardUploadScoreMethod::KeepBest); // Worse
        manager.upload_score(handle, player, 150, LeaderboardUploadScoreMethod::KeepBest); // Better

        let lb = manager.get_leaderboard(handle).unwrap();
        let entry = lb.get_user_entry(player).unwrap();
        assert_eq!(entry.score, 150);
    }

    #[test]
    fn ldb_010_keep_best_ascending() {
        let mut manager = LeaderboardManager::new();

        let handle = manager.find_or_create_leaderboard(
            "Speedrun",
            LeaderboardSortMethod::Ascending,
            LeaderboardDisplayType::TimeMilliSeconds,
        );

        let player = test_steam_id(1);

        manager.upload_score(handle, player, 30000, LeaderboardUploadScoreMethod::KeepBest);
        manager.upload_score(handle, player, 35000, LeaderboardUploadScoreMethod::KeepBest); // Worse
        manager.upload_score(handle, player, 25000, LeaderboardUploadScoreMethod::KeepBest); // Better

        let lb = manager.get_leaderboard(handle).unwrap();
        let entry = lb.get_user_entry(player).unwrap();
        assert_eq!(entry.score, 25000);
    }

    // =============================================================================
    // STAT-001: Request Stats
    // Reference: https://partner.steamgames.com/doc/api/ISteamUserStats#RequestCurrentStats
    // =============================================================================

    #[test]
    fn stat_001_request_stats() {
        let mut manager = LeaderboardManager::new();

        assert!(!manager.user_stats().is_loaded());

        let result = manager.request_current_stats();
        assert!(result);
        assert!(manager.user_stats().is_loaded());
    }

    // =============================================================================
    // STAT-002: Get Int Stat
    // Reference: https://partner.steamgames.com/doc/api/ISteamUserStats#GetStat
    // =============================================================================

    #[test]
    fn stat_002_get_int_stat() {
        let mut manager = LeaderboardManager::new();
        manager.request_current_stats();

        manager.user_stats_mut().set_stat_int("kills", 100);

        assert_eq!(manager.user_stats().get_stat_int("kills"), Some(100));
    }

    // =============================================================================
    // STAT-003: Get Float Stat
    // =============================================================================

    #[test]
    fn stat_003_get_float_stat() {
        let mut manager = LeaderboardManager::new();
        manager.request_current_stats();

        manager.user_stats_mut().set_stat_float("accuracy", 0.75);

        let stat = manager.user_stats().get_stat_float("accuracy").unwrap();
        assert!((stat - 0.75).abs() < 0.001);
    }

    // =============================================================================
    // STAT-004: Set Stat
    // =============================================================================

    #[test]
    fn stat_004_set_stat() {
        let mut manager = LeaderboardManager::new();
        manager.request_current_stats();

        manager.user_stats_mut().set_stat_int("deaths", 50);

        assert_eq!(manager.user_stats().get_stat_int("deaths"), Some(50));
        assert!(manager.user_stats().is_dirty());
    }

    // =============================================================================
    // STAT-005: Store Stats
    // Reference: https://partner.steamgames.com/doc/api/ISteamUserStats#StoreStats
    // =============================================================================

    #[test]
    fn stat_005_store_stats() {
        let mut manager = LeaderboardManager::new();
        manager.request_current_stats();

        manager.user_stats_mut().set_stat_int("score", 1000);
        assert!(manager.user_stats().is_dirty());

        let result = manager.store_stats();
        assert!(result);
        assert!(!manager.user_stats().is_dirty());
    }

    // =============================================================================
    // STAT-006: Reset Stats
    // Reference: https://partner.steamgames.com/doc/api/ISteamUserStats#ResetAllStats
    // =============================================================================

    #[test]
    fn stat_006_reset_stats() {
        let mut manager = LeaderboardManager::new();
        manager.request_current_stats();

        manager.user_stats_mut().set_stat_int("kills", 100);
        manager.user_stats_mut().set_stat_int("deaths", 50);

        manager.user_stats_mut().reset_all();

        assert!(manager.user_stats().get_stat_int("kills").is_none());
        assert!(manager.user_stats().get_stat_int("deaths").is_none());
    }

    // =============================================================================
    // Additional Tests
    // =============================================================================

    #[test]
    fn entries_around_user() {
        let mut manager = LeaderboardManager::new();

        let handle = manager.find_or_create_leaderboard(
            "High Scores",
            LeaderboardSortMethod::Descending,
            LeaderboardDisplayType::Numeric,
        );

        for i in 1..=10 {
            manager.upload_score(handle, test_steam_id(i), i as i32 * 100, LeaderboardUploadScoreMethod::ForceUpdate);
        }

        let lb = manager.get_leaderboard(handle).unwrap();
        let entries = lb.get_entries_around_user(test_steam_id(5), 2, 2);

        assert!(entries.len() >= 3);
    }

    #[test]
    fn stat_conversion() {
        let int_val = StatValue::Int(42);
        assert_eq!(int_val.as_int(), Some(42));
        assert_eq!(int_val.as_float(), Some(42.0));

        let float_val = StatValue::Float(3.14);
        assert_eq!(float_val.as_int(), Some(3));
        assert!((float_val.as_float().unwrap() - 3.14).abs() < 0.001);
    }
}
