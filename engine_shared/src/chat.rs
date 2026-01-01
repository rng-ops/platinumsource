//! Chat system implementation.
//!
//! # Valve Documentation Reference
//! - [ISteamFriends Chat](https://partner.steamgames.com/doc/api/ISteamFriends#SendClanChatMessage)
//! - [In-Game Text Chat](https://developer.valvesoftware.com/wiki/Chat)
//!
//! # Chat Channels
//! - **Global**: All players on server
//! - **Team**: Team-only messages
//! - **Squad**: Squad/party messages
//! - **Private**: Direct messages between players
//!
//! # Moderation Features
//! - Rate limiting
//! - Mute system
//! - Profanity filtering (stub)
//! - Admin commands

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::steam_id::SteamId;

/// Chat message channel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChatChannel {
    /// All players.
    Global,
    /// Team-only.
    Team(u8),
    /// Squad/party.
    Squad(u32),
    /// Private message.
    Private(SteamId),
    /// Console output.
    Console,
    /// Server announcements.
    Server,
}

/// Maximum message length in characters.
pub const MAX_MESSAGE_LENGTH: usize = 256;

/// Rate limit: messages per window.
pub const RATE_LIMIT_MESSAGES: u32 = 5;
/// Rate limit: window duration.
pub const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(10);

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message sender.
    pub sender: SteamId,
    /// Sender's display name.
    pub sender_name: String,
    /// Target channel.
    pub channel: ChatChannel,
    /// Message content.
    pub content: String,
    /// Server timestamp (tick).
    pub timestamp: u64,
}

impl ChatMessage {
    pub fn new(
        sender: SteamId,
        sender_name: &str,
        channel: ChatChannel,
        content: &str,
        timestamp: u64,
    ) -> Self {
        ChatMessage {
            sender,
            sender_name: sender_name.to_string(),
            channel,
            content: content.to_string(),
            timestamp,
        }
    }

    /// Check if content exceeds max length.
    pub fn is_valid_length(&self) -> bool {
        self.content.len() <= MAX_MESSAGE_LENGTH
    }

    /// Truncate content to max length.
    pub fn truncate(&mut self) {
        if self.content.len() > MAX_MESSAGE_LENGTH {
            self.content.truncate(MAX_MESSAGE_LENGTH);
        }
    }
}

/// Rate limiter for chat spam prevention.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Timestamps of recent messages.
    history: VecDeque<Instant>,
    /// Max messages in window.
    max_messages: u32,
    /// Window duration.
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_messages: u32, window: Duration) -> Self {
        RateLimiter {
            history: VecDeque::with_capacity(max_messages as usize),
            max_messages,
            window,
        }
    }

    /// Check if a message can be sent (without recording it).
    pub fn can_send(&self) -> bool {
        let cutoff = Instant::now() - self.window;
        let recent = self.history.iter().filter(|&&t| t > cutoff).count();
        (recent as u32) < self.max_messages
    }

    /// Record a message being sent. Returns false if rate limited.
    pub fn record_message(&mut self) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;

        // Remove old messages
        while let Some(&front) = self.history.front() {
            if front <= cutoff {
                self.history.pop_front();
            } else {
                break;
            }
        }

        // Check if allowed
        if (self.history.len() as u32) >= self.max_messages {
            return false;
        }

        self.history.push_back(now);
        true
    }

    /// Get remaining messages in current window.
    pub fn remaining(&self) -> u32 {
        let cutoff = Instant::now() - self.window;
        let recent = self.history.iter().filter(|&&t| t > cutoff).count() as u32;
        self.max_messages.saturating_sub(recent)
    }

    /// Get time until next message is allowed.
    pub fn time_until_allowed(&self) -> Option<Duration> {
        if self.can_send() {
            return None;
        }

        let cutoff = Instant::now() - self.window;
        self.history
            .front()
            .map(|&oldest| {
                let expires = oldest + self.window;
                expires.saturating_duration_since(Instant::now())
            })
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        RateLimiter::new(RATE_LIMIT_MESSAGES, RATE_LIMIT_WINDOW)
    }
}

