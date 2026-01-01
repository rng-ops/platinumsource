//! Lobby system implementation.
//!
//! # Valve Documentation Reference
//! - [Steam Matchmaking & Lobbies](https://partner.steamgames.com/doc/features/multiplayer/matchmaking)
//! - [ISteamMatchmaking Interface](https://partner.steamgames.com/doc/api/ISteamMatchmaking)
//!
//! # Lobby Types
//! - **Private**: Only visible via invite
//! - **FriendsOnly**: Visible to friends of members
//! - **Public**: Visible in lobby list
//! - **Invisible**: Not returned in search, join by ID only
//!
//! # Lobby Lifecycle
//! 1. Owner creates lobby with CreateLobby()
//! 2. Other players join via JoinLobby() or invite
//! 3. Owner sets lobby data (map, game mode, etc.)
//! 4. When ready, owner sets game server info
//! 5. All members connect to game server
//! 6. Lobby persists until empty

use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::steam_id::SteamId;

/// Lobby type/visibility.
/// 
/// Reference: <https://partner.steamgames.com/doc/api/ISteamMatchmaking#ELobbyType>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LobbyType {
    /// Only joinable via invite.
    Private = 0,
    /// Joinable by friends of lobby members.
    FriendsOnly = 1,
    /// Visible in public lobby list.
    Public = 2,
    /// Not returned in searches, joinable by ID.
    Invisible = 3,
}

impl LobbyType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(LobbyType::Private),
            1 => Some(LobbyType::FriendsOnly),
            2 => Some(LobbyType::Public),
            3 => Some(LobbyType::Invisible),
            _ => None,
        }
    }

    /// Check if lobby appears in public searches.
    pub fn is_searchable(&self) -> bool {
        matches!(self, LobbyType::Public | LobbyType::FriendsOnly)
    }
}

/// Lobby comparison types for filtering.
/// 
/// Reference: <https://partner.steamgames.com/doc/api/ISteamMatchmaking#ELobbyComparison>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyComparison {
    EqualToOrLessThan = -2,
    LessThan = -1,
    Equal = 0,
    GreaterThan = 1,
    EqualToOrGreaterThan = 2,
    NotEqual = 3,
}

/// Lobby distance filter.
/// 
/// Reference: <https://partner.steamgames.com/doc/api/ISteamMatchmaking#ELobbyDistanceFilter>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LobbyDistanceFilter {
    Close = 0,
    #[default]
    Default = 1,
    Far = 2,
    Worldwide = 3,
}

/// Unique lobby identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LobbyId(u64);

impl LobbyId {
    pub const INVALID: LobbyId = LobbyId(0);

    pub fn new(id: u64) -> Self {
        LobbyId(id)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// A lobby member's data.
#[derive(Debug, Clone)]
pub struct LobbyMember {
    pub steam_id: SteamId,
    pub joined_at: Instant,
    /// Per-member metadata.
    pub data: HashMap<String, String>,
}

impl LobbyMember {
    pub fn new(steam_id: SteamId) -> Self {
        LobbyMember {
            steam_id,
            joined_at: Instant::now(),
            data: HashMap::new(),
        }
    }
}

/// Game server information attached to a lobby.
/// 
/// Reference: <https://partner.steamgames.com/doc/api/ISteamMatchmaking#SetLobbyGameServer>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyGameServer {
    /// Server IP address.
    pub ip: u32,
    /// Server port.
    pub port: u16,
    /// Steam ID of game server (if dedicated).
    pub server_id: Option<SteamId>,
}

/// A game lobby.
#[derive(Debug, Clone)]
pub struct Lobby {
    pub id: LobbyId,
    pub lobby_type: LobbyType,
    pub owner: SteamId,
    pub members: Vec<LobbyMember>,
    pub max_members: u32,
    pub created_at: Instant,
    /// Lobby-wide metadata (map, mode, etc.).
    pub data: HashMap<String, String>,
    /// Associated game server, if any.
    pub game_server: Option<LobbyGameServer>,
    /// Whether the lobby is locked.
    pub joinable: bool,
}

/// Limits for lobby data.
/// Reference: <https://partner.steamgames.com/doc/api/ISteamMatchmaking#SetLobbyData>
pub const MAX_LOBBY_KEY_LENGTH: usize = 255;
pub const MAX_LOBBY_VALUE_LENGTH: usize = 8192; // 8KB
pub const MAX_LOBBY_DATA_ENTRIES: usize = 256;

impl Lobby {
    /// Create a new lobby.
    pub fn new(id: LobbyId, owner: SteamId, lobby_type: LobbyType, max_members: u32) -> Self {
        let owner_member = LobbyMember::new(owner);
        Lobby {
            id,
            lobby_type,
            owner,
            members: vec![owner_member],
            max_members: max_members.max(1),
            created_at: Instant::now(),
            data: HashMap::new(),
            game_server: None,
            joinable: true,
        }
    }

