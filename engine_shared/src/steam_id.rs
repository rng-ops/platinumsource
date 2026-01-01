//! Steam ID parsing, validation, and format conversion.
//!
//! # Valve Documentation Reference
//! - [SteamID](https://developer.valvesoftware.com/wiki/SteamID)
//! - [Steam Web API](https://partner.steamgames.com/doc/webapi_overview)
//!
//! # Steam ID Structure
//! A Steam ID is a 64-bit identifier that encodes:
//! - **Account ID** (32 bits): The unique account number
//! - **Instance** (20 bits): Desktop/Console/Web instance
//! - **Account Type** (4 bits): Individual, Multiseat, GameServer, etc.
//! - **Universe** (8 bits): Public, Beta, Internal, Dev
//!
//! ```text
//! 64-bit Steam ID Layout:
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Universe (8) │ Type (4) │ Instance (20) │ Account ID (32)       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Steam Universe identifiers.
/// 
/// Reference: <https://developer.valvesoftware.com/wiki/SteamID#Universes_Available_for_Steam_Accounts>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Universe {
    Invalid = 0,
    Public = 1,
    Beta = 2,
    Internal = 3,
    Dev = 4,
    // RC = 5, // Removed
}

impl Universe {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Universe::Invalid),
            1 => Some(Universe::Public),
            2 => Some(Universe::Beta),
            3 => Some(Universe::Internal),
            4 => Some(Universe::Dev),
            _ => None,
        }
    }
}

/// Steam Account Type identifiers.
/// 
/// Reference: <https://developer.valvesoftware.com/wiki/SteamID#Types_of_Steam_Accounts>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum AccountType {
    Invalid = 0,
    Individual = 1,
    Multiseat = 2,
    GameServer = 3,
    AnonGameServer = 4,
    Pending = 5,
    ContentServer = 6,
    Clan = 7,
    Chat = 8,
    ConsoleUser = 9,      // P2P SuperSeeder in some docs
    AnonUser = 10,
}

impl AccountType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(AccountType::Invalid),
            1 => Some(AccountType::Individual),
            2 => Some(AccountType::Multiseat),
            3 => Some(AccountType::GameServer),
            4 => Some(AccountType::AnonGameServer),
            5 => Some(AccountType::Pending),
            6 => Some(AccountType::ContentServer),
            7 => Some(AccountType::Clan),
            8 => Some(AccountType::Chat),
            9 => Some(AccountType::ConsoleUser),
            10 => Some(AccountType::AnonUser),
            _ => None,
        }
    }

    /// Returns the character code used in SteamID3 format.
    /// Reference: <https://developer.valvesoftware.com/wiki/SteamID#Steam_ID_as_a_Steam_Community_ID>
    pub fn type_char(&self) -> char {
        match self {
            AccountType::Invalid => 'I',
            AccountType::Individual => 'U',
            AccountType::Multiseat => 'M',
            AccountType::GameServer => 'G',
            AccountType::AnonGameServer => 'A',
            AccountType::Pending => 'P',
            AccountType::ContentServer => 'C',
            AccountType::Clan => 'g',
            AccountType::Chat => 'T', // or 'c' for clan chat, 'L' for lobby
            AccountType::ConsoleUser => 'U', // Treated as Individual
            AccountType::AnonUser => 'a',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'I' => Some(AccountType::Invalid),
            'U' => Some(AccountType::Individual),
            'M' => Some(AccountType::Multiseat),
            'G' => Some(AccountType::GameServer),
            'A' => Some(AccountType::AnonGameServer),
            'P' => Some(AccountType::Pending),
            'C' => Some(AccountType::ContentServer),
            'g' => Some(AccountType::Clan),
            'T' | 'c' | 'L' => Some(AccountType::Chat),
            'a' => Some(AccountType::AnonUser),
            _ => None,
        }
    }
}

/// Instance flags for Steam IDs.
/// 
/// Reference: <https://developer.valvesoftware.com/wiki/SteamID#Steam_ID_Instance>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Instance {
    All = 0,
    Desktop = 1,
    Console = 2,
    Web = 4,
}

/// A 64-bit Steam ID.
///
/// # Format Support
/// - **SteamID64**: `76561198012345678`
/// - **SteamID2 (Legacy)**: `STEAM_X:Y:Z` where X=universe, Y=lowest bit, Z=account/2
/// - **SteamID3**: `[U:1:12345678]` where U=type, 1=universe, number=account
///
/// # Examples
/// ```
/// use engine_shared::steam_id::SteamId;
///
/// let id = SteamId::from_u64(76561198012345678);
/// assert!(id.is_valid());
/// assert_eq!(id.account_id(), 52079950);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SteamId(u64);

