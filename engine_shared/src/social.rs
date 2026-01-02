//! Steam Social/Friends system implementation.
//!
//! # Valve Documentation Reference
//! - [ISteamFriends](https://partner.steamgames.com/doc/api/ISteamFriends)
//! - [Friends API](https://partner.steamgames.com/doc/features/friends)
//!
//! # Features
//! - Friends list management
//! - Friend relationship types
//! - Persona states and rich presence
//! - Clan/group integration
//! - Game invites

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Friend relationship types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FriendRelationship {
    /// Not friends.
    #[default]
    None,
    /// User has blocked this person.
    Blocked,
    /// User has received a friend request.
    RequestRecipient,
    /// Confirmed friends.
    Friend,
    /// User has sent a friend request.
    RequestInitiator,
    /// User has ignored this person.
    Ignored,
    /// Friend that is ignored.
    IgnoredFriend,
}

/// Persona state (online status).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PersonaState {
    /// User is not currently logged in.
    #[default]
    Offline,
    /// User is logged in.
    Online,
    /// User is busy.
    Busy,
    /// User is away.
    Away,
    /// User is snooze/idle.
    Snooze,
    /// User is looking to trade.
    LookingToTrade,
    /// User is looking to play.
    LookingToPlay,
    /// User appears offline.
    Invisible,
}

impl PersonaState {
    /// Check if user is considered online.
    pub fn is_online(&self) -> bool {
        !matches!(self, PersonaState::Offline | PersonaState::Invisible)
    }
}

/// Friend flags for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FriendFlags(pub u16);

impl FriendFlags {
    /// No filter.
    pub const NONE: u16 = 0x00;
    /// Filter by blocked users.
    pub const BLOCKED: u16 = 0x01;
    /// Filter by request recipients.
    pub const FRIENDSHIP_REQUESTED: u16 = 0x02;
    /// Filter by immediate friends.
    pub const IMMEDIATE: u16 = 0x04;
    /// Filter by clan members.
    pub const CLAN_MEMBER: u16 = 0x08;
    /// Filter by users on game server.
    pub const ON_GAME_SERVER: u16 = 0x10;
    /// Filter by request initiators.
    pub const REQUEST_INITIATOR: u16 = 0x40;
    /// Filter by ignored.
    pub const IGNORED: u16 = 0x80;
    /// Filter by ignored friends.
    pub const IGNORED_FRIEND: u16 = 0x100;
    /// All flags.
    pub const ALL: u16 = 0xFFFF;
}

/// Game info for a friend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendGameInfo {
    /// Game App ID (0 if not in game).
    pub app_id: u32,
    /// Game server IP.
    pub game_ip: u32,
    /// Game server port.
    pub game_port: u16,
    /// Game server query port.
    pub query_port: u16,
    /// Steam ID of lobby if in one.
    pub lobby_id: u64,
}

impl Default for FriendGameInfo {
    fn default() -> Self {
        Self {
            app_id: 0,
            game_ip: 0,
            game_port: 0,
            query_port: 0,
            lobby_id: 0,
        }
    }
}

/// Friend data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    /// Steam ID.
    pub steam_id: u64,
    /// Persona name.
    pub persona_name: String,
    /// Current state.
    pub persona_state: PersonaState,
    /// Relationship to local user.
    pub relationship: FriendRelationship,
    /// Game info if in game.
    pub game_info: FriendGameInfo,
    /// Rich presence data.
    pub rich_presence: HashMap<String, String>,
}

impl Friend {
    /// Create a new friend.
    pub fn new(steam_id: u64, name: &str) -> Self {
        Self {
            steam_id,
            persona_name: name.to_string(),
            persona_state: PersonaState::Offline,
            relationship: FriendRelationship::Friend,
            game_info: FriendGameInfo::default(),
            rich_presence: HashMap::new(),
        }
    }

    /// Check if friend is in game.
    pub fn is_in_game(&self) -> bool {
        self.game_info.app_id != 0
    }

