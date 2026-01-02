//! DLC and entitlement verification.
//!
//! # Valve Documentation Reference
//! - [Steam DLC](https://partner.steamgames.com/doc/store/application/dlc)
//! - [ISteamApps Interface](https://partner.steamgames.com/doc/api/ISteamApps)
//!
//! # Features
//! - DLC ownership verification
//! - License type detection
//! - Family sharing detection
//! - Free weekend handling

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::steam_id::SteamId;

/// App ID type.
pub type AppId = u32;

/// DLC information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlcInfo {
    /// DLC App ID.
    pub app_id: AppId,
    /// DLC name.
    pub name: String,
    /// Whether it's available (owned or free trial).
    pub available: bool,
    /// Whether it's installed.
    pub installed: bool,
}

/// License type for ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseType {
    /// Permanent ownership.
    Permanent,
    /// Temporary (free weekend, etc.).
    Temporary,
    /// Family sharing.
    FamilySharing,
    /// Free-to-play.
    FreeToPlay,
    /// No license.
    None,
}

/// Free weekend status.
#[derive(Debug, Clone)]
pub struct FreeWeekendInfo {
    /// Whether free weekend is active.
    pub active: bool,
    /// When the free weekend ends.
    pub ends_at: Option<Instant>,
    /// Time remaining.
    pub time_remaining: Option<Duration>,
}

/// Game ban information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameBan {
    /// App ID the ban is for.
    pub app_id: AppId,
    /// When the ban was issued.
    pub issued_at: u64,
    /// Whether it's a permanent ban.
    pub permanent: bool,
    /// Ban reason (if available).
    pub reason: Option<String>,
}

/// Entitlement manager for DLC and license verification.
///
/// In production, this would interface with Steamworks SDK.
pub struct EntitlementManager {
    /// Base app ID.
    base_app_id: AppId,
    /// Owned DLC.
    owned_dlc: HashSet<AppId>,
    /// Installed DLC.
    installed_dlc: HashSet<AppId>,
    /// DLC metadata.
    dlc_info: HashMap<AppId, DlcInfo>,
    /// License types.
    licenses: HashMap<AppId, LicenseType>,
    /// Family sharing lender.
    family_sharing_lender: Option<SteamId>,
    /// Free weekend status.
    free_weekend_active: bool,
    /// Game bans.
    game_bans: Vec<GameBan>,
    /// Install directories.
    install_dirs: HashMap<AppId, String>,
}

impl EntitlementManager {
    /// Create a new entitlement manager.
    pub fn new(base_app_id: AppId) -> Self {
        EntitlementManager {
            base_app_id,
            owned_dlc: HashSet::new(),
            installed_dlc: HashSet::new(),
            dlc_info: HashMap::new(),
            licenses: HashMap::new(),
            family_sharing_lender: None,
            free_weekend_active: false,
            game_bans: Vec::new(),
            install_dirs: HashMap::new(),
        }
    }

    // =========================================================================
    // DLC Methods
    // =========================================================================

    /// Check if DLC is owned.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamApps#BIsDlcInstalled>
    pub fn is_dlc_installed(&self, dlc_app_id: AppId) -> bool {
        self.installed_dlc.contains(&dlc_app_id)
    }

    /// Get DLC count.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamApps#GetDLCCount>
    pub fn get_dlc_count(&self) -> usize {
        self.dlc_info.len()
    }

    /// Get DLC data by index.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamApps#BGetDLCDataByIndex>
    pub fn get_dlc_data_by_index(&self, index: usize) -> Option<&DlcInfo> {
        self.dlc_info.values().nth(index)
    }

    /// Get DLC info by app ID.
    pub fn get_dlc_info(&self, dlc_app_id: AppId) -> Option<&DlcInfo> {
        self.dlc_info.get(&dlc_app_id)
    }