impl SteamId {
    /// The nil/invalid Steam ID.
    pub const NIL: SteamId = SteamId(0);

    /// Create from raw 64-bit value.
    pub const fn from_u64(id: u64) -> Self {
        SteamId(id)
    }

    /// Get the raw 64-bit value.
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// Extract the 32-bit account ID.
    /// This is the lower 32 bits of the Steam ID.
    pub const fn account_id(&self) -> u32 {
        (self.0 & 0xFFFFFFFF) as u32
    }

    /// Extract the instance (bits 32-51, 20 bits).
    pub const fn instance(&self) -> u32 {
        ((self.0 >> 32) & 0xFFFFF) as u32
    }

    /// Extract the account type (bits 52-55, 4 bits).
    pub fn account_type(&self) -> AccountType {
        let t = ((self.0 >> 52) & 0xF) as u8;
        AccountType::from_u8(t).unwrap_or(AccountType::Invalid)
    }

    /// Extract the universe (bits 56-63, 8 bits).
    pub fn universe(&self) -> Universe {
        let u = ((self.0 >> 56) & 0xFF) as u8;
        Universe::from_u8(u).unwrap_or(Universe::Invalid)
    }

    /// Check if this is a valid, non-nil Steam ID.
    pub fn is_valid(&self) -> bool {
        self.0 != 0
            && self.account_type() != AccountType::Invalid
            && self.universe() != Universe::Invalid
    }

    /// Check if this represents an individual user account.
    pub fn is_individual(&self) -> bool {
        self.account_type() == AccountType::Individual
    }

    /// Check if this represents a game server.
    pub fn is_game_server(&self) -> bool {
        matches!(
            self.account_type(),
            AccountType::GameServer | AccountType::AnonGameServer
        )
    }

    /// Check if this represents a Steam group/clan.
    pub fn is_clan(&self) -> bool {
        self.account_type() == AccountType::Clan
    }

    /// Check if this represents a lobby.
    pub fn is_lobby(&self) -> bool {
        self.account_type() == AccountType::Chat && (self.instance() & 0x40000) != 0
    }

    /// Construct a Steam ID from components.
    pub fn from_parts(account_id: u32, instance: u32, account_type: AccountType, universe: Universe) -> Self {
        let mut id: u64 = 0;
        id |= account_id as u64;
        id |= ((instance & 0xFFFFF) as u64) << 32;
        id |= ((account_type as u8) as u64) << 52;
        id |= ((universe as u8) as u64) << 56;
        SteamId(id)
    }

    /// Create a standard individual user Steam ID.
    pub fn from_account_id(account_id: u32) -> Self {
        Self::from_parts(account_id, Instance::Desktop as u32, AccountType::Individual, Universe::Public)
    }

    /// Format as SteamID2 (legacy format): STEAM_X:Y:Z
    /// 
    /// Reference: <https://developer.valvesoftware.com/wiki/SteamID#As_Represented_in_Computer_Programs>
    pub fn to_steam2(&self) -> String {
        let y = self.account_id() & 1;
        let z = self.account_id() >> 1;
        // Note: Universe 1 (Public) is often represented as 0 in STEAM_X format for legacy reasons
        let x = if self.universe() == Universe::Public { 0 } else { self.universe() as u8 };
        format!("STEAM_{}:{}:{}", x, y, z)
    }

    /// Format as SteamID3: [T:U:A] or [T:U:A:I]
    /// 
    /// Reference: <https://developer.valvesoftware.com/wiki/SteamID#Steam_ID_as_a_Steam_Community_ID>
    pub fn to_steam3(&self) -> String {
        let t = self.account_type().type_char();
        let u = self.universe() as u8;
        let a = self.account_id();
        let i = self.instance();
        
        if i == Instance::Desktop as u32 || i == Instance::All as u32 {
            format!("[{}:{}:{}]", t, u, a)
        } else {
            format!("[{}:{}:{}:{}]", t, u, a, i)
        }
    }

    /// Parse from SteamID2 format: STEAM_X:Y:Z
    pub fn parse_steam2(s: &str) -> Option<Self> {
        let s = s.strip_prefix("STEAM_")?;
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return None;
        }
        
