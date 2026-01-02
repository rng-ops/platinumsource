//! Steam Cloud saves implementation.
//!
//! # Valve Documentation Reference
//! - [Steam Cloud](https://partner.steamgames.com/doc/features/cloud)
//! - [ISteamRemoteStorage](https://partner.steamgames.com/doc/api/ISteamRemoteStorage)
//!
//! # Features
//! - File read/write operations
//! - Quota management
//! - File enumeration
//! - Sync conflict detection

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Cloud storage quota information.
#[derive(Debug, Clone, Copy, Default)]
pub struct CloudQuota {
    /// Total available bytes.
    pub total_bytes: u64,
    /// Currently used bytes.
    pub used_bytes: u64,
}

impl CloudQuota {
    /// Get remaining available bytes.
    pub fn available(&self) -> u64 {
        self.total_bytes.saturating_sub(self.used_bytes)
    }

    /// Check if there's enough space for additional bytes.
    pub fn has_space(&self, bytes: u64) -> bool {
        self.available() >= bytes
    }
}

/// Cloud file metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFileInfo {
    /// File name.
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// Last modified timestamp.
    pub timestamp: u64,
    /// Whether file is persisted to cloud.
    pub persisted: bool,
}

/// Result of a cloud operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudResult {
    Ok,
    FileNotFound,
    QuotaExceeded,
    CloudDisabled,
    WriteError,
    ReadError,
    InvalidName,
    SyncConflict,
}

/// Conflict resolution preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep local version.
    KeepLocal,
    /// Keep remote version.
    KeepRemote,
    /// Keep both (rename local).
    KeepBoth,
}

/// A file in conflict.
#[derive(Debug, Clone)]
pub struct CloudConflict {
    /// File name.
    pub name: String,
    /// Local file data.
    pub local_data: Vec<u8>,
    /// Local timestamp.
    pub local_timestamp: u64,
    /// Remote file data.
    pub remote_data: Vec<u8>,
    /// Remote timestamp.
    pub remote_timestamp: u64,
}

/// Mock cloud storage for testing.
///
/// In production, this would interface with Steamworks SDK.
pub struct CloudStorage {
    /// Stored files.
    files: HashMap<String, Vec<u8>>,
    /// File metadata.
    metadata: HashMap<String, CloudFileInfo>,
    /// Storage quota.
    quota: CloudQuota,
    /// Whether cloud is enabled for account.
    account_enabled: bool,
    /// Whether cloud is enabled for app.
    app_enabled: bool,
    /// Pending conflicts.
    conflicts: Vec<CloudConflict>,
}

impl CloudStorage {
    /// Create a new cloud storage with given quota.
    pub fn new(total_bytes: u64) -> Self {
        CloudStorage {
            files: HashMap::new(),
            metadata: HashMap::new(),
            quota: CloudQuota {
                total_bytes,
                used_bytes: 0,
            },
            account_enabled: true,
            app_enabled: true,
            conflicts: Vec::new(),
        }
    }

    /// Check if cloud is enabled for account.
    pub fn is_cloud_enabled_for_account(&self) -> bool {
        self.account_enabled
    }

    /// Check if cloud is enabled for app.
    pub fn is_cloud_enabled_for_app(&self) -> bool {
        self.app_enabled
    }

    /// Check if cloud is fully enabled.
    pub fn is_enabled(&self) -> bool {
        self.account_enabled && self.app_enabled
    }

    /// Set account cloud enabled.
    pub fn set_account_enabled(&mut self, enabled: bool) {
        self.account_enabled = enabled;
    }

    /// Set app cloud enabled.
    pub fn set_app_enabled(&mut self, enabled: bool) {
        self.app_enabled = enabled;
    }

    /// Get quota information.
    pub fn get_quota(&self) -> CloudQuota {
        self.quota
    }

    /// Get file count.
    pub fn get_file_count(&self) -> usize {
        self.files.len()
    }

    /// Get file info by index.
    pub fn get_file_by_index(&self, index: usize) -> Option<&CloudFileInfo> {
        self.metadata.values().nth(index)
    }

