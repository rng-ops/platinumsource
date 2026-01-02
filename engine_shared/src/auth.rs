//! Authentication ticket and session management.
//!
//! # Valve Documentation Reference
//! - [Steam Authentication](https://partner.steamgames.com/doc/features/auth)
//! - [ISteamUser Interface](https://partner.steamgames.com/doc/api/ISteamUser)
//! - [Session Tickets](https://partner.steamgames.com/doc/features/auth#session_tickets)
//!
//! # Authentication Flow
//! 1. Client calls `GetAuthSessionTicket()` to generate a ticket
//! 2. Client sends ticket to game server
//! 3. Server calls `BeginAuthSession()` to validate ticket
//! 4. Steam backend sends `ValidateAuthTicketResponse_t` callback
//! 5. Server accepts/rejects based on response
//!
//! # Ticket Lifetime
//! - Tickets are valid until cancelled or Steam disconnection
//! - Maximum ticket size: 1024 bytes
//! - Tickets are bound to the requesting SteamID

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::steam_id::SteamId;

/// Maximum size of an auth ticket in bytes.
/// Reference: <https://partner.steamgames.com/doc/api/ISteamUser#GetAuthSessionTicket>
pub const MAX_AUTH_TICKET_SIZE: usize = 1024;

/// Auth session response codes.
///
/// Reference: <https://partner.steamgames.com/doc/api/steam_api#EAuthSessionResponse>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum AuthSessionResponse {
    /// Ticket is valid.
    Ok = 0,
    /// User not connected to Steam.
    UserNotConnectedToSteam = 1,
    /// No license or expired license.
    NoLicenseOrExpired = 2,
    /// User is VAC banned.
    VACBanned = 3,
    /// User logged in elsewhere.
    LoggedInElsewhere = 4,
    /// VAC check timed out.
    VACCheckTimedOut = 5,
    /// Auth ticket was cancelled.
    AuthTicketCanceled = 6,
    /// Ticket already used (replay attack).
    AuthTicketInvalidAlreadyUsed = 7,
    /// Ticket is malformed.
    AuthTicketInvalid = 8,
    /// Publisher-issued ban.
    PublisherIssuedBan = 9,
}

impl AuthSessionResponse {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(AuthSessionResponse::Ok),
            1 => Some(AuthSessionResponse::UserNotConnectedToSteam),
            2 => Some(AuthSessionResponse::NoLicenseOrExpired),
            3 => Some(AuthSessionResponse::VACBanned),
            4 => Some(AuthSessionResponse::LoggedInElsewhere),
            5 => Some(AuthSessionResponse::VACCheckTimedOut),
            6 => Some(AuthSessionResponse::AuthTicketCanceled),
            7 => Some(AuthSessionResponse::AuthTicketInvalidAlreadyUsed),
            8 => Some(AuthSessionResponse::AuthTicketInvalid),
            9 => Some(AuthSessionResponse::PublisherIssuedBan),
            _ => None,
        }
    }

    /// Check if this response allows the connection.
    pub fn is_success(&self) -> bool {
        *self == AuthSessionResponse::Ok
    }

    /// Check if this is a permanent rejection (ban).
    pub fn is_permanent_rejection(&self) -> bool {
        matches!(
            self,
            AuthSessionResponse::VACBanned | AuthSessionResponse::PublisherIssuedBan
        )
    }

    /// Check if this is a temporary/recoverable issue.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            AuthSessionResponse::UserNotConnectedToSteam
                | AuthSessionResponse::VACCheckTimedOut
                | AuthSessionResponse::LoggedInElsewhere
        )
    }

    /// Get a human-readable error message.
    pub fn message(&self) -> &'static str {
        match self {
            AuthSessionResponse::Ok => "Authentication successful",
            AuthSessionResponse::UserNotConnectedToSteam => "Not connected to Steam",
            AuthSessionResponse::NoLicenseOrExpired => "No game license or license expired",
            AuthSessionResponse::VACBanned => "VAC banned from secure servers",
            AuthSessionResponse::LoggedInElsewhere => "Account logged in elsewhere",
            AuthSessionResponse::VACCheckTimedOut => "VAC check timed out, try again",
            AuthSessionResponse::AuthTicketCanceled => "Authentication ticket was cancelled",
            AuthSessionResponse::AuthTicketInvalidAlreadyUsed => "Ticket already used",
            AuthSessionResponse::AuthTicketInvalid => "Invalid authentication ticket",
            AuthSessionResponse::PublisherIssuedBan => "Banned by game publisher",
        }
    }
}