/// Chat state for a player.
#[derive(Debug)]
pub struct PlayerChatState {
    pub steam_id: SteamId,
    pub rate_limiter: RateLimiter,
    /// Players this user has muted.
    pub muted_players: HashSet<SteamId>,
    /// Whether this player is server-muted.
    pub server_muted: bool,
    /// Mute expiry time (if temporary).
    pub mute_expires: Option<Instant>,
    /// Current team ID.
    pub team_id: Option<u8>,
    /// Current squad ID.
    pub squad_id: Option<u32>,
}

impl PlayerChatState {
    pub fn new(steam_id: SteamId) -> Self {
        PlayerChatState {
            steam_id,
            rate_limiter: RateLimiter::default(),
            muted_players: HashSet::new(),
            server_muted: false,
            mute_expires: None,
            team_id: None,
            squad_id: None,
        }
    }

    /// Check if player can receive messages from sender.
    pub fn can_receive_from(&self, sender: SteamId) -> bool {
        !self.muted_players.contains(&sender)
    }

    /// Check if player can send messages.
    pub fn can_send(&self) -> bool {
        if self.server_muted {
            if let Some(expires) = self.mute_expires {
                if Instant::now() < expires {
                    return false;
                }
                // Mute expired, will be cleared on next check
            } else {
                return false; // Permanent mute
            }
        }
        self.rate_limiter.can_send()
    }

    /// Mute a player.
    pub fn mute_player(&mut self, player: SteamId) {
        self.muted_players.insert(player);
    }

    /// Unmute a player.
    pub fn unmute_player(&mut self, player: SteamId) {
        self.muted_players.remove(&player);
    }

    /// Apply server mute.
    pub fn server_mute(&mut self, duration: Option<Duration>) {
        self.server_muted = true;
        self.mute_expires = duration.map(|d| Instant::now() + d);
    }

    /// Remove server mute.
    pub fn server_unmute(&mut self) {
        self.server_muted = false;
        self.mute_expires = None;
    }
}

/// Chat result for operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatResult {
    Ok,
    RateLimited,
    Muted,
    InvalidChannel,
    MessageTooLong,
    SenderNotFound,
    RecipientNotFound,
}

/// Chat manager for handling server-side chat.
#[derive(Default)]
pub struct ChatManager {
    /// Per-player chat state.
    players: HashMap<SteamId, PlayerChatState>,
    /// Message history (limited).
    history: VecDeque<ChatMessage>,
    /// Max history size.
    max_history: usize,
    /// Current server tick.
    current_tick: u64,
}

impl ChatManager {
    pub fn new(max_history: usize) -> Self {
        ChatManager {
            players: HashMap::new(),
            history: VecDeque::new(),
            max_history,
            current_tick: 0,
        }
    }

    /// Set current tick for timestamps.
    pub fn set_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Register a player.
    pub fn add_player(&mut self, steam_id: SteamId) {
        self.players.insert(steam_id, PlayerChatState::new(steam_id));
    }

    /// Remove a player.
    pub fn remove_player(&mut self, steam_id: SteamId) {
        self.players.remove(&steam_id);
    }

    /// Get player state.
    pub fn get_player(&self, steam_id: SteamId) -> Option<&PlayerChatState> {
        self.players.get(&steam_id)
    }

    /// Get mutable player state.
    pub fn get_player_mut(&mut self, steam_id: SteamId) -> Option<&mut PlayerChatState> {
        self.players.get_mut(&steam_id)
    }

    /// Set player's team.
    pub fn set_player_team(&mut self, steam_id: SteamId, team_id: Option<u8>) {
        if let Some(state) = self.players.get_mut(&steam_id) {
            state.team_id = team_id;
        }
    }

    /// Set player's squad.
    pub fn set_player_squad(&mut self, steam_id: SteamId, squad_id: Option<u32>) {
        if let Some(state) = self.players.get_mut(&steam_id) {
            state.squad_id = squad_id;
        }
    }