    /// Get current member count.
    pub fn member_count(&self) -> u32 {
        self.members.len() as u32
    }

    /// Get available slots.
    pub fn available_slots(&self) -> u32 {
        self.max_members.saturating_sub(self.member_count())
    }

    /// Check if lobby is full.
    pub fn is_full(&self) -> bool {
        self.member_count() >= self.max_members
    }

    /// Check if a player is a member.
    pub fn is_member(&self, steam_id: SteamId) -> bool {
        self.members.iter().any(|m| m.steam_id == steam_id)
    }

    /// Check if a player is the owner.
    pub fn is_owner(&self, steam_id: SteamId) -> bool {
        self.owner == steam_id
    }

    /// Add a member to the lobby.
    pub fn add_member(&mut self, steam_id: SteamId) -> Result<(), LobbyError> {
        if !self.joinable {
            return Err(LobbyError::LobbyNotJoinable);
        }
        if self.is_full() {
            return Err(LobbyError::LobbyFull);
        }
        if self.is_member(steam_id) {
            return Err(LobbyError::AlreadyMember);
        }
        self.members.push(LobbyMember::new(steam_id));
        Ok(())
    }

    /// Remove a member from the lobby.
    pub fn remove_member(&mut self, steam_id: SteamId) -> Result<(), LobbyError> {
        let initial_len = self.members.len();
        self.members.retain(|m| m.steam_id != steam_id);
        
        if self.members.len() == initial_len {
            return Err(LobbyError::NotMember);
        }

        // Transfer ownership if owner left
        if self.owner == steam_id && !self.members.is_empty() {
            self.owner = self.members[0].steam_id;
        }

        Ok(())
    }