        let x: u8 = parts[0].parse().ok()?;
        let y: u32 = parts[1].parse().ok()?;
        let z: u32 = parts[2].parse().ok()?;
        
        if y > 1 {
            return None;
        }
        
        let account_id = z * 2 + y;
        let universe = if x == 0 { Universe::Public } else { Universe::from_u8(x)? };
        
        Some(Self::from_parts(account_id, Instance::Desktop as u32, AccountType::Individual, universe))
    }

    /// Parse from SteamID3 format: [T:U:A] or [T:U:A:I]
    pub fn parse_steam3(s: &str) -> Option<Self> {
        let s = s.strip_prefix('[')?.strip_suffix(']')?;
        let parts: Vec<&str> = s.split(':').collect();
        
        if parts.len() < 3 || parts.len() > 4 {
            return None;
        }
        
        let type_char = parts[0].chars().next()?;
        let account_type = AccountType::from_char(type_char)?;
        let universe_num: u8 = parts[1].parse().ok()?;
        let universe = Universe::from_u8(universe_num)?;
        let account_id: u32 = parts[2].parse().ok()?;
        let instance: u32 = if parts.len() == 4 {
            parts[3].parse().ok()?
        } else {
            Instance::Desktop as u32
        };
        
        Some(Self::from_parts(account_id, instance, account_type, universe))
    }
}

impl Default for SteamId {
    fn default() -> Self {
        Self::NIL
    }
}

impl fmt::Debug for SteamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SteamId({})", self.0)
    }
}

impl fmt::Display for SteamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SteamId {
    type Err = SteamIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        
        // Try SteamID64 (pure number)
        if let Ok(id) = s.parse::<u64>() {
            return Ok(SteamId::from_u64(id));
        }
        
        // Try SteamID2 format
        if s.starts_with("STEAM_") {
            return SteamId::parse_steam2(s).ok_or(SteamIdParseError::InvalidFormat);
        }
        
        // Try SteamID3 format
        if s.starts_with('[') && s.ends_with(']') {
            return SteamId::parse_steam3(s).ok_or(SteamIdParseError::InvalidFormat);
        }
        
        Err(SteamIdParseError::InvalidFormat)
    }
}

/// Error type for Steam ID parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteamIdParseError {
    InvalidFormat,
    InvalidUniverse,
    InvalidAccountType,
}

impl fmt::Display for SteamIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SteamIdParseError::InvalidFormat => write!(f, "invalid Steam ID format"),
            SteamIdParseError::InvalidUniverse => write!(f, "invalid universe value"),
            SteamIdParseError::InvalidAccountType => write!(f, "invalid account type"),
        }
    }
}