    /// Send a chat message.
    pub fn send_message(
        &mut self,
        sender: SteamId,
        sender_name: &str,
        channel: ChatChannel,
        content: &str,
    ) -> Result<Vec<SteamId>, ChatResult> {
        // Validate sender
        let sender_state = self.players.get_mut(&sender).ok_or(ChatResult::SenderNotFound)?;

        // Check mute status
        if sender_state.server_muted {
            if let Some(expires) = sender_state.mute_expires {
                if Instant::now() < expires {
                    return Err(ChatResult::Muted);
                }
                sender_state.server_muted = false;
                sender_state.mute_expires = None;
            } else {
                return Err(ChatResult::Muted);
            }
        }

        // Check rate limit
        if !sender_state.rate_limiter.record_message() {
            return Err(ChatResult::RateLimited);
        }

        // Validate message length
        if content.len() > MAX_MESSAGE_LENGTH {
            return Err(ChatResult::MessageTooLong);
        }

        // Create message
        let message = ChatMessage::new(sender, sender_name, channel, content, self.current_tick);

        // Determine recipients
        let recipients: Vec<SteamId> = self.players
            .iter()
            .filter(|(&pid, state)| {
                // Don't send to sender (they already have it)
                if pid == sender {
                    return false;
                }
                // Check if recipient has muted sender
                if !state.can_receive_from(sender) {
                    return false;
                }
                // Check channel access
                match channel {
                    ChatChannel::Global | ChatChannel::Server | ChatChannel::Console => true,
                    ChatChannel::Team(team) => state.team_id == Some(team),
                    ChatChannel::Squad(squad) => state.squad_id == Some(squad),
                    ChatChannel::Private(target) => pid == target,
                }
            })
            .map(|(&pid, _)| pid)
            .collect();

        // Store in history
        self.history.push_back(message);
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }

        Ok(recipients)
    }

    /// Get recent history.
    pub fn get_history(&self, count: usize) -> Vec<&ChatMessage> {
        self.history.iter().rev().take(count).collect()
    }

    /// Clear history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Admin mute a player.
    pub fn admin_mute(&mut self, target: SteamId, duration: Option<Duration>) -> bool {
        if let Some(state) = self.players.get_mut(&target) {
            state.server_mute(duration);
            true
        } else {
            false
        }
    }

    /// Admin unmute a player.
    pub fn admin_unmute(&mut self, target: SteamId) -> bool {
        if let Some(state) = self.players.get_mut(&target) {
            state.server_unmute();
            true
        } else {
            false
        }
    }
}

/// Simple profanity filter (stub implementation).
pub struct ProfanityFilter {
    blocked_words: HashSet<String>,
    replacement: char,
}

impl ProfanityFilter {
    pub fn new() -> Self {
        ProfanityFilter {
            blocked_words: HashSet::new(),
            replacement: '*',
        }
    }

    /// Add a word to the filter.
    pub fn add_word(&mut self, word: &str) {
        self.blocked_words.insert(word.to_lowercase());
    }

    /// Filter a message, replacing blocked words.
    pub fn filter(&self, message: &str) -> String {
        let mut result = message.to_string();
        for word in &self.blocked_words {
            let replacement = self.replacement.to_string().repeat(word.len());
            // Simple case-insensitive replacement
            let lower = result.to_lowercase();
            while let Some(pos) = lower.find(word.as_str()) {
                let end = pos + word.len();
                result.replace_range(pos..end, &replacement);
            }
        }
        result
    }

    /// Check if message contains blocked words.
    pub fn contains_blocked(&self, message: &str) -> bool {
        let lower = message.to_lowercase();
        self.blocked_words.iter().any(|w| lower.contains(w.as_str()))
    }
}

