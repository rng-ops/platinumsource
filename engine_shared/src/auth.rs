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

/// Simulated auth ticket generator for testing.
/// 
/// In production, this would interface with Steamworks SDK.
pub struct MockAuthProvider {
    next_handle: u32,
    app_id: u32,
}

impl MockAuthProvider {
    pub fn new(app_id: u32) -> Self {
        MockAuthProvider {
            next_handle: 1,
            app_id,
        }
    }

    /// Generate a mock auth ticket.
    pub fn get_auth_ticket(&mut self, owner: SteamId) -> AuthTicket {
        let handle = AuthTicketHandle::new(self.next_handle);
        self.next_handle += 1;

        // Generate deterministic ticket data based on owner
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(&self.app_id.to_le_bytes());
        data.extend_from_slice(&owner.as_u64().to_le_bytes());
        data.extend_from_slice(&handle.as_u32().to_le_bytes());
        // Pad to typical size
        data.resize(64, 0);

        AuthTicket::new(handle, data, owner, self.app_id)
    }

    /// Validate a ticket (mock implementation).
    pub fn validate_ticket(&self, ticket: &AuthTicket, expected_owner: SteamId) -> AuthSessionResponse {
        // Check size
        if !ticket.is_valid_size() {
            return AuthSessionResponse::AuthTicketInvalid;
        }

        // Check handle
        if !ticket.handle.is_valid() {
            return AuthSessionResponse::AuthTicketInvalid;
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
}