    /// Check if file exists.
    pub fn file_exists(&self, name: &str) -> bool {
        self.files.contains_key(name)
    }

    /// Get file size.
    pub fn get_file_size(&self, name: &str) -> Option<u64> {
        self.metadata.get(name).map(|m| m.size)
    }

    /// Get file timestamp.
    pub fn get_file_timestamp(&self, name: &str) -> Option<u64> {
        self.metadata.get(name).map(|m| m.timestamp)
    }

    /// Write a file to cloud.
    pub fn file_write(&mut self, name: &str, data: &[u8]) -> Result<(), CloudResult> {
        if !self.is_enabled() {
            return Err(CloudResult::CloudDisabled);
        }

        if name.is_empty() || name.len() > 255 {
            return Err(CloudResult::InvalidName);
        }

        let new_size = data.len() as u64;
        let old_size = self.files.get(name).map(|f| f.len() as u64).unwrap_or(0);

        // Check quota
        let size_change = new_size.saturating_sub(old_size);
        if !self.quota.has_space(size_change) {
            return Err(CloudResult::QuotaExceeded);
        }

        // Update quota
        self.quota.used_bytes = self.quota.used_bytes.saturating_sub(old_size) + new_size;

        // Write file
        self.files.insert(name.to_string(), data.to_vec());

        // Update metadata
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.metadata.insert(
            name.to_string(),
            CloudFileInfo {
                name: name.to_string(),
                size: new_size,
                timestamp,
                persisted: true,
            },
        );

        Ok(())
    }

    /// Read a file from cloud.
    pub fn file_read(&self, name: &str) -> Result<Vec<u8>, CloudResult> {
        if !self.is_enabled() {
            return Err(CloudResult::CloudDisabled);
        }

        self.files
            .get(name)
            .cloned()
            .ok_or(CloudResult::FileNotFound)
    }

    /// Delete a file from cloud.
    pub fn file_delete(&mut self, name: &str) -> Result<(), CloudResult> {
        if !self.is_enabled() {
            return Err(CloudResult::CloudDisabled);
        }

        if let Some(data) = self.files.remove(name) {
            self.quota.used_bytes = self.quota.used_bytes.saturating_sub(data.len() as u64);
            self.metadata.remove(name);
            Ok(())
        } else {
            Err(CloudResult::FileNotFound)
        }
    }

    /// List all files.
    pub fn list_files(&self) -> Vec<&CloudFileInfo> {
        self.metadata.values().collect()
    }

    /// Add a sync conflict (for testing).
    pub fn add_conflict(&mut self, conflict: CloudConflict) {
        self.conflicts.push(conflict);
    }

    /// Get pending conflicts.
    pub fn get_conflicts(&self) -> &[CloudConflict] {
        &self.conflicts
    }

    /// Resolve a conflict.
    pub fn resolve_conflict(
        &mut self,
        name: &str,
        resolution: ConflictResolution,
    ) -> Result<(), CloudResult> {
        let conflict_idx = self.conflicts.iter().position(|c| c.name == name);

        if let Some(idx) = conflict_idx {
            let conflict = self.conflicts.remove(idx);

            match resolution {
                ConflictResolution::KeepLocal => {
                    self.file_write(&conflict.name, &conflict.local_data)
                }
                ConflictResolution::KeepRemote => {
                    self.file_write(&conflict.name, &conflict.remote_data)
                }
                ConflictResolution::KeepBoth => {
                    // Rename local file with _local suffix
                    let local_name = format!("{}_local", conflict.name);
                    self.file_write(&local_name, &conflict.local_data)?;
                    self.file_write(&conflict.name, &conflict.remote_data)
                }
            }
        } else {
            Err(CloudResult::FileNotFound)
        }
    }