    /// Set lobby data.
    pub fn set_data(&mut self, key: &str, value: &str) -> Result<(), LobbyError> {
        if key.len() > MAX_LOBBY_KEY_LENGTH {
            return Err(LobbyError::KeyTooLong);
        }
        if value.len() > MAX_LOBBY_VALUE_LENGTH {
            return Err(LobbyError::ValueTooLong);
        }
        if self.data.len() >= MAX_LOBBY_DATA_ENTRIES && !self.data.contains_key(key) {
            return Err(LobbyError::TooManyEntries);
        }
        self.data.insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// Get lobby data.
    pub fn get_data(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Set member data.
    pub fn set_member_data(&mut self, steam_id: SteamId, key: &str, value: &str) -> Result<(), LobbyError> {
        if key.len() > MAX_LOBBY_KEY_LENGTH {
            return Err(LobbyError::KeyTooLong);
        }
        if value.len() > MAX_LOBBY_VALUE_LENGTH {
            return Err(LobbyError::ValueTooLong);
        }
        
        let member = self.members
            .iter_mut()
            .find(|m| m.steam_id == steam_id)
            .ok_or(LobbyError::NotMember)?;
        
        member.data.insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// Get member data.
    pub fn get_member_data(&self, steam_id: SteamId, key: &str) -> Option<&str> {
        self.members
            .iter()
            .find(|m| m.steam_id == steam_id)
            .and_then(|m| m.data.get(key).map(|s| s.as_str()))
    }

    /// Set game server.
    pub fn set_game_server(&mut self, ip: u32, port: u16, server_id: Option<SteamId>) {
        self.game_server = Some(LobbyGameServer { ip, port, server_id });
    }

    /// Set member limit.
    pub fn set_member_limit(&mut self, limit: u32) -> Result<(), LobbyError> {
        if limit < self.member_count() {
            return Err(LobbyError::LimitTooLow);
        }
        self.max_members = limit;
        Ok(())
    }

    /// Transfer ownership.
    pub fn set_owner(&mut self, new_owner: SteamId) -> Result<(), LobbyError> {
        if !self.is_member(new_owner) {
            return Err(LobbyError::NotMember);
        }
        self.owner = new_owner;
        Ok(())
    }
}

/// Lobby operation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LobbyError {
    LobbyFull,
    LobbyNotJoinable,
    NotMember,
    AlreadyMember,
    NotOwner,
    KeyTooLong,
    ValueTooLong,
    TooManyEntries,
    LimitTooLow,
    InvalidLobby,
}

/// Lobby search filter.
#[derive(Debug, Clone, Default)]
pub struct LobbySearchFilter {
    pub string_filters: Vec<(String, String, LobbyComparison)>,
    pub numeric_filters: Vec<(String, i32, LobbyComparison)>,
    pub slots_available: Option<u32>,
    pub distance: LobbyDistanceFilter,
    pub max_results: Option<u32>,
}

impl LobbySearchFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a string filter.
    pub fn with_string_filter(mut self, key: &str, value: &str, comparison: LobbyComparison) -> Self {
        self.string_filters.push((key.to_string(), value.to_string(), comparison));
        self
    }

    /// Add a numeric filter.
    pub fn with_numeric_filter(mut self, key: &str, value: i32, comparison: LobbyComparison) -> Self {
        self.numeric_filters.push((key.to_string(), value, comparison));
        self
    }

    /// Filter by available slots.
    pub fn with_slots_available(mut self, slots: u32) -> Self {
        self.slots_available = Some(slots);
        self
    }

    /// Set distance filter.
    pub fn with_distance(mut self, distance: LobbyDistanceFilter) -> Self {
        self.distance = distance;
        self
    }

    /// Set max results.
    pub fn with_max_results(mut self, max: u32) -> Self {
        self.max_results = Some(max);
        self
    }

    /// Check if a lobby matches this filter.
    pub fn matches(&self, lobby: &Lobby) -> bool {
        // Check slots
        if let Some(slots) = self.slots_available {
            if lobby.available_slots() < slots {
                return false;
            }
        }

        // Check string filters
        for (key, expected, comparison) in &self.string_filters {
            let actual = lobby.get_data(key).unwrap_or("");
            let matches = match comparison {
                LobbyComparison::Equal => actual == expected,
                LobbyComparison::NotEqual => actual != expected,
                _ => actual == expected, // String comparison defaults to equal
            };
            if !matches {
                return false;
            }
        }

        // Check numeric filters
        for (key, expected, comparison) in &self.numeric_filters {
            let actual: i32 = lobby
                .get_data(key)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            
            let matches = match comparison {
                LobbyComparison::Equal => actual == *expected,
                LobbyComparison::NotEqual => actual != *expected,
                LobbyComparison::LessThan => actual < *expected,
                LobbyComparison::GreaterThan => actual > *expected,
                LobbyComparison::EqualToOrLessThan => actual <= *expected,
                LobbyComparison::EqualToOrGreaterThan => actual >= *expected,
            };
            if !matches {
                return false;
            }
        }

        true
    }
}

/// Lobby manager for tracking multiple lobbies.
#[derive(Default)]
pub struct LobbyManager {
    lobbies: HashMap<LobbyId, Lobby>,
    next_id: u64,
}

impl LobbyManager {
    pub fn new() -> Self {
        LobbyManager {
            lobbies: HashMap::new(),
            next_id: 1,
        }
    }