    /// Install DLC.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamApps#InstallDLC>
    pub fn install_dlc(&mut self, dlc_app_id: AppId) -> bool {
        if !self.owned_dlc.contains(&dlc_app_id) {
            return false;
        }
        self.installed_dlc.insert(dlc_app_id);
        if let Some(info) = self.dlc_info.get_mut(&dlc_app_id) {
            info.installed = true;
        }
        true
    }

    /// Uninstall DLC.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamApps#UninstallDLC>
    pub fn uninstall_dlc(&mut self, dlc_app_id: AppId) {
        self.installed_dlc.remove(&dlc_app_id);
        if let Some(info) = self.dlc_info.get_mut(&dlc_app_id) {
            info.installed = false;
        }
    }

    /// Add DLC to catalog (for testing).
    pub fn add_dlc(&mut self, app_id: AppId, name: &str, owned: bool) {
        let installed = owned && self.installed_dlc.contains(&app_id);
        self.dlc_info.insert(
            app_id,
            DlcInfo {
                app_id,
                name: name.to_string(),
                available: owned,
                installed,
            },
        );
        if owned {
            self.owned_dlc.insert(app_id);
        }
    }

    // =========================================================================
    // License Methods
    // =========================================================================

    /// Check if app is subscribed (owned).
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamApps#BIsSubscribedApp>
    pub fn is_subscribed_app(&self, app_id: AppId) -> bool {
        self.licenses
            .get(&app_id)
            .map(|l| *l != LicenseType::None)
            .unwrap_or(false)
    }

    /// Get license type for an app.
    pub fn get_license_type(&self, app_id: AppId) -> LicenseType {
        self.licenses.get(&app_id).copied().unwrap_or(LicenseType::None)
    }

    /// Set license for an app (for testing).
    pub fn set_license(&mut self, app_id: AppId, license_type: LicenseType) {
        self.licenses.insert(app_id, license_type);
    }

    /// Check if from free weekend.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamApps#BIsSubscribedFromFreeWeekend>
    pub fn is_subscribed_from_free_weekend(&self) -> bool {
        self.free_weekend_active
            && self
                .licenses
                .get(&self.base_app_id)
                .map(|l| *l == LicenseType::Temporary)
                .unwrap_or(false)
    }

    /// Set free weekend status (for testing).
    pub fn set_free_weekend(&mut self, active: bool) {
        self.free_weekend_active = active;
        if active {
            self.licenses.insert(self.base_app_id, LicenseType::Temporary);
        }
    }

    /// Check if from family sharing.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamApps#BIsSubscribedFromFamilySharing>
    pub fn is_subscribed_from_family_sharing(&self) -> bool {
        self.family_sharing_lender.is_some()
    }

    /// Get family sharing lender.
    pub fn get_family_sharing_lender(&self) -> Option<SteamId> {
        self.family_sharing_lender
    }

    /// Set family sharing lender (for testing).
    pub fn set_family_sharing(&mut self, lender: Option<SteamId>) {
        self.family_sharing_lender = lender;
        if lender.is_some() {
            self.licenses
                .insert(self.base_app_id, LicenseType::FamilySharing);
        }
    }

    // =========================================================================
    // Install Directory Methods
    // =========================================================================

    /// Get app install directory.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamApps#GetAppInstallDir>
    pub fn get_app_install_dir(&self, app_id: AppId) -> Option<&str> {
        self.install_dirs.get(&app_id).map(|s| s.as_str())
    }

    /// Set install directory (for testing).
    pub fn set_install_dir(&mut self, app_id: AppId, path: &str) {
        self.install_dirs.insert(app_id, path.to_string());
    }

    // =========================================================================
    // Game Ban Methods
    // =========================================================================

    /// Check if player has game ban.
    pub fn has_game_ban(&self) -> bool {
        !self.game_bans.is_empty()
    }

    /// Get game bans.
    pub fn get_game_bans(&self) -> &[GameBan] {
        &self.game_bans
    }