    /// Has pending conflicts.
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

impl Default for CloudStorage {
    fn default() -> Self {
        // Default: 100 MB quota
        Self::new(100 * 1024 * 1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // CLD-001: File Write
    // Reference: https://partner.steamgames.com/doc/api/ISteamRemoteStorage#FileWrite
    // =============================================================================

    #[test]
    fn cld_001_file_write() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        let result = cloud.file_write("save.dat", b"game save data");
        assert!(result.is_ok());
        assert!(cloud.file_exists("save.dat"));
    }

    #[test]
    fn cld_001_file_overwrite() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        cloud.file_write("save.dat", b"old data").unwrap();
        cloud.file_write("save.dat", b"new data").unwrap();

        let data = cloud.file_read("save.dat").unwrap();
        assert_eq!(data, b"new data");
    }

    // =============================================================================
    // CLD-002: File Read
    // Reference: https://partner.steamgames.com/doc/api/ISteamRemoteStorage#FileRead
    // =============================================================================

    #[test]
    fn cld_002_file_read() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        cloud.file_write("config.cfg", b"settings data").unwrap();

        let data = cloud.file_read("config.cfg").unwrap();
        assert_eq!(data, b"settings data");
    }

    #[test]
    fn cld_002_read_nonexistent() {
        let cloud = CloudStorage::new(1024 * 1024);

        let result = cloud.file_read("missing.dat");
        assert_eq!(result, Err(CloudResult::FileNotFound));
    }

    // =============================================================================
    // CLD-003: File Delete
    // Reference: https://partner.steamgames.com/doc/api/ISteamRemoteStorage#FileDelete
    // =============================================================================

    #[test]
    fn cld_003_file_delete() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        cloud.file_write("temp.dat", b"temporary").unwrap();
        assert!(cloud.file_exists("temp.dat"));