/// Handle for an active auth session ticket.
///
/// Reference: <https://partner.steamgames.com/doc/api/ISteamUser#HAuthTicket>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthTicketHandle(u32);

impl AuthTicketHandle {
    pub const INVALID: AuthTicketHandle = AuthTicketHandle(0);

    pub fn new(handle: u32) -> Self {
        AuthTicketHandle(handle)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// An authentication ticket for session validation.
///
/// This represents the ticket data returned by `GetAuthSessionTicket()`.
#[derive(Clone)]
pub struct AuthTicket {
    /// The ticket handle for cancellation.
    pub handle: AuthTicketHandle,
    /// Raw ticket data (max 1024 bytes).
    pub data: Vec<u8>,
    /// Steam ID of the ticket owner.
    pub owner: SteamId,
    /// When the ticket was created.
    pub created_at: Instant,
    /// App ID this ticket is for.
    pub app_id: u32,
}

impl AuthTicket {
    /// Create a new auth ticket.
    pub fn new(handle: AuthTicketHandle, data: Vec<u8>, owner: SteamId, app_id: u32) -> Self {
        AuthTicket {
            handle,
            data,
            owner,
            created_at: Instant::now(),
            app_id,
        }
    }

    /// Check if ticket data is within size limits.
    pub fn is_valid_size(&self) -> bool {
        !self.data.is_empty() && self.data.len() <= MAX_AUTH_TICKET_SIZE
    }

    /// Get ticket age.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Authentication session state for a connected client.
#[derive(Debug, Clone)]
pub struct AuthSession {
    /// The client's Steam ID.
    pub steam_id: SteamId,
    /// Current auth state.
    pub state: AuthSessionState,
    /// When the session started.
    pub started_at: Instant,
    /// Last validation response.
    pub last_response: Option<AuthSessionResponse>,
    /// Number of validation attempts.
    pub validation_attempts: u32,
}

/// State of an auth session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSessionState {
    /// Session not started.
    None,
    /// Waiting for validation callback.
    Pending,
    /// Successfully validated.
    Validated,
    /// Validation failed.
    Failed,
    /// Session was cancelled.
    Cancelled,
}

impl AuthSession {
    pub fn new(steam_id: SteamId) -> Self {
        AuthSession {
            steam_id,
            state: AuthSessionState::None,
            started_at: Instant::now(),
            last_response: None,
            validation_attempts: 0,
        }
    }

    /// Start validation (called after BeginAuthSession).
    pub fn begin_validation(&mut self) {
        self.state = AuthSessionState::Pending;
        self.validation_attempts += 1;
    }

    /// Process validation response.
    pub fn on_validation_response(&mut self, response: AuthSessionResponse) {
        self.last_response = Some(response);
        self.state = if response.is_success() {
            AuthSessionState::Validated
        } else {
            AuthSessionState::Failed
        };
    }

    /// Cancel the session.
    pub fn cancel(&mut self) {
        self.state = AuthSessionState::Cancelled;
    }

    /// Check if session is currently valid.
    pub fn is_valid(&self) -> bool {
        self.state == AuthSessionState::Validated
    }

    /// Check if session needs retry.
    pub fn should_retry(&self) -> bool {
        if let Some(response) = self.last_response {
            response.is_recoverable() && self.validation_attempts < 3
        } else {
            false
        }
    }
}

/// VAC ban status for a player.
///
/// Reference: <https://partner.steamgames.com/doc/features/anticheat>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VacBanStatus {
    /// Player has no VAC bans.
    Clean,
    /// Player is VAC banned.
    Banned,
    /// VAC check is pending.
    Pending,
    /// VAC check timed out.
    TimedOut,
}

/// Result of VAC validation.
#[derive(Debug, Clone)]
pub struct VacValidationResult {
    /// Whether the player is allowed on this server.
    pub allowed: bool,
    /// Auth response to send to client.
    pub response: AuthSessionResponse,
    /// Ban status if any.
    pub ban_status: VacBanStatus,
}

/// VAC module for server-side anti-cheat.
///
/// Reference: <https://partner.steamgames.com/doc/features/anticheat>
pub struct VacModule {
    /// Whether VAC is enabled (sv_vac_secure).
    enabled: bool,
    /// Initialized status.
    initialized: bool,
    /// Known ban statuses (in production, queried from Steam).
    ban_cache: std::collections::HashMap<SteamId, VacBanStatus>,
}

impl VacModule {
    /// Create a new VAC module.
    pub fn new(enabled: bool) -> Self {
        VacModule {
            enabled,
            initialized: true,
            ban_cache: std::collections::HashMap::new(),
        }
    }