    /// Check if friend is playing the same game.
    pub fn is_playing_game(&self, app_id: u32) -> bool {
        self.game_info.app_id == app_id
    }
}

/// Steam Clan/Group data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clan {
    /// Clan Steam ID.
    pub clan_id: u64,
    /// Clan name.
    pub name: String,
    /// Clan tag.
    pub tag: String,
    /// Officer count.
    pub officer_count: usize,
    /// Online member count.
    pub online_count: u32,
    /// In-game member count.
    pub in_game_count: u32,
    /// Chatting member count.
    pub chatting_count: u32,
}

impl Clan {
    /// Create a new clan.
    pub fn new(clan_id: u64, name: &str, tag: &str) -> Self {
        Self {
            clan_id,
            name: name.to_string(),
            tag: tag.to_string(),
            officer_count: 0,
            online_count: 0,
            in_game_count: 0,
            chatting_count: 0,
        }
    }
}

/// Recently played with data.
#[derive(Debug, Clone)]
pub struct CoplayFriend {
    /// Steam ID.
    pub steam_id: u64,
    /// Time of last coplay.
    pub time: u64,
    /// App ID where coplay occurred.
    pub app_id: u32,
}

/// Game invite result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteResult {
    /// Invite sent successfully.
    Ok,
    /// User is not a friend.
    NotFriend,
    /// User is blocked.
    Blocked,
    /// User is offline.
    Offline,
    /// Rate limited.
    RateLimited,
    /// Failed to send.
    Failed,
}

/// Mock Friends manager for testing.
///
/// In production, this would interface with Steamworks SDK.
pub struct FriendsManager {
    /// Local user Steam ID.
    local_user: u64,
    /// Friends list.
    friends: HashMap<u64, Friend>,
    /// Clans.
    clans: Vec<Clan>,
    /// Coplay friends (recently played with).
    coplay: Vec<CoplayFriend>,
    /// Invite cooldown tracking.
    invite_timestamps: HashMap<u64, u64>,
    /// Current app ID.
    app_id: u32,
}

impl FriendsManager {
    /// Create a new friends manager.
    pub fn new(local_user: u64, app_id: u32) -> Self {
        Self {
            local_user,
            friends: HashMap::new(),
            clans: Vec::new(),
            coplay: Vec::new(),
            invite_timestamps: HashMap::new(),
            app_id,
        }
    }

    /// Get friend count with optional filter.
    pub fn get_friend_count(&self, flags: u16) -> usize {
        if flags == FriendFlags::NONE || flags == FriendFlags::ALL {
            return self.friends.len();
        }

        self.friends
            .values()
            .filter(|f| self.matches_flags(f, flags))
            .count()
    }

    /// Check if friend matches flags.
    fn matches_flags(&self, friend: &Friend, flags: u16) -> bool {
        if flags & FriendFlags::IMMEDIATE != 0 && friend.relationship == FriendRelationship::Friend {
            return true;
        }
        if flags & FriendFlags::BLOCKED != 0 && friend.relationship == FriendRelationship::Blocked {
            return true;
        }
        if flags & FriendFlags::FRIENDSHIP_REQUESTED != 0
            && friend.relationship == FriendRelationship::RequestRecipient
        {
            return true;
        }
        if flags & FriendFlags::REQUEST_INITIATOR != 0
            && friend.relationship == FriendRelationship::RequestInitiator
        {
            return true;
        }
        if flags & FriendFlags::IGNORED != 0 && friend.relationship == FriendRelationship::Ignored {
            return true;
        }
        false
    }

    /// Get friend by index.
    pub fn get_friend_by_index(&self, index: usize, _flags: u16) -> Option<u64> {
        self.friends.keys().nth(index).copied()
    }

    /// Get friend relationship.
    pub fn get_friend_relationship(&self, steam_id: u64) -> FriendRelationship {
        self.friends
            .get(&steam_id)
            .map(|f| f.relationship)
            .unwrap_or(FriendRelationship::None)
    }

