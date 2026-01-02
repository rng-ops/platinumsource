//! Party system implementation.
//!
//! # Overview
//! Parties are groups of players that queue and play together.
//! Unlike lobbies, parties are more persistent and follow players
//! across games and matchmaking sessions.
//!
//! # Features
//! - Party creation and management
//! - Invite system
//! - Ready-up synchronization
//! - Party chat
//! - Cross-game persistence

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::steam_id::SteamId;

/// Unique party identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartyId(u64);

impl PartyId {
    pub const INVALID: PartyId = PartyId(0);

    pub fn new(id: u64) -> Self {
        PartyId(id)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Party member data.
#[derive(Debug, Clone)]
pub struct PartyMember {
    /// Member's Steam ID.
    pub steam_id: SteamId,
    /// Display name.
    pub name: String,
    /// Ready status.
    pub ready: bool,
    /// When they joined.
    pub joined_at: Instant,
    /// Is party leader.
    pub is_leader: bool,
}

impl PartyMember {
    pub fn new(steam_id: SteamId, name: &str, is_leader: bool) -> Self {
        PartyMember {
            steam_id,
            name: name.to_string(),
            ready: false,
            joined_at: Instant::now(),
            is_leader,
        }
    }
}

/// Party invite.
#[derive(Debug, Clone)]
pub struct PartyInvite {
    /// Party the invite is for.
    pub party_id: PartyId,
    /// Who sent the invite.
    pub from: SteamId,
    /// Who the invite is for.
    pub to: SteamId,
    /// When the invite was sent.
    pub sent_at: Instant,
    /// Invite expiration.
    pub expires_at: Instant,
}

impl PartyInvite {
    pub fn new(party_id: PartyId, from: SteamId, to: SteamId, duration: Duration) -> Self {
        let now = Instant::now();
        PartyInvite {
            party_id,
            from,
            to,
            sent_at: now,
            expires_at: now + duration,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Party state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyState {
    /// In lobby, waiting for members.
    Idle,
    /// All members ready, preparing to queue.
    Ready,
    /// In matchmaking queue.
    Queuing,
    /// Match found, loading.
    Loading,
    /// In game.
    InGame,
}

/// A party of players.
#[derive(Debug, Clone)]
pub struct Party {
    /// Party ID.
    pub id: PartyId,
    /// Party leader.
    pub leader: SteamId,
    /// Party members.
    members: Vec<PartyMember>,
    /// Maximum size.
    pub max_size: u32,
    /// Current state.
    pub state: PartyState,
    /// Created time.
    pub created_at: Instant,
    /// Pending invites.
    invites: Vec<PartyInvite>,
    /// Party metadata.
    pub metadata: HashMap<String, String>,
}

/// Party size limits.
pub const MIN_PARTY_SIZE: u32 = 1;
pub const MAX_PARTY_SIZE: u32 = 10;
pub const DEFAULT_INVITE_DURATION: Duration = Duration::from_secs(60);

impl Party {
    /// Create a new party with the given leader.
    pub fn new(id: PartyId, leader: SteamId, leader_name: &str, max_size: u32) -> Self {
        let max_size = max_size.clamp(MIN_PARTY_SIZE, MAX_PARTY_SIZE);
        let leader_member = PartyMember::new(leader, leader_name, true);

        Party {
            id,
            leader,
            members: vec![leader_member],
            max_size,
            state: PartyState::Idle,
            created_at: Instant::now(),
            invites: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Get member count.
    pub fn member_count(&self) -> u32 {
        self.members.len() as u32
    }

    /// Check if party is full.
    pub fn is_full(&self) -> bool {
        self.member_count() >= self.max_size
    }

    /// Check if a player is in the party.
    pub fn is_member(&self, steam_id: SteamId) -> bool {
        self.members.iter().any(|m| m.steam_id == steam_id)
    }

    /// Check if a player is the leader.
    pub fn is_leader(&self, steam_id: SteamId) -> bool {
        self.leader == steam_id
    }

    /// Get a member.
    pub fn get_member(&self, steam_id: SteamId) -> Option<&PartyMember> {
        self.members.iter().find(|m| m.steam_id == steam_id)
    }

    /// Get all members.
    pub fn members(&self) -> &[PartyMember] {
        &self.members
    }

    /// Send an invite.
    pub fn invite(&mut self, to: SteamId) -> Result<(), PartyError> {
        if self.is_full() {
            return Err(PartyError::PartyFull);
        }
        if self.is_member(to) {
            return Err(PartyError::AlreadyMember);
        }
        if self.invites.iter().any(|i| i.to == to && !i.is_expired()) {
            return Err(PartyError::InvitePending);
        }

        let invite = PartyInvite::new(self.id, self.leader, to, DEFAULT_INVITE_DURATION);
        self.invites.push(invite);
        Ok(())
    }

    /// Accept an invite and join.
    pub fn accept_invite(&mut self, member: SteamId, name: &str) -> Result<(), PartyError> {
        // Clean expired invites
        self.invites.retain(|i| !i.is_expired());

        let invite_idx = self.invites.iter().position(|i| i.to == member);
        if invite_idx.is_none() {
            return Err(PartyError::NoInvite);
        }

        if self.is_full() {
            return Err(PartyError::PartyFull);
        }
        if self.is_member(member) {
            return Err(PartyError::AlreadyMember);
        }

        self.invites.remove(invite_idx.unwrap());
        self.members.push(PartyMember::new(member, name, false));
        Ok(())
    }

    /// Decline an invite.
    pub fn decline_invite(&mut self, member: SteamId) -> Result<(), PartyError> {
        let invite_idx = self.invites.iter().position(|i| i.to == member);
        if let Some(idx) = invite_idx {
            self.invites.remove(idx);
            Ok(())
        } else {
            Err(PartyError::NoInvite)
        }
    }

    /// Leave the party.
    pub fn leave(&mut self, member: SteamId) -> Result<(), PartyError> {
        if !self.is_member(member) {
            return Err(PartyError::NotMember);
        }

        self.members.retain(|m| m.steam_id != member);

        // Transfer leadership if leader left
        if self.leader == member && !self.members.is_empty() {
            self.leader = self.members[0].steam_id;
            self.members[0].is_leader = true;
        }

        Ok(())
    }

    /// Kick a member (leader only).
    pub fn kick(&mut self, kicker: SteamId, target: SteamId) -> Result<(), PartyError> {
        if !self.is_leader(kicker) {
            return Err(PartyError::NotLeader);
        }
        if kicker == target {
            return Err(PartyError::CannotKickSelf);
        }
        if !self.is_member(target) {
            return Err(PartyError::NotMember);
        }

        self.members.retain(|m| m.steam_id != target);
        Ok(())
    }

    /// Transfer leadership.
    pub fn transfer_leadership(
        &mut self,
        current_leader: SteamId,
        new_leader: SteamId,
    ) -> Result<(), PartyError> {
        if !self.is_leader(current_leader) {
            return Err(PartyError::NotLeader);
        }
        if !self.is_member(new_leader) {
            return Err(PartyError::NotMember);
        }

        // Update leader flags
        for member in &mut self.members {
            member.is_leader = member.steam_id == new_leader;
        }
        self.leader = new_leader;
        Ok(())
    }

    /// Set ready status.
    pub fn set_ready(&mut self, member: SteamId, ready: bool) -> Result<(), PartyError> {
        if let Some(m) = self.members.iter_mut().find(|m| m.steam_id == member) {
            m.ready = ready;
            self.update_state();
            Ok(())
        } else {
            Err(PartyError::NotMember)
        }
    }

    /// Check if all members are ready.
    pub fn all_ready(&self) -> bool {
        self.members.iter().all(|m| m.ready)
    }

    /// Update party state based on member readiness.
    fn update_state(&mut self) {
        if self.state == PartyState::Idle && self.all_ready() {
            self.state = PartyState::Ready;
        } else if self.state == PartyState::Ready && !self.all_ready() {
            self.state = PartyState::Idle;
        }
    }

    /// Check if party is empty.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Get pending invite count.
    pub fn pending_invite_count(&self) -> usize {
        self.invites.iter().filter(|i| !i.is_expired()).count()
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Get metadata.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }
}

/// Party operation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartyError {
    PartyFull,
    AlreadyMember,
    NotMember,
    NotLeader,
    NoInvite,
    InvitePending,
    InviteExpired,
    CannotKickSelf,
    PartyNotFound,
}

/// Party manager for tracking multiple parties.
#[derive(Default)]
pub struct PartyManager {
    /// All parties.
    parties: HashMap<PartyId, Party>,
    /// Player -> party mapping.
    player_parties: HashMap<SteamId, PartyId>,
    /// Pending invites for players.
    player_invites: HashMap<SteamId, Vec<PartyId>>,
    /// Next party ID.
    next_id: u64,
}

impl PartyManager {
    /// Create a new party manager.
    pub fn new() -> Self {
        PartyManager {
            parties: HashMap::new(),
            player_parties: HashMap::new(),
            player_invites: HashMap::new(),
            next_id: 1,
        }
    }

    /// Create a new party.
    pub fn create_party(
        &mut self,
        leader: SteamId,
        leader_name: &str,
        max_size: u32,
    ) -> Result<PartyId, PartyError> {
        // Leave any existing party first
        if let Some(existing) = self.get_player_party(leader) {
            self.leave_party(leader)?;
        }

        let id = PartyId::new(self.next_id);
        self.next_id += 1;

        let party = Party::new(id, leader, leader_name, max_size);
        self.parties.insert(id, party);
        self.player_parties.insert(leader, id);

        Ok(id)
    }

    /// Get a party by ID.
    pub fn get_party(&self, id: PartyId) -> Option<&Party> {
        self.parties.get(&id)
    }

    /// Get a mutable party by ID.
    pub fn get_party_mut(&mut self, id: PartyId) -> Option<&mut Party> {
        self.parties.get_mut(&id)
    }

    /// Get a player's current party.
    pub fn get_player_party(&self, player: SteamId) -> Option<PartyId> {
        self.player_parties.get(&player).copied()
    }

    /// Send invite.
    pub fn send_invite(&mut self, party_id: PartyId, to: SteamId) -> Result<(), PartyError> {
        let party = self.parties.get_mut(&party_id).ok_or(PartyError::PartyNotFound)?;
        party.invite(to)?;

        self.player_invites
            .entry(to)
            .or_default()
            .push(party_id);

        Ok(())
    }

    /// Accept invite.
    pub fn accept_invite(
        &mut self,
        player: SteamId,
        name: &str,
        party_id: PartyId,
    ) -> Result<(), PartyError> {
        // Leave current party if any
        if let Some(current) = self.get_player_party(player) {
            if current != party_id {
                self.leave_party(player)?;
            }
        }

        let party = self.parties.get_mut(&party_id).ok_or(PartyError::PartyNotFound)?;
        party.accept_invite(player, name)?;

        self.player_parties.insert(player, party_id);
        self.player_invites.entry(player).or_default().retain(|&id| id != party_id);

        Ok(())
    }

    /// Decline invite.
    pub fn decline_invite(&mut self, player: SteamId, party_id: PartyId) -> Result<(), PartyError> {
        let party = self.parties.get_mut(&party_id).ok_or(PartyError::PartyNotFound)?;
        party.decline_invite(player)?;

        self.player_invites.entry(player).or_default().retain(|&id| id != party_id);
        Ok(())
    }

    /// Leave party.
    pub fn leave_party(&mut self, player: SteamId) -> Result<(), PartyError> {
        let party_id = self.player_parties.get(&player).copied().ok_or(PartyError::NotMember)?;

        let party = self.parties.get_mut(&party_id).ok_or(PartyError::PartyNotFound)?;
        party.leave(player)?;

        self.player_parties.remove(&player);

        // Clean up empty parties
        if party.is_empty() {
            self.parties.remove(&party_id);
        }

        Ok(())
    }

    /// Kick player.
    pub fn kick_player(
        &mut self,
        kicker: SteamId,
        target: SteamId,
    ) -> Result<(), PartyError> {
        let party_id = self.player_parties.get(&kicker).copied().ok_or(PartyError::NotMember)?;

        let party = self.parties.get_mut(&party_id).ok_or(PartyError::PartyNotFound)?;
        party.kick(kicker, target)?;

        self.player_parties.remove(&target);
        Ok(())
    }

    /// Get pending invites for a player.
    pub fn get_invites(&self, player: SteamId) -> Vec<PartyId> {
        self.player_invites.get(&player).cloned().unwrap_or_default()
    }

    /// Clean up empty parties.
    pub fn cleanup_empty(&mut self) {
        let empty_ids: Vec<_> = self
            .parties
            .iter()
            .filter(|(_, p)| p.is_empty())
            .map(|(id, _)| *id)
            .collect();

        for id in empty_ids {
            self.parties.remove(&id);
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
    // PTY-001: Create Party
    // =============================================================================

    #[test]
    fn pty_001_create_party() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let party_id = manager.create_party(leader, "Leader", 5).unwrap();

        assert!(party_id.is_valid());

        let party = manager.get_party(party_id).unwrap();
        assert_eq!(party.leader, leader);
        assert_eq!(party.member_count(), 1);
        assert!(party.is_leader(leader));
    }

    // =============================================================================
    // PTY-002: Invite to Party
    // =============================================================================

    #[test]
    fn pty_002_send_invite() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let invitee = test_steam_id(2);

        let party_id = manager.create_party(leader, "Leader", 5).unwrap();
        manager.send_invite(party_id, invitee).unwrap();

        let party = manager.get_party(party_id).unwrap();
        assert_eq!(party.pending_invite_count(), 1);

        let invites = manager.get_invites(invitee);
        assert_eq!(invites.len(), 1);
    }

    // =============================================================================
    // PTY-003: Accept Invite
    // =============================================================================

    #[test]
    fn pty_003_accept_invite() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let member = test_steam_id(2);

        let party_id = manager.create_party(leader, "Leader", 5).unwrap();
        manager.send_invite(party_id, member).unwrap();
        manager.accept_invite(member, "Member", party_id).unwrap();

        let party = manager.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 2);
        assert!(party.is_member(member));
    }

    // =============================================================================
    // PTY-004: Decline Invite
    // =============================================================================

    #[test]
    fn pty_004_decline_invite() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let invitee = test_steam_id(2);

        let party_id = manager.create_party(leader, "Leader", 5).unwrap();
        manager.send_invite(party_id, invitee).unwrap();
        manager.decline_invite(invitee, party_id).unwrap();

        let party = manager.get_party(party_id).unwrap();
        assert_eq!(party.pending_invite_count(), 0);
        assert!(!party.is_member(invitee));
    }

    // =============================================================================
    // PTY-005: Leave Party
    // =============================================================================

    #[test]
    fn pty_005_leave_party() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let member = test_steam_id(2);

        let party_id = manager.create_party(leader, "Leader", 5).unwrap();
        manager.send_invite(party_id, member).unwrap();
        manager.accept_invite(member, "Member", party_id).unwrap();

        manager.leave_party(member).unwrap();

        let party = manager.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 1);
        assert!(!party.is_member(member));
    }

    // =============================================================================
    // PTY-006: Kick from Party
    // =============================================================================

    #[test]
    fn pty_006_kick_member() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let member = test_steam_id(2);

        let party_id = manager.create_party(leader, "Leader", 5).unwrap();
        manager.send_invite(party_id, member).unwrap();
        manager.accept_invite(member, "Member", party_id).unwrap();

        manager.kick_player(leader, member).unwrap();

        let party = manager.get_party(party_id).unwrap();
        assert_eq!(party.member_count(), 1);
        assert!(!party.is_member(member));
    }