    /// Check if VAC is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if VAC module is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if this is a secure server.
    pub fn is_secure_server(&self) -> bool {
        self.enabled && self.initialized
    }

    /// Get ban status for a player.
    pub fn get_ban_status(&self, steam_id: SteamId) -> VacBanStatus {
        self.ban_cache
            .get(&steam_id)
            .copied()
            .unwrap_or(VacBanStatus::Clean)
    }

    /// Add or update a ban status (for testing/simulation).
    pub fn add_ban(&mut self, steam_id: SteamId, status: VacBanStatus) {
        self.ban_cache.insert(steam_id, status);
    }

    /// Validate a player for connection.
    pub fn validate_player(&self, steam_id: SteamId) -> VacValidationResult {
        let ban_status = self.get_ban_status(steam_id);

        // Insecure servers allow everyone
        if !self.enabled {
            return VacValidationResult {
                allowed: true,
                response: AuthSessionResponse::Ok,
                ban_status,
            };
        }

        match ban_status {
            VacBanStatus::Clean => VacValidationResult {
                allowed: true,
                response: AuthSessionResponse::Ok,
                ban_status,
            },
            VacBanStatus::Banned => VacValidationResult {
                allowed: false,
                response: AuthSessionResponse::VACBanned,
                ban_status,
            },
            VacBanStatus::Pending => VacValidationResult {
                allowed: true, // Allow while pending
                response: AuthSessionResponse::Ok,
                ban_status,
            },
            VacBanStatus::TimedOut => VacValidationResult {
                allowed: false,
                response: AuthSessionResponse::VACCheckTimedOut,
                ban_status,
            },
        }
    }

    /// Clear ban cache.
    pub fn clear_cache(&mut self) {
        self.ban_cache.clear();
    }
}

/// Simulated auth ticket generator for testing.
///
/// In production, this would interface with Steamworks SDK.
pub struct MockAuthProvider {
    next_handle: u32,
    app_id: u32,
    /// Active tickets (handle -> is_valid).
    active_tickets: std::collections::HashMap<u32, bool>,
}

impl MockAuthProvider {
    pub fn new(app_id: u32) -> Self {
        MockAuthProvider {
            next_handle: 1,
            app_id,
            active_tickets: std::collections::HashMap::new(),
        }
    }

    /// Generate a mock auth ticket.
    pub fn get_auth_ticket(&mut self, owner: SteamId) -> AuthTicket {
        let handle = AuthTicketHandle::new(self.next_handle);
        self.next_handle += 1;

        // Track active ticket
        self.active_tickets.insert(handle.as_u32(), true);

        // Generate deterministic ticket data based on owner
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(&self.app_id.to_le_bytes());
        data.extend_from_slice(&owner.as_u64().to_le_bytes());
        data.extend_from_slice(&handle.as_u32().to_le_bytes());
        // Pad to typical size
        data.resize(64, 0);

        AuthTicket::new(handle, data, owner, self.app_id)
    }