        let result = cloud.file_delete("temp.dat");
        assert!(result.is_ok());
        assert!(!cloud.file_exists("temp.dat"));
    }

    #[test]
    fn cld_003_delete_nonexistent() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        let result = cloud.file_delete("missing.dat");
        assert_eq!(result, Err(CloudResult::FileNotFound));
    }

    // =============================================================================
    // CLD-004: File Exists
    // Reference: https://partner.steamgames.com/doc/api/ISteamRemoteStorage#FileExists
    // =============================================================================

    #[test]
    fn cld_004_file_exists() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        assert!(!cloud.file_exists("save.dat"));

        cloud.file_write("save.dat", b"data").unwrap();
        assert!(cloud.file_exists("save.dat"));
    }

    // =============================================================================
    // CLD-005: File Count
    // Reference: https://partner.steamgames.com/doc/api/ISteamRemoteStorage#GetFileCount
    // =============================================================================

    #[test]
    fn cld_005_file_count() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        assert_eq!(cloud.get_file_count(), 0);

        cloud.file_write("file1.dat", b"data1").unwrap();
        cloud.file_write("file2.dat", b"data2").unwrap();
        cloud.file_write("file3.dat", b"data3").unwrap();

        assert_eq!(cloud.get_file_count(), 3);
    }

    // =============================================================================
    // CLD-006: Quota Check
    // Reference: https://partner.steamgames.com/doc/api/ISteamRemoteStorage#GetQuota
    // =============================================================================

    #[test]
    fn cld_006_quota_check() {
        let mut cloud = CloudStorage::new(1000);

        let quota = cloud.get_quota();
        assert_eq!(quota.total_bytes, 1000);
        assert_eq!(quota.used_bytes, 0);
        assert_eq!(quota.available(), 1000);

        cloud.file_write("data.dat", &[0u8; 300]).unwrap();

        let quota = cloud.get_quota();
        assert_eq!(quota.used_bytes, 300);
        assert_eq!(quota.available(), 700);
    }

    #[test]
    fn cld_006_quota_exceeded() {
        let mut cloud = CloudStorage::new(100);

        let result = cloud.file_write("big.dat", &[0u8; 200]);
        assert_eq!(result, Err(CloudResult::QuotaExceeded));
    }

    // =============================================================================
    // CLD-007: Cloud Enabled Check
    // Reference: https://partner.steamgames.com/doc/api/ISteamRemoteStorage#IsCloudEnabledForAccount
    // =============================================================================

    #[test]
    fn cld_007_cloud_enabled_account() {
        let mut cloud = CloudStorage::new(1024);

        assert!(cloud.is_cloud_enabled_for_account());

        cloud.set_account_enabled(false);
        assert!(!cloud.is_cloud_enabled_for_account());
        assert!(!cloud.is_enabled());
    }

    #[test]
    fn cld_007_cloud_enabled_app() {
        let mut cloud = CloudStorage::new(1024);

        assert!(cloud.is_cloud_enabled_for_app());

        cloud.set_app_enabled(false);
        assert!(!cloud.is_cloud_enabled_for_app());
        assert!(!cloud.is_enabled());
    }

    #[test]
    fn cld_007_operations_disabled() {
        let mut cloud = CloudStorage::new(1024);
        cloud.set_account_enabled(false);

        let write_result = cloud.file_write("test.dat", b"data");
        assert_eq!(write_result, Err(CloudResult::CloudDisabled));

        let read_result = cloud.file_read("test.dat");
        assert_eq!(read_result, Err(CloudResult::CloudDisabled));
    }

    // =============================================================================
    // CLD-008: Conflict Resolution
    // =============================================================================

    #[test]
    fn cld_008_conflict_detection() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        let conflict = CloudConflict {
            name: "save.dat".to_string(),
            local_data: b"local version".to_vec(),
            local_timestamp: 1000,
            remote_data: b"remote version".to_vec(),
            remote_timestamp: 2000,
        };

        cloud.add_conflict(conflict);
        assert!(cloud.has_conflicts());
        assert_eq!(cloud.get_conflicts().len(), 1);
    }

    #[test]
    fn cld_008_resolve_keep_local() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        let conflict = CloudConflict {
            name: "save.dat".to_string(),
            local_data: b"local version".to_vec(),
            local_timestamp: 1000,
            remote_data: b"remote version".to_vec(),
            remote_timestamp: 2000,
        };

        cloud.add_conflict(conflict);
        cloud.resolve_conflict("save.dat", ConflictResolution::KeepLocal).unwrap();

        let data = cloud.file_read("save.dat").unwrap();
        assert_eq!(data, b"local version");
        assert!(!cloud.has_conflicts());
    }

    #[test]
    fn cld_008_resolve_keep_remote() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        let conflict = CloudConflict {
            name: "save.dat".to_string(),
            local_data: b"local version".to_vec(),
            local_timestamp: 1000,
            remote_data: b"remote version".to_vec(),
            remote_timestamp: 2000,
        };

        cloud.add_conflict(conflict);
        cloud.resolve_conflict("save.dat", ConflictResolution::KeepRemote).unwrap();

        let data = cloud.file_read("save.dat").unwrap();
        assert_eq!(data, b"remote version");
    }

    // =============================================================================
    // Additional Tests
    // =============================================================================

    #[test]
    fn quota_updates_on_delete() {
        let mut cloud = CloudStorage::new(1000);

        cloud.file_write("data.dat", &[0u8; 300]).unwrap();
        assert_eq!(cloud.get_quota().used_bytes, 300);

        cloud.file_delete("data.dat").unwrap();
        assert_eq!(cloud.get_quota().used_bytes, 0);
    }

    #[test]
    fn quota_updates_on_overwrite() {
        let mut cloud = CloudStorage::new(1000);

        cloud.file_write("data.dat", &[0u8; 300]).unwrap();
        assert_eq!(cloud.get_quota().used_bytes, 300);

        cloud.file_write("data.dat", &[0u8; 500]).unwrap();
        assert_eq!(cloud.get_quota().used_bytes, 500);
    }

    #[test]
    fn file_timestamp() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        cloud.file_write("test.dat", b"data").unwrap();

        let timestamp = cloud.get_file_timestamp("test.dat");
        assert!(timestamp.is_some());
        assert!(timestamp.unwrap() > 0);
    }

    #[test]
    fn invalid_file_name() {
        let mut cloud = CloudStorage::new(1024 * 1024);

        let result = cloud.file_write("", b"data");
        assert_eq!(result, Err(CloudResult::InvalidName));

        let long_name = "a".repeat(300);
        let result = cloud.file_write(&long_name, b"data");
        assert_eq!(result, Err(CloudResult::InvalidName));
    }
}