    /// Add game ban (for testing).
    pub fn add_game_ban(&mut self, ban: GameBan) {
        self.game_bans.push(ban);
    }

    /// Check if banned from specific app.
    pub fn is_banned_from_app(&self, app_id: AppId) -> bool {
        self.game_bans.iter().any(|b| b.app_id == app_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // DLC-001: Check DLC Owned
    // Reference: https://partner.steamgames.com/doc/api/ISteamApps#BIsDlcInstalled
    // =============================================================================

    #[test]
    fn dlc_001_check_dlc_owned() {
        let mut manager = EntitlementManager::new(730);

        // DLC not owned
        assert!(!manager.is_dlc_installed(1234));

        // Add and install DLC
        manager.add_dlc(1234, "Test DLC", true);
        manager.install_dlc(1234);

        assert!(manager.is_dlc_installed(1234));
    }

    // =============================================================================
    // DLC-002: Get DLC Count
    // Reference: https://partner.steamgames.com/doc/api/ISteamApps#GetDLCCount
    // =============================================================================

    #[test]
    fn dlc_002_get_dlc_count() {
        let mut manager = EntitlementManager::new(730);

        assert_eq!(manager.get_dlc_count(), 0);

        manager.add_dlc(1001, "DLC 1", true);
        manager.add_dlc(1002, "DLC 2", true);
        manager.add_dlc(1003, "DLC 3", false);

        assert_eq!(manager.get_dlc_count(), 3);
    }

    // =============================================================================
    // DLC-003: Get DLC Data
    // Reference: https://partner.steamgames.com/doc/api/ISteamApps#BGetDLCDataByIndex
    // =============================================================================

    #[test]
    fn dlc_003_get_dlc_data() {
        let mut manager = EntitlementManager::new(730);

        manager.add_dlc(1001, "Operation Bravo", true);

        let info = manager.get_dlc_info(1001).unwrap();
        assert_eq!(info.app_id, 1001);
        assert_eq!(info.name, "Operation Bravo");
        assert!(info.available);
    }

    // =============================================================================
    // DLC-004: Install DLC
    // Reference: https://partner.steamgames.com/doc/api/ISteamApps#InstallDLC
    // =============================================================================

    #[test]
    fn dlc_004_install_dlc() {
        let mut manager = EntitlementManager::new(730);

        manager.add_dlc(1001, "Test DLC", true);
        assert!(!manager.is_dlc_installed(1001));

        let result = manager.install_dlc(1001);
        assert!(result);
        assert!(manager.is_dlc_installed(1001));
    }

    #[test]
    fn dlc_004_install_unowned_fails() {
        let mut manager = EntitlementManager::new(730);

        manager.add_dlc(1001, "Test DLC", false); // Not owned

        let result = manager.install_dlc(1001);
        assert!(!result);
        assert!(!manager.is_dlc_installed(1001));
    }

    // =============================================================================
    // DLC-005: Uninstall DLC
    // Reference: https://partner.steamgames.com/doc/api/ISteamApps#UninstallDLC
    // =============================================================================

    #[test]
    fn dlc_005_uninstall_dlc() {
        let mut manager = EntitlementManager::new(730);

        manager.add_dlc(1001, "Test DLC", true);
        manager.install_dlc(1001);
        assert!(manager.is_dlc_installed(1001));

        manager.uninstall_dlc(1001);
        assert!(!manager.is_dlc_installed(1001));
    }

    // =============================================================================
    // DLC-007: Early Access / Free Weekend
    // Reference: https://partner.steamgames.com/doc/api/ISteamApps#BIsSubscribedFromFreeWeekend
    // =============================================================================

    #[test]
    fn dlc_007_free_weekend() {
        let mut manager = EntitlementManager::new(730);

        assert!(!manager.is_subscribed_from_free_weekend());

        manager.set_free_weekend(true);
        assert!(manager.is_subscribed_from_free_weekend());

        manager.set_free_weekend(false);
        assert!(!manager.is_subscribed_from_free_weekend());
    }

    // =============================================================================
    // DLC-008: App ID Chains / Install Dir
    // Reference: https://partner.steamgames.com/doc/api/ISteamApps#GetAppInstallDir
    // =============================================================================

    #[test]
    fn dlc_008_install_dir() {
        let mut manager = EntitlementManager::new(730);

        manager.set_install_dir(730, "/home/user/.steam/steamapps/common/Counter-Strike 2");
        manager.set_install_dir(1001, "/home/user/.steam/steamapps/common/CS2-DLC");

        assert_eq!(
            manager.get_app_install_dir(730),
            Some("/home/user/.steam/steamapps/common/Counter-Strike 2")
        );
        assert_eq!(
            manager.get_app_install_dir(1001),
            Some("/home/user/.steam/steamapps/common/CS2-DLC")
        );
    }

    // =============================================================================
    // LIC-001: Game Ownership
    // Reference: https://partner.steamgames.com/doc/api/ISteamApps#BIsSubscribedApp
    // =============================================================================

    #[test]
    fn lic_001_game_ownership() {
        let mut manager = EntitlementManager::new(730);

        assert!(!manager.is_subscribed_app(730));

        manager.set_license(730, LicenseType::Permanent);
        assert!(manager.is_subscribed_app(730));
    }

    // =============================================================================
    // LIC-002: License Type
    // =============================================================================

    #[test]
    fn lic_002_license_type() {
        let mut manager = EntitlementManager::new(730);

        assert_eq!(manager.get_license_type(730), LicenseType::None);

        manager.set_license(730, LicenseType::Permanent);
        assert_eq!(manager.get_license_type(730), LicenseType::Permanent);

        manager.set_license(730, LicenseType::Temporary);
        assert_eq!(manager.get_license_type(730), LicenseType::Temporary);
    }

    // =============================================================================
    // LIC-003: Family Sharing
    // Reference: https://partner.steamgames.com/doc/api/ISteamApps#BIsSubscribedFromFamilySharing
    // =============================================================================

    #[test]
    fn lic_003_family_sharing() {
        let mut manager = EntitlementManager::new(730);

        assert!(!manager.is_subscribed_from_family_sharing());
        assert!(manager.get_family_sharing_lender().is_none());

        let lender = SteamId::from_account_id(12345);
        manager.set_family_sharing(Some(lender));

        assert!(manager.is_subscribed_from_family_sharing());
        assert_eq!(manager.get_family_sharing_lender(), Some(lender));
        assert_eq!(manager.get_license_type(730), LicenseType::FamilySharing);
    }

    // =============================================================================
    // LIC-005: VAC Game Ban
    // =============================================================================

    #[test]
    fn lic_005_game_ban() {
        let mut manager = EntitlementManager::new(730);

        assert!(!manager.has_game_ban());
        assert!(!manager.is_banned_from_app(730));

        manager.add_game_ban(GameBan {
            app_id: 730,
            issued_at: 1700000000,
            permanent: true,
            reason: Some("Cheating".to_string()),
        });

        assert!(manager.has_game_ban());
        assert!(manager.is_banned_from_app(730));
        assert!(!manager.is_banned_from_app(440)); // Different app
    }

    #[test]
    fn lic_005_multiple_bans() {
        let mut manager = EntitlementManager::new(730);

        manager.add_game_ban(GameBan {
            app_id: 730,
            issued_at: 1700000000,
            permanent: true,
            reason: None,
        });

        manager.add_game_ban(GameBan {
            app_id: 440,
            issued_at: 1700100000,
            permanent: false,
            reason: Some("Temporary ban".to_string()),
        });

        assert_eq!(manager.get_game_bans().len(), 2);
        assert!(manager.is_banned_from_app(730));
        assert!(manager.is_banned_from_app(440));
    }
}