impl Default for ProfanityFilter {
    fn default() -> Self {
        Self::new()
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
    // CHAT-001: Global Chat
    // Reference: https://developer.valvesoftware.com/wiki/Chat
    // =============================================================================

    #[test]
    fn chat_001_global_broadcast() {
        let mut manager = ChatManager::new(100);
        
        let sender = test_steam_id(1);
        let receiver1 = test_steam_id(2);
        let receiver2 = test_steam_id(3);
        
        manager.add_player(sender);
        manager.add_player(receiver1);
        manager.add_player(receiver2);
        
        let recipients = manager.send_message(
            sender,
            "Player1",
            ChatChannel::Global,
            "Hello everyone!"
        ).unwrap();
        
        assert_eq!(recipients.len(), 2);
        assert!(recipients.contains(&receiver1));
        assert!(recipients.contains(&receiver2));
    }

    // =============================================================================
    // CHAT-002: Team Chat
    // =============================================================================

    #[test]
    fn chat_002_team_only() {
        let mut manager = ChatManager::new(100);
        
        let sender = test_steam_id(1);
        let teammate = test_steam_id(2);
        let enemy = test_steam_id(3);
        
        manager.add_player(sender);
        manager.add_player(teammate);
        manager.add_player(enemy);
        
        manager.set_player_team(sender, Some(1));
        manager.set_player_team(teammate, Some(1));
        manager.set_player_team(enemy, Some(2));
        
        let recipients = manager.send_message(
            sender,
            "Player1",
            ChatChannel::Team(1),
            "Team message"
        ).unwrap();
        
        assert_eq!(recipients.len(), 1);
        assert!(recipients.contains(&teammate));
        assert!(!recipients.contains(&enemy));
    }

    // =============================================================================
    // CHAT-003: Squad Chat
    // =============================================================================

    #[test]
    fn chat_003_squad_only() {
        let mut manager = ChatManager::new(100);
        
        let sender = test_steam_id(1);
        let squadmate = test_steam_id(2);
        let other = test_steam_id(3);
        
        manager.add_player(sender);
        manager.add_player(squadmate);
        manager.add_player(other);
        
        manager.set_player_squad(sender, Some(100));
        manager.set_player_squad(squadmate, Some(100));
        manager.set_player_squad(other, Some(200));
        
        let recipients = manager.send_message(
            sender,
            "Player1",
            ChatChannel::Squad(100),
            "Squad message"
        ).unwrap();
        
        assert_eq!(recipients.len(), 1);
        assert!(recipients.contains(&squadmate));
    }

    // =============================================================================
    // CHAT-004: Private Message
    // =============================================================================

    #[test]
    fn chat_004_private_message() {
        let mut manager = ChatManager::new(100);
        
        let sender = test_steam_id(1);
        let recipient = test_steam_id(2);
        let other = test_steam_id(3);
        
        manager.add_player(sender);
        manager.add_player(recipient);
        manager.add_player(other);
        
        let recipients = manager.send_message(
            sender,
            "Player1",
            ChatChannel::Private(recipient),
            "Private message"
        ).unwrap();
        
        assert_eq!(recipients.len(), 1);
        assert!(recipients.contains(&recipient));
        assert!(!recipients.contains(&other));
    }

    // =============================================================================
    // CHAT-006: Message Length Limit
    // =============================================================================

    #[test]
    fn chat_006_message_length_limit() {
        let mut manager = ChatManager::new(100);
        let sender = test_steam_id(1);
        manager.add_player(sender);
        
        let long_message = "x".repeat(MAX_MESSAGE_LENGTH + 1);
        
        let result = manager.send_message(
            sender,
            "Player1",
            ChatChannel::Global,
            &long_message
        );
        
        assert_eq!(result, Err(ChatResult::MessageTooLong));
    }

    #[test]
    fn chat_006_max_length_accepted() {
        let mut manager = ChatManager::new(100);
        let sender = test_steam_id(1);
        manager.add_player(sender);
        
        let max_message = "x".repeat(MAX_MESSAGE_LENGTH);
        
        let result = manager.send_message(
            sender,
            "Player1",
            ChatChannel::Global,
            &max_message
        );
        
        assert!(result.is_ok());
    }

    // =============================================================================
    // CHAT-007: Rate Limiting
    // =============================================================================

    #[test]
    fn chat_007_rate_limiting() {
        let mut limiter = RateLimiter::new(3, Duration::from_millis(100));
        
        // Send 3 messages (should succeed)
        assert!(limiter.record_message());
        assert!(limiter.record_message());
        assert!(limiter.record_message());
        
        // 4th message should fail
        assert!(!limiter.record_message());
        assert!(!limiter.can_send());
    }

    #[test]
    fn chat_007_rate_limit_recovery() {
        let mut limiter = RateLimiter::new(2, Duration::from_millis(50));
        
        limiter.record_message();
        limiter.record_message();
        assert!(!limiter.can_send());
        
        // Wait for window to pass
        sleep(Duration::from_millis(60));
        
        assert!(limiter.can_send());
        assert!(limiter.record_message());
    }

    #[test]
    fn chat_007_rate_limit_remaining() {
        let limiter = RateLimiter::new(5, Duration::from_secs(10));
        assert_eq!(limiter.remaining(), 5);
    }

    // =============================================================================
    // CHAT-008: Mute Player
    // =============================================================================

    #[test]
    fn chat_008_player_mute() {
        let mut manager = ChatManager::new(100);
        
        let sender = test_steam_id(1);
        let receiver = test_steam_id(2);
        
        manager.add_player(sender);
        manager.add_player(receiver);
        
        // Receiver mutes sender
        manager.get_player_mut(receiver).unwrap().mute_player(sender);
        
        let recipients = manager.send_message(
            sender,
            "Player1",
            ChatChannel::Global,
            "Hello"
        ).unwrap();
        
        // Receiver should not receive the message
        assert!(!recipients.contains(&receiver));
    }

    #[test]
    fn chat_008_unmute_player() {
        let mut manager = ChatManager::new(100);
        
        let sender = test_steam_id(1);
        let receiver = test_steam_id(2);
        
        manager.add_player(sender);
        manager.add_player(receiver);
        
        // Mute then unmute
        manager.get_player_mut(receiver).unwrap().mute_player(sender);
        manager.get_player_mut(receiver).unwrap().unmute_player(sender);
        
        let recipients = manager.send_message(
            sender,
            "Player1",
            ChatChannel::Global,
            "Hello"
        ).unwrap();
        
        // Receiver should receive the message now
        assert!(recipients.contains(&receiver));
    }

    // =============================================================================
    // CHAT-MOD-003: Admin Mute
    // =============================================================================

    #[test]
    fn chat_mod_003_admin_mute() {
        let mut manager = ChatManager::new(100);
        let player = test_steam_id(1);
        manager.add_player(player);
        
        // Admin mutes player
        manager.admin_mute(player, None);
        
        let result = manager.send_message(
            player,
            "Player1",
            ChatChannel::Global,
            "Test"
        );
        
        assert_eq!(result, Err(ChatResult::Muted));
    }

    #[test]
    fn chat_mod_003_timed_mute() {
        let mut manager = ChatManager::new(100);
        let player = test_steam_id(1);
        manager.add_player(player);
        
        // Admin mutes player for 50ms
        manager.admin_mute(player, Some(Duration::from_millis(50)));
        
        // Should be muted immediately
        let result = manager.send_message(player, "Player1", ChatChannel::Global, "Test");
        assert_eq!(result, Err(ChatResult::Muted));
        
        // Wait for mute to expire
        sleep(Duration::from_millis(60));
        
        // Should be able to send now
        let result = manager.send_message(player, "Player1", ChatChannel::Global, "Test");
        assert!(result.is_ok());
    }

    #[test]
    fn chat_mod_003_admin_unmute() {
        let mut manager = ChatManager::new(100);
        let player = test_steam_id(1);
        manager.add_player(player);
        
        manager.admin_mute(player, None);
        manager.admin_unmute(player);
        
        let result = manager.send_message(player, "Player1", ChatChannel::Global, "Test");
        assert!(result.is_ok());
    }

    // =============================================================================
    // CHAT-010: Chat History
    // =============================================================================

    #[test]
    fn chat_010_history() {
        let mut manager = ChatManager::new(5);
        let sender = test_steam_id(1);
        manager.add_player(sender);
        
        for i in 0..3 {
            manager.send_message(sender, "Player1", ChatChannel::Global, &format!("Message {}", i)).unwrap();
        }
        
        let history = manager.get_history(10);
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn chat_010_history_limit() {
        let mut manager = ChatManager::new(3);
        let sender = test_steam_id(1);
        manager.add_player(sender);
        
        for i in 0..5 {
            manager.send_message(sender, "Player1", ChatChannel::Global, &format!("Message {}", i)).unwrap();
        }
        
        let history = manager.get_history(10);
        assert_eq!(history.len(), 3); // Limited to max_history
    }

    // =============================================================================
    // Profanity Filter Tests
    // =============================================================================

    #[test]
    fn profanity_filter_basic() {
        let mut filter = ProfanityFilter::new();
        filter.add_word("bad");
        
        let result = filter.filter("This is a bad word");
        assert_eq!(result, "This is a *** word");
    }

    #[test]
    fn profanity_filter_detection() {
        let mut filter = ProfanityFilter::new();
        filter.add_word("blocked");
        
        assert!(filter.contains_blocked("This is blocked content"));
        assert!(!filter.contains_blocked("This is fine"));
    }
}