impl std::error::Error for SteamIdParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // SID-001: SteamID64 Parsing
    // Reference: https://developer.valvesoftware.com/wiki/SteamID
    // =============================================================================

    #[test]
    fn sid_001_steamid64_parsing() {
        // Test valid 64-bit Steam IDs
        let id = SteamId::from_u64(76561198012345678);
        assert!(id.is_valid());
        assert_eq!(id.as_u64(), 76561198012345678);
        
        // Parse from string
        let parsed: SteamId = "76561198012345678".parse().unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn sid_001_steamid64_components() {
        // Known Steam ID: 76561198012345678
        // Binary breakdown validates component extraction
        let id = SteamId::from_u64(76561198012345678);
        
        assert_eq!(id.account_id(), 52079950);
        assert_eq!(id.universe(), Universe::Public);
        assert_eq!(id.account_type(), AccountType::Individual);
        assert_eq!(id.instance(), 1); // Desktop
    }

    // =============================================================================
    // SID-002: SteamID32 Conversion
    // Reference: https://developer.valvesoftware.com/wiki/SteamID#Steam_ID_as_a_Steam_Community_ID
    // =============================================================================

    #[test]
    fn sid_002_account_id_extraction() {
        let id = SteamId::from_u64(76561198012345678);
        let account_id = id.account_id();
        
        // Account ID should be the lower 32 bits
        assert_eq!(account_id, 52079950);
        
        // Reconstruct from account ID
        let reconstructed = SteamId::from_account_id(account_id);
        assert_eq!(reconstructed.account_id(), account_id);
    }

    #[test]
    fn sid_002_roundtrip_conversion() {
        let original = SteamId::from_u64(76561198012345678);
        let account_id = original.account_id();
        let reconstructed = SteamId::from_account_id(account_id);
        
        // Account IDs should match
        assert_eq!(original.account_id(), reconstructed.account_id());
    }

    // =============================================================================
    // SID-003: SteamID3 Format
    // Reference: https://developer.valvesoftware.com/wiki/SteamID#Steam_ID_as_a_Steam_Community_ID
    // =============================================================================

    #[test]
    fn sid_003_steamid3_format_individual() {
        let id = SteamId::from_account_id(52079950);
        let steam3 = id.to_steam3();
        
        // Format: [U:1:ACCOUNTID]
        assert_eq!(steam3, "[U:1:52079950]");
        
        // Parse back
        let parsed = SteamId::parse_steam3(&steam3).unwrap();
        assert_eq!(parsed.account_id(), id.account_id());
    }

    #[test]
    fn sid_003_steamid3_format_with_instance() {
        // Create ID with non-default instance
        let id = SteamId::from_parts(12345, 2, AccountType::Individual, Universe::Public);
        let steam3 = id.to_steam3();
        
        // Should include instance
        assert_eq!(steam3, "[U:1:12345:2]");
        
        // Parse back
        let parsed = SteamId::parse_steam3(&steam3).unwrap();
        assert_eq!(parsed.instance(), 2);
    }

    #[test]
    fn sid_003_steamid3_account_types() {
        // Test various account types
        let test_cases = [
            (AccountType::Individual, 'U'),
            (AccountType::GameServer, 'G'),
            (AccountType::AnonGameServer, 'A'),
            (AccountType::Clan, 'g'),
        ];
        
        for (account_type, expected_char) in test_cases {
            let id = SteamId::from_parts(12345, 1, account_type, Universe::Public);
            let steam3 = id.to_steam3();
            assert!(steam3.starts_with(&format!("[{}:", expected_char)));
        }
    }

    // =============================================================================
    // SID-004: STEAM_X:Y:Z Format (Legacy)
    // Reference: https://developer.valvesoftware.com/wiki/SteamID#As_Represented_in_Computer_Programs
    // =============================================================================

    #[test]
    fn sid_004_steam2_format() {
        let id = SteamId::from_account_id(52079950);
        let steam2 = id.to_steam2();
        
        // STEAM_0:Y:Z where Y is lowest bit, Z is account/2
        // 52079950 = 26039975 * 2 + 0, so Y=0, Z=26039975
        assert_eq!(steam2, "STEAM_0:0:26039975");
    }

    #[test]
    fn sid_004_steam2_odd_account() {
        let id = SteamId::from_account_id(52079951); // Odd number
        let steam2 = id.to_steam2();
        
        // 52079951 = 26039975 * 2 + 1, so Y=1, Z=26039975
        assert_eq!(steam2, "STEAM_0:1:26039975");
    }

    #[test]
    fn sid_004_steam2_parse() {
        let parsed = SteamId::parse_steam2("STEAM_0:0:26039975").unwrap();
        assert_eq!(parsed.account_id(), 52079950);
        
        let parsed_odd = SteamId::parse_steam2("STEAM_0:1:26039975").unwrap();
        assert_eq!(parsed_odd.account_id(), 52079951);
    }

    #[test]
    fn sid_004_steam2_roundtrip() {
        let original = SteamId::from_account_id(52079950);
        let steam2 = original.to_steam2();
        let parsed = SteamId::parse_steam2(&steam2).unwrap();
        
        assert_eq!(original.account_id(), parsed.account_id());
    }

    // =============================================================================
    // SID-005: Account Type Detection
    // Reference: https://developer.valvesoftware.com/wiki/SteamID#Types_of_Steam_Accounts
    // =============================================================================

    #[test]
    fn sid_005_individual_account() {
        let id = SteamId::from_parts(12345, 1, AccountType::Individual, Universe::Public);
        assert!(id.is_individual());
        assert!(!id.is_game_server());
        assert!(!id.is_clan());
    }

    #[test]
    fn sid_005_game_server_account() {
        let gs = SteamId::from_parts(12345, 1, AccountType::GameServer, Universe::Public);
        assert!(gs.is_game_server());
        assert!(!gs.is_individual());
        
        let anon_gs = SteamId::from_parts(12345, 1, AccountType::AnonGameServer, Universe::Public);
        assert!(anon_gs.is_game_server());
    }

    #[test]
    fn sid_005_clan_account() {
        let clan = SteamId::from_parts(12345, 0, AccountType::Clan, Universe::Public);
        assert!(clan.is_clan());
        assert!(!clan.is_individual());
    }

    // =============================================================================
    // SID-006: Universe Detection
    // Reference: https://developer.valvesoftware.com/wiki/SteamID#Universes_Available_for_Steam_Accounts
    // =============================================================================

    #[test]
    fn sid_006_universe_public() {
        let id = SteamId::from_parts(12345, 1, AccountType::Individual, Universe::Public);
        assert_eq!(id.universe(), Universe::Public);
    }

    #[test]
    fn sid_006_universe_beta() {
        let id = SteamId::from_parts(12345, 1, AccountType::Individual, Universe::Beta);
        assert_eq!(id.universe(), Universe::Beta);
    }

    #[test]
    fn sid_006_all_universes() {
        for universe in [Universe::Invalid, Universe::Public, Universe::Beta, Universe::Internal, Universe::Dev] {
            let id = SteamId::from_parts(12345, 1, AccountType::Individual, universe);
            assert_eq!(id.universe(), universe);
        }
    }

    // =============================================================================
    // SID-007: Invalid SteamID Rejection
    // =============================================================================

    #[test]
    fn sid_007_nil_invalid() {
        assert!(!SteamId::NIL.is_valid());
        assert!(!SteamId::from_u64(0).is_valid());
    }

    #[test]
    fn sid_007_invalid_universe() {
        let id = SteamId::from_parts(12345, 1, AccountType::Individual, Universe::Invalid);
        assert!(!id.is_valid());
    }

    #[test]
    fn sid_007_invalid_type() {
        let id = SteamId::from_parts(12345, 1, AccountType::Invalid, Universe::Public);
        assert!(!id.is_valid());
    }

    #[test]
    fn sid_007_parse_invalid_format() {
        assert!("not_a_steam_id".parse::<SteamId>().is_err());
        assert!("STEAM_invalid".parse::<SteamId>().is_err());
        assert!("[X:1:123]".parse::<SteamId>().is_err()); // Invalid type char
        assert!("STEAM_0:2:123".parse::<SteamId>().is_err()); // Y must be 0 or 1
    }

    // =============================================================================
    // SID-008: Anonymous GameServer ID
    // Reference: https://developer.valvesoftware.com/wiki/SteamID
    // =============================================================================

    #[test]
    fn sid_008_anon_game_server() {
        let anon = SteamId::from_parts(0, 0, AccountType::AnonGameServer, Universe::Public);
        assert!(anon.is_game_server());
        assert_eq!(anon.account_type(), AccountType::AnonGameServer);
    }

    // =============================================================================
    // SID-009: Pending ID Handling
    // =============================================================================

    #[test]
    fn sid_009_pending_account() {
        let pending = SteamId::from_parts(0, 0, AccountType::Pending, Universe::Public);
        assert_eq!(pending.account_type(), AccountType::Pending);
        // Pending accounts are technically valid in the system
    }

    // =============================================================================
    // SID-010: SteamID Uniqueness (bit manipulation)
    // =============================================================================

    #[test]
    fn sid_010_component_uniqueness() {
        // Different components should produce different IDs
        let id1 = SteamId::from_parts(12345, 1, AccountType::Individual, Universe::Public);
        let id2 = SteamId::from_parts(12346, 1, AccountType::Individual, Universe::Public);
        let id3 = SteamId::from_parts(12345, 2, AccountType::Individual, Universe::Public);
        let id4 = SteamId::from_parts(12345, 1, AccountType::GameServer, Universe::Public);
        let id5 = SteamId::from_parts(12345, 1, AccountType::Individual, Universe::Beta);
        
        // All should be unique
        let ids = [id1, id2, id3, id4, id5];
        for (i, a) in ids.iter().enumerate() {
            for (j, b) in ids.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "IDs at {} and {} should be different", i, j);
                }
            }
        }
    }

    #[test]
    fn sid_010_bit_boundaries() {
        // Test max values for each component
        let max_account = SteamId::from_parts(u32::MAX, 1, AccountType::Individual, Universe::Public);
        assert_eq!(max_account.account_id(), u32::MAX);
        
        let max_instance = SteamId::from_parts(1, 0xFFFFF, AccountType::Individual, Universe::Public);
        assert_eq!(max_instance.instance(), 0xFFFFF);
    }
}