    /// Get friend persona name.
    pub fn get_friend_persona_name(&self, steam_id: u64) -> Option<&str> {
        self.friends.get(&steam_id).map(|f| f.persona_name.as_str())
    }

    /// Get friend persona state.
    pub fn get_friend_persona_state(&self, steam_id: u64) -> PersonaState {
        self.friends
            .get(&steam_id)
            .map(|f| f.persona_state)
            .unwrap_or(PersonaState::Offline)
    }

    /// Get friend game info.
    pub fn get_friend_game_played(&self, steam_id: u64) -> Option<&FriendGameInfo> {
        self.friends.get(&steam_id).map(|f| &f.game_info)
    }

    /// Get friend rich presence.
    pub fn get_friend_rich_presence(&self, steam_id: u64, key: &str) -> Option<&str> {
        self.friends
            .get(&steam_id)
            .and_then(|f| f.rich_presence.get(key).map(|s| s.as_str()))
    }

    /// Invite friend to game.
    pub fn invite_user_to_game(&mut self, steam_id: u64, connect_string: &str) -> InviteResult {
        let _ = connect_string;

        let friend = match self.friends.get(&steam_id) {
            Some(f) => f,
            None => return InviteResult::NotFriend,
        };

        if friend.relationship == FriendRelationship::Blocked {
            return InviteResult::Blocked;
        }

        if friend.relationship != FriendRelationship::Friend {
            return InviteResult::NotFriend;
        }

        if friend.persona_state == PersonaState::Offline {
            return InviteResult::Offline;
        }

        // Check rate limit (simplified).
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Some(&last_invite) = self.invite_timestamps.get(&steam_id) {
            if now - last_invite < 60 {
                return InviteResult::RateLimited;
            }
        }

        self.invite_timestamps.insert(steam_id, now);
        InviteResult::Ok
    }

    /// Get clan count.
    pub fn get_clan_count(&self) -> usize {
        self.clans.len()
    }

    /// Get clan by index.
    pub fn get_clan_by_index(&self, index: usize) -> Option<u64> {
        self.clans.get(index).map(|c| c.clan_id)
    }

    /// Get clan name.
    pub fn get_clan_name(&self, clan_id: u64) -> Option<&str> {
        self.clans
            .iter()
            .find(|c| c.clan_id == clan_id)
            .map(|c| c.name.as_str())
    }

    /// Get clan tag.
    pub fn get_clan_tag(&self, clan_id: u64) -> Option<&str> {
        self.clans
            .iter()
            .find(|c| c.clan_id == clan_id)
            .map(|c| c.tag.as_str())
    }

    /// Get clan officer count.
    pub fn get_clan_officer_count(&self, clan_id: u64) -> usize {
        self.clans
            .iter()
            .find(|c| c.clan_id == clan_id)
            .map(|c| c.officer_count)
            .unwrap_or(0)
    }

    /// Get clan activity counts.
    pub fn get_clan_activity_counts(&self, clan_id: u64) -> Option<(u32, u32, u32)> {
        self.clans.iter().find(|c| c.clan_id == clan_id).map(|c| {
            (c.online_count, c.in_game_count, c.chatting_count)
        })
    }

    /// Get coplay friend count.
    pub fn get_coplay_friend_count(&self) -> usize {
        self.coplay.len()
    }

    /// Get coplay friend by index.
    pub fn get_coplay_friend(&self, index: usize) -> Option<&CoplayFriend> {
        self.coplay.get(index)
    }

    /// Add a friend (for testing).
    pub fn add_friend(&mut self, friend: Friend) {
        self.friends.insert(friend.steam_id, friend);
    }

    /// Add a clan (for testing).
    pub fn add_clan(&mut self, clan: Clan) {
        self.clans.push(clan);
    }

    /// Add coplay friend (for testing).
    pub fn add_coplay(&mut self, coplay: CoplayFriend) {
        self.coplay.push(coplay);
    }