    /// Cancel a ticket by handle.
    pub fn cancel_ticket(&mut self, handle: AuthTicketHandle) {
        if let Some(valid) = self.active_tickets.get_mut(&handle.as_u32()) {
            *valid = false;
        }
    }

    /// Check if a ticket handle is still valid (not cancelled).
    pub fn is_ticket_valid(&self, handle: AuthTicketHandle) -> bool {
        self.active_tickets
            .get(&handle.as_u32())
            .copied()
            .unwrap_or(false)
    }

    /// Validate a ticket by handle only.
    pub fn validate_ticket_by_handle(&self, handle: AuthTicketHandle) -> AuthSessionResponse {
        if !handle.is_valid() {
            return AuthSessionResponse::AuthTicketInvalid;
        }

        match self.active_tickets.get(&handle.as_u32()) {
            Some(true) => AuthSessionResponse::Ok,
            Some(false) => AuthSessionResponse::AuthTicketCanceled,
            None => AuthSessionResponse::AuthTicketInvalid,
        }
    }

    /// Validate a ticket (mock implementation).
    pub fn validate_ticket(
        &self,
        ticket: &AuthTicket,
        expected_owner: SteamId,
    ) -> AuthSessionResponse {
        // Check size
        if !ticket.is_valid_size() {
            return AuthSessionResponse::AuthTicketInvalid;
        }

        // Check handle
        if !ticket.handle.is_valid() {
            return AuthSessionResponse::AuthTicketInvalid;
        }

        // Check if ticket was cancelled
        if !self.is_ticket_valid(ticket.handle) {
            return AuthSessionResponse::AuthTicketCanceled;
        }

        // Check owner matches
        if ticket.owner != expected_owner {
            return AuthSessionResponse::AuthTicketInvalid;
        }

        // Check app ID (would be embedded in real ticket)
        if ticket.app_id != self.app_id {
            return AuthSessionResponse::NoLicenseOrExpired;
        }

        AuthSessionResponse::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // AUTH-001: Valid Steam Login
    // Reference: https://partner.steamgames.com/doc/features/auth
    // =============================================================================

    #[test]
    fn auth_001_valid_ticket_generation() {
        let mut provider = MockAuthProvider::new(730); // CS:GO app ID
        let steam_id = SteamId::from_account_id(12345);

        let ticket = provider.get_auth_ticket(steam_id);

        assert!(ticket.handle.is_valid());
        assert!(ticket.is_valid_size());
        assert_eq!(ticket.owner, steam_id);
        assert_eq!(ticket.app_id, 730);
    }

    #[test]
    fn auth_001_ticket_contains_owner_info() {
        let mut provider = MockAuthProvider::new(730);
        let steam_id = SteamId::from_account_id(12345);

        let ticket = provider.get_auth_ticket(steam_id);

        // Ticket data should embed the Steam ID
        let embedded_id = u64::from_le_bytes(ticket.data[4..12].try_into().unwrap());
        assert_eq!(embedded_id, steam_id.as_u64());
    }

    // =============================================================================
    // AUTH-002: Invalid Credentials Rejection
    // Reference: https://partner.steamgames.com/doc/api/steam_api#EAuthSessionResponse
    // =============================================================================

    #[test]
    fn auth_002_wrong_owner_rejected() {
        let mut provider = MockAuthProvider::new(730);
        let real_owner = SteamId::from_account_id(12345);
        let fake_owner = SteamId::from_account_id(99999);

        let ticket = provider.get_auth_ticket(real_owner);

        // Validate with wrong owner
        let response = provider.validate_ticket(&ticket, fake_owner);
        assert_eq!(response, AuthSessionResponse::AuthTicketInvalid);
    }

    #[test]
    fn auth_002_invalid_handle_rejected() {
        let provider = MockAuthProvider::new(730);
        let steam_id = SteamId::from_account_id(12345);

        let ticket = AuthTicket {
            handle: AuthTicketHandle::INVALID,
            data: vec![0; 64],
            owner: steam_id,
            created_at: Instant::now(),
            app_id: 730,
        };

        let response = provider.validate_ticket(&ticket, steam_id);
        assert_eq!(response, AuthSessionResponse::AuthTicketInvalid);
    }

    // =============================================================================
    // AUTH-004: Auth Ticket Generation
    // Reference: https://partner.steamgames.com/doc/api/ISteamUser#GetAuthSessionTicket
    // =============================================================================

    #[test]
    fn auth_004_ticket_size_within_limits() {
        let mut provider = MockAuthProvider::new(730);
        let steam_id = SteamId::from_account_id(12345);

        let ticket = provider.get_auth_ticket(steam_id);

        assert!(ticket.data.len() <= MAX_AUTH_TICKET_SIZE);
        assert!(!ticket.data.is_empty());
    }

    #[test]
    fn auth_004_unique_handles() {
        let mut provider = MockAuthProvider::new(730);
        let steam_id = SteamId::from_account_id(12345);

        let ticket1 = provider.get_auth_ticket(steam_id);
        let ticket2 = provider.get_auth_ticket(steam_id);

        assert_ne!(ticket1.handle.as_u32(), ticket2.handle.as_u32());
    }

    // =============================================================================
    // AUTH-005: Auth Ticket Validation
    // Reference: https://partner.steamgames.com/doc/api/ISteamUser#BeginAuthSession
    // =============================================================================

    #[test]
    fn auth_005_valid_ticket_accepted() {
        let mut provider = MockAuthProvider::new(730);
        let steam_id = SteamId::from_account_id(12345);

        let ticket = provider.get_auth_ticket(steam_id);
        let response = provider.validate_ticket(&ticket, steam_id);

        assert_eq!(response, AuthSessionResponse::Ok);
        assert!(response.is_success());
    }

    #[test]
    fn auth_005_wrong_app_rejected() {
        let mut provider = MockAuthProvider::new(730);
        let steam_id = SteamId::from_account_id(12345);

        let mut ticket = provider.get_auth_ticket(steam_id);
        ticket.app_id = 440; // Wrong app ID

        let response = provider.validate_ticket(&ticket, steam_id);
        assert_eq!(response, AuthSessionResponse::NoLicenseOrExpired);
    }

    // =============================================================================
    // AUTH-007: Auth Timeout Handling
    // =============================================================================

    #[test]
    fn auth_007_session_timeout_recovery() {
        let steam_id = SteamId::from_account_id(12345);
        let mut session = AuthSession::new(steam_id);

        session.begin_validation();
        session.on_validation_response(AuthSessionResponse::VACCheckTimedOut);

        assert!(session.should_retry());
        assert!(session.last_response.unwrap().is_recoverable());
    }

    #[test]
    fn auth_007_max_retry_limit() {
        let steam_id = SteamId::from_account_id(12345);
        let mut session = AuthSession::new(steam_id);

        // Simulate 3 failed attempts
        for _ in 0..3 {
            session.begin_validation();
            session.on_validation_response(AuthSessionResponse::VACCheckTimedOut);
        }

        assert!(!session.should_retry()); // No more retries after 3
    }

    // =============================================================================
    // AUTH-009: Auth Callback Processing
    // Reference: https://partner.steamgames.com/doc/api/ISteamUser#ValidateAuthTicketResponse_t
    // =============================================================================

    #[test]
    fn auth_009_response_state_transitions() {
        let steam_id = SteamId::from_account_id(12345);
        let mut session = AuthSession::new(steam_id);

        assert_eq!(session.state, AuthSessionState::None);

        session.begin_validation();
        assert_eq!(session.state, AuthSessionState::Pending);

        session.on_validation_response(AuthSessionResponse::Ok);
        assert_eq!(session.state, AuthSessionState::Validated);
        assert!(session.is_valid());
    }

    #[test]
    fn auth_009_failed_response_handling() {
        let steam_id = SteamId::from_account_id(12345);
        let mut session = AuthSession::new(steam_id);

        session.begin_validation();
        session.on_validation_response(AuthSessionResponse::VACBanned);

        assert_eq!(session.state, AuthSessionState::Failed);
        assert!(!session.is_valid());
        assert!(session.last_response.unwrap().is_permanent_rejection());
    }

    // =============================================================================
    // Response Code Coverage
    // =============================================================================

    #[test]
    fn response_code_messages() {
        // Ensure all response codes have meaningful messages
        let responses = [
            AuthSessionResponse::Ok,
            AuthSessionResponse::UserNotConnectedToSteam,
            AuthSessionResponse::NoLicenseOrExpired,
            AuthSessionResponse::VACBanned,
            AuthSessionResponse::LoggedInElsewhere,
            AuthSessionResponse::VACCheckTimedOut,
            AuthSessionResponse::AuthTicketCanceled,
            AuthSessionResponse::AuthTicketInvalidAlreadyUsed,
            AuthSessionResponse::AuthTicketInvalid,
            AuthSessionResponse::PublisherIssuedBan,
        ];

        for response in responses {
            assert!(!response.message().is_empty());
        }
    }

    #[test]
    fn response_code_classification() {
        assert!(AuthSessionResponse::Ok.is_success());
        assert!(!AuthSessionResponse::VACBanned.is_success());

        assert!(AuthSessionResponse::VACBanned.is_permanent_rejection());
        assert!(AuthSessionResponse::PublisherIssuedBan.is_permanent_rejection());
        assert!(!AuthSessionResponse::VACCheckTimedOut.is_permanent_rejection());

        assert!(AuthSessionResponse::VACCheckTimedOut.is_recoverable());
        assert!(AuthSessionResponse::LoggedInElsewhere.is_recoverable());
        assert!(!AuthSessionResponse::VACBanned.is_recoverable());
    }

    // =============================================================================
    // VAC-001: VAC Module Load
    // Reference: https://partner.steamgames.com/doc/features/anticheat
    // =============================================================================

    #[test]
    fn vac_001_module_initialization() {
        let vac = VacModule::new(true);
        assert!(vac.is_enabled());
        assert!(vac.is_initialized());
    }

    #[test]
    fn vac_001_disabled_module() {
        let vac = VacModule::new(false);
        assert!(!vac.is_enabled());
    }

    // =============================================================================
    // VAC-002: VAC Secure Mode Flag
    // =============================================================================

    #[test]
    fn vac_002_secure_mode_flag() {
        let vac = VacModule::new(true);
        assert!(vac.is_secure_server());

        let insecure = VacModule::new(false);
        assert!(!insecure.is_secure_server());
    }

    // =============================================================================
    // VAC-003: VAC Ban Check
    // =============================================================================

    #[test]
    fn vac_003_ban_status_query() {
        let mut vac = VacModule::new(true);
        let player = SteamId::from_account_id(12345);

        // Player not banned by default
        assert_eq!(vac.get_ban_status(player), VacBanStatus::Clean);

        // Add a ban
        vac.add_ban(player, VacBanStatus::Banned);
        assert_eq!(vac.get_ban_status(player), VacBanStatus::Banned);
    }

    // =============================================================================
    // VAC-004: VAC Ban Rejection
    // =============================================================================

    #[test]
    fn vac_004_banned_player_rejected() {
        let mut vac = VacModule::new(true);
        let player = SteamId::from_account_id(12345);

        vac.add_ban(player, VacBanStatus::Banned);
        let result = vac.validate_player(player);

        assert!(!result.allowed);
        assert_eq!(result.response, AuthSessionResponse::VACBanned);
    }

    #[test]
    fn vac_004_clean_player_accepted() {
        let vac = VacModule::new(true);
        let player = SteamId::from_account_id(12345);

        let result = vac.validate_player(player);
        assert!(result.allowed);
        assert_eq!(result.response, AuthSessionResponse::Ok);
    }

    // =============================================================================
    // VAC-005: Insecure Server Bypass
    // =============================================================================

    #[test]
    fn vac_005_insecure_allows_banned() {
        let mut vac = VacModule::new(false); // Insecure
        let player = SteamId::from_account_id(12345);

        vac.add_ban(player, VacBanStatus::Banned);
        let result = vac.validate_player(player);

        // Insecure server allows banned players
        assert!(result.allowed);
    }

    // =============================================================================
    // VAC-010: Real-time VAC Status
    // =============================================================================

    #[test]
    fn vac_010_mid_session_ban() {
        let mut vac = VacModule::new(true);
        let player = SteamId::from_account_id(12345);

        // Initial check - clean
        let initial = vac.validate_player(player);
        assert!(initial.allowed);

        // Ban is added during session
        vac.add_ban(player, VacBanStatus::Banned);

        // Re-check should detect ban
        let recheck = vac.validate_player(player);
        assert!(!recheck.allowed);
    }

    // =============================================================================
    // TKT-002: Ticket Size Bounds
    // =============================================================================

    #[test]
    fn tkt_002_empty_ticket_invalid() {
        let steam_id = SteamId::from_account_id(12345);
        let ticket = AuthTicket {
            handle: AuthTicketHandle::new(1),
            data: vec![], // Empty
            owner: steam_id,
            created_at: Instant::now(),
            app_id: 730,
        };
        assert!(!ticket.is_valid_size());
    }

    #[test]
    fn tkt_002_oversized_ticket_invalid() {
        let steam_id = SteamId::from_account_id(12345);
        let ticket = AuthTicket {
            handle: AuthTicketHandle::new(1),
            data: vec![0; MAX_AUTH_TICKET_SIZE + 1], // Too large
            owner: steam_id,
            created_at: Instant::now(),
            app_id: 730,
        };
        assert!(!ticket.is_valid_size());
    }

    #[test]
    fn tkt_002_max_size_valid() {
        let steam_id = SteamId::from_account_id(12345);
        let ticket = AuthTicket {
            handle: AuthTicketHandle::new(1),
            data: vec![0; MAX_AUTH_TICKET_SIZE], // Exactly max
            owner: steam_id,
            created_at: Instant::now(),
            app_id: 730,
        };
        assert!(ticket.is_valid_size());
    }

    // =============================================================================
    // TKT-005: Ticket Cancellation
    // =============================================================================

    #[test]
    fn tkt_005_ticket_cancellation() {
        let mut provider = MockAuthProvider::new(730);
        let steam_id = SteamId::from_account_id(12345);

        let ticket = provider.get_auth_ticket(steam_id);
        let handle = ticket.handle;

        // Ticket should be valid initially
        assert!(handle.is_valid());

        // Cancel the ticket
        provider.cancel_ticket(handle);

        // Cancelled tickets should fail validation
        let result = provider.validate_ticket_by_handle(handle);
        assert_eq!(result, AuthSessionResponse::AuthTicketCanceled);
    }

    // =============================================================================
    // TKT-007: Multiple Tickets
    // =============================================================================

    #[test]
    fn tkt_007_multiple_active_tickets() {
        let mut provider = MockAuthProvider::new(730);
        let steam_id = SteamId::from_account_id(12345);

        let ticket1 = provider.get_auth_ticket(steam_id);
        let ticket2 = provider.get_auth_ticket(steam_id);
        let ticket3 = provider.get_auth_ticket(steam_id);

        // All tickets should be valid and unique
        assert!(ticket1.handle.is_valid());
        assert!(ticket2.handle.is_valid());
        assert!(ticket3.handle.is_valid());

        assert_ne!(ticket1.handle, ticket2.handle);
        assert_ne!(ticket2.handle, ticket3.handle);

        // All should validate successfully
        assert_eq!(provider.validate_ticket(&ticket1, steam_id), AuthSessionResponse::Ok);
        assert_eq!(provider.validate_ticket(&ticket2, steam_id), AuthSessionResponse::Ok);
        assert_eq!(provider.validate_ticket(&ticket3, steam_id), AuthSessionResponse::Ok);
    }
}