    #[test]
    fn pty_006_non_leader_cannot_kick() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let member1 = test_steam_id(2);
        let member2 = test_steam_id(3);

        let party_id = manager.create_party(leader, "Leader", 5).unwrap();
        manager.send_invite(party_id, member1).unwrap();
        manager.accept_invite(member1, "Member1", party_id).unwrap();
        manager.send_invite(party_id, member2).unwrap();
        manager.accept_invite(member2, "Member2", party_id).unwrap();

        // member1 tries to kick member2 - should fail
        let result = manager.kick_player(member1, member2);
        assert_eq!(result, Err(PartyError::NotLeader));
    }

    // =============================================================================
    // PTY-007: Party Leader Change
    // =============================================================================

    #[test]
    fn pty_007_transfer_leadership() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let member = test_steam_id(2);

        let party_id = manager.create_party(leader, "Leader", 5).unwrap();
        manager.send_invite(party_id, member).unwrap();
        manager.accept_invite(member, "Member", party_id).unwrap();

        let party = manager.get_party_mut(party_id).unwrap();
        party.transfer_leadership(leader, member).unwrap();

        assert_eq!(party.leader, member);
        assert!(party.is_leader(member));
        assert!(!party.is_leader(leader));
    }

    #[test]
    fn pty_007_auto_transfer_on_leave() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let member = test_steam_id(2);

        let party_id = manager.create_party(leader, "Leader", 5).unwrap();
        manager.send_invite(party_id, member).unwrap();
        manager.accept_invite(member, "Member", party_id).unwrap();

        // Leader leaves
        manager.leave_party(leader).unwrap();

        let party = manager.get_party(party_id).unwrap();
        assert_eq!(party.leader, member);
    }

    // =============================================================================
    // PTY-008: Party Auto-Disband
    // =============================================================================

    #[test]
    fn pty_008_auto_disband() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let party_id = manager.create_party(leader, "Leader", 5).unwrap();

        manager.leave_party(leader).unwrap();

        assert!(manager.get_party(party_id).is_none());
    }

    // =============================================================================
    // PTY-009: Party Size Limit
    // =============================================================================

    #[test]
    fn pty_009_size_limit() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let party_id = manager.create_party(leader, "Leader", 2).unwrap();

        manager.send_invite(party_id, test_steam_id(2)).unwrap();
        manager.accept_invite(test_steam_id(2), "M2", party_id).unwrap();

        // Party is now full
        let result = manager.send_invite(party_id, test_steam_id(3));
        assert_eq!(result, Err(PartyError::PartyFull));
    }

    // =============================================================================
    // PTY-SYNC-002: Ready State
    // =============================================================================

    #[test]
    fn pty_sync_002_ready_state() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let member = test_steam_id(2);

        let party_id = manager.create_party(leader, "Leader", 5).unwrap();
        manager.send_invite(party_id, member).unwrap();
        manager.accept_invite(member, "Member", party_id).unwrap();

        let party = manager.get_party_mut(party_id).unwrap();

        // Initially not ready
        assert!(!party.all_ready());
        assert_eq!(party.state, PartyState::Idle);

        // Both ready up
        party.set_ready(leader, true).unwrap();
        party.set_ready(member, true).unwrap();

        assert!(party.all_ready());
        assert_eq!(party.state, PartyState::Ready);
    }

    #[test]
    fn pty_sync_002_unready() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let party_id = manager.create_party(leader, "Leader", 5).unwrap();

        let party = manager.get_party_mut(party_id).unwrap();
        party.set_ready(leader, true).unwrap();
        assert_eq!(party.state, PartyState::Ready);

        party.set_ready(leader, false).unwrap();
        assert_eq!(party.state, PartyState::Idle);
    }

    // =============================================================================
    // Additional Tests
    // =============================================================================

    #[test]
    fn party_metadata() {
        let mut manager = PartyManager::new();

        let leader = test_steam_id(1);
        let party_id = manager.create_party(leader, "Leader", 5).unwrap();

        let party = manager.get_party_mut(party_id).unwrap();
        party.set_metadata("game_mode", "competitive");
        party.set_metadata("map", "de_dust2");

        assert_eq!(party.get_metadata("game_mode"), Some("competitive"));
        assert_eq!(party.get_metadata("map"), Some("de_dust2"));
    }

    #[test]
    fn cannot_join_while_in_party() {
        let mut manager = PartyManager::new();

        let leader1 = test_steam_id(1);
        let leader2 = test_steam_id(2);
        let member = test_steam_id(3);

        let party1 = manager.create_party(leader1, "Leader1", 5).unwrap();
        let party2 = manager.create_party(leader2, "Leader2", 5).unwrap();

        manager.send_invite(party1, member).unwrap();
        manager.accept_invite(member, "Member", party1).unwrap();

        manager.send_invite(party2, member).unwrap();
        // Accepting party2 should leave party1
        manager.accept_invite(member, "Member", party2).unwrap();

        assert!(!manager.get_party(party1).unwrap().is_member(member));
        assert!(manager.get_party(party2).unwrap().is_member(member));
    }
}