    /// Set friend online status (for testing).
    pub fn set_friend_state(&mut self, steam_id: u64, state: PersonaState) {
        if let Some(friend) = self.friends.get_mut(&steam_id) {
            friend.persona_state = state;
        }
    }

    /// Set friend game info (for testing).
    pub fn set_friend_game(&mut self, steam_id: u64, app_id: u32) {
        if let Some(friend) = self.friends.get_mut(&steam_id) {
            friend.game_info.app_id = app_id;
        }
    }

    /// Set friend rich presence (for testing).
    pub fn set_friend_rich_presence(&mut self, steam_id: u64, key: &str, value: &str) {
        if let Some(friend) = self.friends.get_mut(&steam_id) {
            friend.rich_presence.insert(key.to_string(), value.to_string());
        }
    }

    /// Block a user.
    pub fn block_user(&mut self, steam_id: u64) {
        if let Some(friend) = self.friends.get_mut(&steam_id) {
            friend.relationship = FriendRelationship::Blocked;
        }
    }

    /// Unblock a user.
    pub fn unblock_user(&mut self, steam_id: u64) {
        if let Some(friend) = self.friends.get_mut(&steam_id) {
            friend.relationship = FriendRelationship::Friend;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // SOC-001: Get Friends List
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetFriendCount
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetFriendByIndex
    // =============================================================================

    #[test]
    fn soc_001_get_friends_list() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));
        mgr.add_friend(Friend::new(222, "Bob"));
        mgr.add_friend(Friend::new(333, "Charlie"));