    /// Create a new lobby.
    pub fn create_lobby(
        &mut self,
        owner: SteamId,
        lobby_type: LobbyType,
        max_members: u32,
    ) -> LobbyId {
        let id = LobbyId::new(self.next_id);
        self.next_id += 1;

        let lobby = Lobby::new(id, owner, lobby_type, max_members);
        self.lobbies.insert(id, lobby);
        id
    }

    /// Get a lobby by ID.
    pub fn get_lobby(&self, id: LobbyId) -> Option<&Lobby> {
        self.lobbies.get(&id)
    }

    /// Get a mutable lobby by ID.
    pub fn get_lobby_mut(&mut self, id: LobbyId) -> Option<&mut Lobby> {
        self.lobbies.get_mut(&id)
    }

    /// Remove a lobby.
    pub fn remove_lobby(&mut self, id: LobbyId) -> Option<Lobby> {
        self.lobbies.remove(&id)
    }

    /// Search for lobbies matching a filter.
    pub fn search(&self, filter: &LobbySearchFilter) -> Vec<&Lobby> {
        let mut results: Vec<_> = self.lobbies
            .values()
            .filter(|lobby| {
                lobby.lobby_type.is_searchable()
                    && lobby.joinable
                    && filter.matches(lobby)
            })
            .collect();

        // Apply max results
        if let Some(max) = filter.max_results {
            results.truncate(max as usize);
        }

        results
    }

    /// Clean up empty lobbies.
    pub fn cleanup_empty(&mut self) {
        self.lobbies.retain(|_, lobby| !lobby.members.is_empty());
    }

    /// Get all lobbies a player is in.
    pub fn get_player_lobbies(&self, steam_id: SteamId) -> Vec<LobbyId> {
        self.lobbies
            .iter()
            .filter(|(_, lobby)| lobby.is_member(steam_id))
            .map(|(id, _)| *id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_steam_id(n: u32) -> SteamId {
        SteamId::from_account_id(n)
    }

    // =============================================================================
    // LOB-001: Create Public Lobby
    // Reference: https://partner.steamgames.com/doc/api/ISteamMatchmaking#CreateLobby
    // =============================================================================

    #[test]
    fn lob_001_create_public_lobby() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        assert!(lobby_id.is_valid());
        let lobby = manager.get_lobby(lobby_id).unwrap();
        assert_eq!(lobby.lobby_type, LobbyType::Public);
        assert_eq!(lobby.owner, owner);
        assert_eq!(lobby.max_members, 8);
        assert_eq!(lobby.member_count(), 1); // Owner is first member
    }

    // =============================================================================
    // LOB-002: Create Private Lobby
    // =============================================================================

    #[test]
    fn lob_002_create_private_lobby() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Private, 4);
        
        let lobby = manager.get_lobby(lobby_id).unwrap();
        assert_eq!(lobby.lobby_type, LobbyType::Private);
        assert!(!lobby.lobby_type.is_searchable());
    }

    // =============================================================================
    // LOB-003: Create Friends-Only Lobby
    // =============================================================================

    #[test]
    fn lob_003_create_friends_only_lobby() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::FriendsOnly, 4);
        