        assert_eq!(mgr.get_friend_count(FriendFlags::ALL), 3);
    }

    #[test]
    fn soc_001_get_friend_by_index() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));
        mgr.add_friend(Friend::new(222, "Bob"));

        let friend0 = mgr.get_friend_by_index(0, FriendFlags::ALL);
        let friend1 = mgr.get_friend_by_index(1, FriendFlags::ALL);

        assert!(friend0.is_some());
        assert!(friend1.is_some());
        assert!(mgr.get_friend_by_index(2, FriendFlags::ALL).is_none());
    }

    // =============================================================================
    // SOC-002: Friend Relationship
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetFriendRelationship
    // =============================================================================

    #[test]
    fn soc_002_friend_relationship() {
        let mut mgr = FriendsManager::new(12345, 730);

        let mut friend = Friend::new(111, "Alice");
        friend.relationship = FriendRelationship::Friend;
        mgr.add_friend(friend);

        assert_eq!(
            mgr.get_friend_relationship(111),
            FriendRelationship::Friend
        );
    }

    #[test]
    fn soc_002_not_friend() {
        let mgr = FriendsManager::new(12345, 730);

        assert_eq!(
            mgr.get_friend_relationship(999),
            FriendRelationship::None
        );
    }

    #[test]
    fn soc_002_blocked_relationship() {
        let mut mgr = FriendsManager::new(12345, 730);

        let mut friend = Friend::new(111, "Blocked User");
        friend.relationship = FriendRelationship::Blocked;
        mgr.add_friend(friend);

        assert_eq!(
            mgr.get_friend_relationship(111),
            FriendRelationship::Blocked
        );
    }

    // =============================================================================
    // SOC-003: Friend Persona Name
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetFriendPersonaName
    // =============================================================================

    #[test]
    fn soc_003_friend_persona_name() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "CoolPlayer123"));

        assert_eq!(mgr.get_friend_persona_name(111), Some("CoolPlayer123"));
    }

    #[test]
    fn soc_003_unknown_user_name() {
        let mgr = FriendsManager::new(12345, 730);

        assert_eq!(mgr.get_friend_persona_name(999), None);
    }

    // =============================================================================
    // SOC-004: Friend Game Info
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetFriendGamePlayed
    // =============================================================================

    #[test]
    fn soc_004_friend_game_info() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));
        mgr.set_friend_game(111, 730);

        let info = mgr.get_friend_game_played(111);
        assert!(info.is_some());
        assert_eq!(info.unwrap().app_id, 730);
    }

    #[test]
    fn soc_004_friend_not_in_game() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));

        let info = mgr.get_friend_game_played(111);
        assert!(info.is_some());
        assert_eq!(info.unwrap().app_id, 0);
    }

    // =============================================================================
    // SOC-005: Friend State
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetFriendPersonaState
    // =============================================================================

    #[test]
    fn soc_005_friend_state() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));
        mgr.set_friend_state(111, PersonaState::Online);

        assert_eq!(mgr.get_friend_persona_state(111), PersonaState::Online);
    }

    #[test]
    fn soc_005_friend_state_away() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));
        mgr.set_friend_state(111, PersonaState::Away);

        assert_eq!(mgr.get_friend_persona_state(111), PersonaState::Away);
        assert!(PersonaState::Away.is_online());
    }

    #[test]
    fn soc_005_unknown_user_state() {
        let mgr = FriendsManager::new(12345, 730);

        assert_eq!(mgr.get_friend_persona_state(999), PersonaState::Offline);
    }

    // =============================================================================
    // SOC-006: Friend Rich Presence
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetFriendRichPresence
    // =============================================================================

    #[test]
    fn soc_006_friend_rich_presence() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));
        mgr.set_friend_rich_presence(111, "status", "In competitive match");

        assert_eq!(
            mgr.get_friend_rich_presence(111, "status"),
            Some("In competitive match")
        );
    }

    #[test]
    fn soc_006_rich_presence_missing_key() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));

        assert_eq!(mgr.get_friend_rich_presence(111, "missing"), None);
    }

    // =============================================================================
    // SOC-007: Invite Friend
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#InviteUserToGame
    // =============================================================================

    #[test]
    fn soc_007_invite_friend() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));
        mgr.set_friend_state(111, PersonaState::Online);

        let result = mgr.invite_user_to_game(111, "+connect 192.168.1.1:27015");
        assert_eq!(result, InviteResult::Ok);
    }

    #[test]
    fn soc_007_invite_not_friend() {
        let mut mgr = FriendsManager::new(12345, 730);

        let result = mgr.invite_user_to_game(999, "+connect server");
        assert_eq!(result, InviteResult::NotFriend);
    }

    #[test]
    fn soc_007_invite_blocked() {
        let mut mgr = FriendsManager::new(12345, 730);

        let mut friend = Friend::new(111, "Blocked");
        friend.relationship = FriendRelationship::Blocked;
        mgr.add_friend(friend);

        let result = mgr.invite_user_to_game(111, "+connect server");
        assert_eq!(result, InviteResult::Blocked);
    }

    #[test]
    fn soc_007_invite_offline() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));
        // Friend is offline by default.

        let result = mgr.invite_user_to_game(111, "+connect server");
        assert_eq!(result, InviteResult::Offline);
    }

    // =============================================================================
    // SOC-009: Block Player
    // =============================================================================

    #[test]
    fn soc_009_block_player() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_friend(Friend::new(111, "Alice"));
        assert_eq!(mgr.get_friend_relationship(111), FriendRelationship::Friend);

        mgr.block_user(111);
        assert_eq!(mgr.get_friend_relationship(111), FriendRelationship::Blocked);
    }

    #[test]
    fn soc_009_unblock_player() {
        let mut mgr = FriendsManager::new(12345, 730);

        let mut friend = Friend::new(111, "Alice");
        friend.relationship = FriendRelationship::Blocked;
        mgr.add_friend(friend);

        mgr.unblock_user(111);
        assert_eq!(mgr.get_friend_relationship(111), FriendRelationship::Friend);
    }

    // =============================================================================
    // SOC-010: Recently Played
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetCoplayFriendCount
    // =============================================================================

    #[test]
    fn soc_010_recently_played() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_coplay(CoplayFriend {
            steam_id: 111,
            time: 1000,
            app_id: 730,
        });
        mgr.add_coplay(CoplayFriend {
            steam_id: 222,
            time: 2000,
            app_id: 730,
        });

        assert_eq!(mgr.get_coplay_friend_count(), 2);

        let coplay = mgr.get_coplay_friend(0);
        assert!(coplay.is_some());
    }

    // =============================================================================
    // SOC-CLAN-001: Get Clan Count
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetClanCount
    // =============================================================================

    #[test]
    fn soc_clan_001_get_clan_count() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_clan(Clan::new(1001, "Valve", "VALVe"));
        mgr.add_clan(Clan::new(1002, "Steam Community", "STEAM"));

        assert_eq!(mgr.get_clan_count(), 2);
    }

    // =============================================================================
    // SOC-CLAN-002: Get Clan Details
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetClanByIndex
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetClanName
    // =============================================================================

    #[test]
    fn soc_clan_002_get_clan_details() {
        let mut mgr = FriendsManager::new(12345, 730);

        mgr.add_clan(Clan::new(1001, "Valve Corporation", "VALVe"));

        let clan_id = mgr.get_clan_by_index(0);
        assert_eq!(clan_id, Some(1001));

        assert_eq!(mgr.get_clan_name(1001), Some("Valve Corporation"));
        assert_eq!(mgr.get_clan_tag(1001), Some("VALVe"));
    }

    // =============================================================================
    // SOC-CLAN-003: Clan Officer List
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetClanOfficerCount
    // =============================================================================

    #[test]
    fn soc_clan_003_clan_officers() {
        let mut mgr = FriendsManager::new(12345, 730);

        let mut clan = Clan::new(1001, "Test Clan", "TEST");
        clan.officer_count = 5;
        mgr.add_clan(clan);

        assert_eq!(mgr.get_clan_officer_count(1001), 5);
    }

    // =============================================================================
    // SOC-CLAN-004: Clan Activity
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetClanActivityCounts
    // =============================================================================

    #[test]
    fn soc_clan_004_clan_activity() {
        let mut mgr = FriendsManager::new(12345, 730);

        let mut clan = Clan::new(1001, "Active Clan", "ACT");
        clan.online_count = 100;
        clan.in_game_count = 50;
        clan.chatting_count = 25;
        mgr.add_clan(clan);

        let activity = mgr.get_clan_activity_counts(1001);
        assert_eq!(activity, Some((100, 50, 25)));
    }

    // =============================================================================
    // Additional Tests
    // =============================================================================

    #[test]
    fn filter_friends_by_flag() {
        let mut mgr = FriendsManager::new(12345, 730);

        let mut friend1 = Friend::new(111, "Friend");
        friend1.relationship = FriendRelationship::Friend;
        mgr.add_friend(friend1);

        let mut friend2 = Friend::new(222, "Blocked");
        friend2.relationship = FriendRelationship::Blocked;
        mgr.add_friend(friend2);

        let mut friend3 = Friend::new(333, "Pending");
        friend3.relationship = FriendRelationship::RequestRecipient;
        mgr.add_friend(friend3);

        assert_eq!(mgr.get_friend_count(FriendFlags::IMMEDIATE), 1);
        assert_eq!(mgr.get_friend_count(FriendFlags::BLOCKED), 1);
        assert_eq!(mgr.get_friend_count(FriendFlags::ALL), 3);
    }

    #[test]
    fn persona_state_is_online() {
        assert!(!PersonaState::Offline.is_online());
        assert!(PersonaState::Online.is_online());
        assert!(PersonaState::Busy.is_online());
        assert!(PersonaState::Away.is_online());
        assert!(PersonaState::Snooze.is_online());
        assert!(!PersonaState::Invisible.is_online());
    }

    #[test]
    fn friend_is_in_game() {
        let mut friend = Friend::new(111, "Alice");
        assert!(!friend.is_in_game());

        friend.game_info.app_id = 730;
        assert!(friend.is_in_game());
        assert!(friend.is_playing_game(730));
        assert!(!friend.is_playing_game(440));
    }
}