        let lobby = manager.get_lobby(lobby_id).unwrap();
        assert_eq!(lobby.lobby_type, LobbyType::FriendsOnly);
        assert!(lobby.lobby_type.is_searchable());
    }

    // =============================================================================
    // LOB-004: Create Invisible Lobby
    // =============================================================================

    #[test]
    fn lob_004_create_invisible_lobby() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Invisible, 2);
        
        let lobby = manager.get_lobby(lobby_id).unwrap();
        assert_eq!(lobby.lobby_type, LobbyType::Invisible);
        assert!(!lobby.lobby_type.is_searchable());
    }

    // =============================================================================
    // LOB-005: Join Lobby by ID
    // Reference: https://partner.steamgames.com/doc/api/ISteamMatchmaking#JoinLobby
    // =============================================================================

    #[test]
    fn lob_005_join_lobby() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        let joiner = test_steam_id(67890);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.add_member(joiner).unwrap();
        
        assert_eq!(lobby.member_count(), 2);
        assert!(lobby.is_member(joiner));
    }

    #[test]
    fn lob_005_join_full_lobby_fails() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(1);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 2);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.add_member(test_steam_id(2)).unwrap();
        
        // Third member should fail
        let result = lobby.add_member(test_steam_id(3));
        assert_eq!(result, Err(LobbyError::LobbyFull));
    }

    // =============================================================================
    // LOB-006: Leave Lobby
    // Reference: https://partner.steamgames.com/doc/api/ISteamMatchmaking#LeaveLobby
    // =============================================================================

    #[test]
    fn lob_006_leave_lobby() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        let member = test_steam_id(67890);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.add_member(member).unwrap();
        lobby.remove_member(member).unwrap();
        
        assert_eq!(lobby.member_count(), 1);
        assert!(!lobby.is_member(member));
    }

    // =============================================================================
    // LOB-007: Lobby Member Limit
    // Reference: https://partner.steamgames.com/doc/api/ISteamMatchmaking#SetLobbyMemberLimit
    // =============================================================================

    #[test]
    fn lob_007_set_member_limit() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.set_member_limit(16).unwrap();
        
        assert_eq!(lobby.max_members, 16);
    }

    #[test]
    fn lob_007_limit_below_members_fails() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.add_member(test_steam_id(2)).unwrap();
        lobby.add_member(test_steam_id(3)).unwrap();
        
        // Can't set limit below current count
        let result = lobby.set_member_limit(2);
        assert_eq!(result, Err(LobbyError::LimitTooLow));
    }

    // =============================================================================
    // LOB-008: Lobby Owner Transfer
    // Reference: https://partner.steamgames.com/doc/api/ISteamMatchmaking#SetLobbyOwner
    // =============================================================================

    #[test]
    fn lob_008_owner_transfer_on_leave() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        let member = test_steam_id(67890);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.add_member(member).unwrap();
        lobby.remove_member(owner).unwrap();
        
        // New owner should be the remaining member
        assert_eq!(lobby.owner, member);
    }

    #[test]
    fn lob_008_explicit_owner_transfer() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        let member = test_steam_id(67890);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.add_member(member).unwrap();
        lobby.set_owner(member).unwrap();
        
        assert_eq!(lobby.owner, member);
    }

    // =============================================================================
    // LOB-009: Lobby Deletion When Empty
    // =============================================================================

    #[test]
    fn lob_009_cleanup_empty_lobbies() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        // Remove owner (last member)
        manager.get_lobby_mut(lobby_id).unwrap().remove_member(owner).unwrap();
        
        manager.cleanup_empty();
        
        assert!(manager.get_lobby(lobby_id).is_none());
    }

    // =============================================================================
    // LOB-010: Lobby Search
    // Reference: https://partner.steamgames.com/doc/api/ISteamMatchmaking#RequestLobbyList
    // =============================================================================

    #[test]
    fn lob_010_basic_search() {
        let mut manager = LobbyManager::new();
        
        // Create some lobbies
        manager.create_lobby(test_steam_id(1), LobbyType::Public, 8);
        manager.create_lobby(test_steam_id(2), LobbyType::Private, 4);
        manager.create_lobby(test_steam_id(3), LobbyType::Public, 8);
        
        let filter = LobbySearchFilter::new();
        let results = manager.search(&filter);
        
        // Should only return public lobbies
        assert_eq!(results.len(), 2);
    }

    // =============================================================================
    // LOB-DATA-001: Set Lobby Data
    // Reference: https://partner.steamgames.com/doc/api/ISteamMatchmaking#SetLobbyData
    // =============================================================================

    #[test]
    fn lob_data_001_set_lobby_data() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.set_data("map", "de_dust2").unwrap();
        lobby.set_data("gamemode", "competitive").unwrap();
        
        assert_eq!(lobby.get_data("map"), Some("de_dust2"));
        assert_eq!(lobby.get_data("gamemode"), Some("competitive"));
    }

    // =============================================================================
    // LOB-DATA-003: Lobby Data Limits
    // =============================================================================

    #[test]
    fn lob_data_003_key_length_limit() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        let long_key = "k".repeat(MAX_LOBBY_KEY_LENGTH + 1);
        
        let result = lobby.set_data(&long_key, "value");
        assert_eq!(result, Err(LobbyError::KeyTooLong));
    }

    #[test]
    fn lob_data_003_value_length_limit() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        let long_value = "v".repeat(MAX_LOBBY_VALUE_LENGTH + 1);
        
        let result = lobby.set_data("key", &long_value);
        assert_eq!(result, Err(LobbyError::ValueTooLong));
    }

    // =============================================================================
    // LOB-DATA-004: Member Data
    // =============================================================================

    #[test]
    fn lob_data_004_member_data() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.set_member_data(owner, "ready", "true").unwrap();
        
        assert_eq!(lobby.get_member_data(owner, "ready"), Some("true"));
    }

    // =============================================================================
    // LOB-DATA-006: Game Server Info
    // Reference: https://partner.steamgames.com/doc/api/ISteamMatchmaking#SetLobbyGameServer
    // =============================================================================

    #[test]
    fn lob_data_006_game_server_info() {
        let mut manager = LobbyManager::new();
        let owner = test_steam_id(12345);
        
        let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
        
        let lobby = manager.get_lobby_mut(lobby_id).unwrap();
        lobby.set_game_server(0x7F000001, 27015, None); // 127.0.0.1:27015
        
        let gs = lobby.game_server.as_ref().unwrap();
        assert_eq!(gs.ip, 0x7F000001);
        assert_eq!(gs.port, 27015);
    }

    // =============================================================================
    // Filter Tests
    // =============================================================================

    #[test]
    fn filter_by_slots_available() {
        let mut manager = LobbyManager::new();
        
        // Create a lobby with 8 slots, 1 member
        let lobby_id = manager.create_lobby(test_steam_id(1), LobbyType::Public, 8);
        
        // Search for lobbies with at least 5 slots
        let filter = LobbySearchFilter::new().with_slots_available(5);
        let results = manager.search(&filter);
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].available_slots(), 7);
    }

    #[test]
    fn filter_by_string_data() {
        let mut manager = LobbyManager::new();
        
        let lobby_id1 = manager.create_lobby(test_steam_id(1), LobbyType::Public, 8);
        let lobby_id2 = manager.create_lobby(test_steam_id(2), LobbyType::Public, 8);
        
        manager.get_lobby_mut(lobby_id1).unwrap().set_data("map", "de_dust2").unwrap();
        manager.get_lobby_mut(lobby_id2).unwrap().set_data("map", "cs_office").unwrap();
        
        let filter = LobbySearchFilter::new()
            .with_string_filter("map", "de_dust2", LobbyComparison::Equal);
        
        let results = manager.search(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get_data("map"), Some("de_dust2"));
    }

    #[test]
    fn filter_by_numeric_data() {
        let mut manager = LobbyManager::new();
        
        let lobby_id1 = manager.create_lobby(test_steam_id(1), LobbyType::Public, 8);
        let lobby_id2 = manager.create_lobby(test_steam_id(2), LobbyType::Public, 8);
        
        manager.get_lobby_mut(lobby_id1).unwrap().set_data("skill", "1000").unwrap();
        manager.get_lobby_mut(lobby_id2).unwrap().set_data("skill", "2000").unwrap();
        
        let filter = LobbySearchFilter::new()
            .with_numeric_filter("skill", 1500, LobbyComparison::GreaterThan);
        
        let results = manager.search(&filter);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn filter_max_results() {
        let mut manager = LobbyManager::new();
        
        for i in 0..10 {
            manager.create_lobby(test_steam_id(i), LobbyType::Public, 8);
        }
        
        let filter = LobbySearchFilter::new().with_max_results(3);
        let results = manager.search(&filter);
        
        assert_eq!(results.len(), 3);
    }
}
