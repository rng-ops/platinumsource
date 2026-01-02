//! Steam Workshop integration.
//!
//! # Valve Documentation Reference
//! - [Steam Workshop](https://partner.steamgames.com/doc/features/workshop)
//! - [ISteamUGC](https://partner.steamgames.com/doc/api/ISteamUGC)
//!
//! # Features
//! - Workshop item publishing and updating
//! - Item subscription and download management
//! - Query and discovery APIs
//! - Voting and engagement

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Workshop item published file ID.
pub type PublishedFileId = u64;

/// Workshop item state flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemState {
    bits: u32,
}

impl ItemState {
    /// No state.
    pub const NONE: u32 = 0;
    /// User has subscribed to this item.
    pub const SUBSCRIBED: u32 = 1;
    /// Legacy Steamworks item (not managed via ISteamUGC).
    pub const LEGACY_ITEM: u32 = 2;
    /// Item is fully installed.
    pub const INSTALLED: u32 = 4;
    /// Item needs update.
    pub const NEEDS_UPDATE: u32 = 8;
    /// Item is currently downloading.
    pub const DOWNLOADING: u32 = 16;
    /// Item is pending download.
    pub const DOWNLOAD_PENDING: u32 = 32;

    /// Create from raw bits.
    pub fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    /// Get raw bits.
    pub fn bits(&self) -> u32 {
        self.bits
    }

    /// Check if flag is set.
    pub fn contains(&self, flag: u32) -> bool {
        (self.bits & flag) != 0
    }

    /// Add a flag.
    pub fn insert(&mut self, flag: u32) {
        self.bits |= flag;
    }

    /// Remove a flag.
    pub fn remove(&mut self, flag: u32) {
        self.bits &= !flag;
    }
}

/// Workshop item visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ItemVisibility {
    /// Visible to everyone.
    #[default]
    Public,
    /// Visible only to friends.
    FriendsOnly,
    /// Visible only to owner.
    Private,
    /// Unlisted (accessible via direct link).
    Unlisted,
}

/// Workshop item type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkshopFileType {
    /// Regular item.
    #[default]
    Community,
    /// Microtransaction item.
    Microtransaction,
    /// Collection of items.
    Collection,
    /// Art item.
    Art,
    /// Video item.
    Video,
    /// Screenshot.
    Screenshot,
    /// Game guide.
    Guide,
    /// Integrated guide.
    IntegratedGuide,
    /// Merch item.
    Merch,
}

/// Workshop item details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopItem {
    /// Published file ID.
    pub file_id: PublishedFileId,
    /// Item title.
    pub title: String,
    /// Item description.
    pub description: String,
    /// Creator Steam ID.
    pub creator_id: u64,
    /// App ID this item belongs to.
    pub app_id: u32,
    /// File size in bytes.
    pub file_size: u64,
    /// Preview image URL.
    pub preview_url: String,
    /// Tags.
    pub tags: Vec<String>,
    /// Vote score (up - down).
    pub vote_score: f32,
    /// Vote count up.
    pub votes_up: u32,
    /// Vote count down.
    pub votes_down: u32,
    /// Subscription count.
    pub subscriptions: u64,
    /// Creation timestamp.
    pub created: u64,
    /// Last updated timestamp.
    pub updated: u64,
    /// Visibility.
    pub visibility: ItemVisibility,
    /// Content hash for verification.
    pub content_hash: String,
}

impl WorkshopItem {
    /// Create a new workshop item.
    pub fn new(file_id: PublishedFileId, title: &str, app_id: u32) -> Self {
        Self {
            file_id,
            title: title.to_string(),
            description: String::new(),
            creator_id: 0,
            app_id,
            file_size: 0,
            preview_url: String::new(),
            tags: Vec::new(),
            vote_score: 0.0,
            votes_up: 0,
            votes_down: 0,
            subscriptions: 0,
            created: 0,
            updated: 0,
            visibility: ItemVisibility::Public,
            content_hash: String::new(),
        }
    }
}

/// Workshop item installation info.
#[derive(Debug, Clone)]
pub struct InstallInfo {
    /// Installation folder path.
    pub folder: String,
    /// Total size in bytes.
    pub size_on_disk: u64,
    /// Timestamp of installation.
    pub timestamp: u64,
}

/// Workshop item download progress.
#[derive(Debug, Clone, Copy, Default)]
pub struct DownloadProgress {
    /// Bytes downloaded.
    pub bytes_downloaded: u64,
    /// Total bytes.
    pub bytes_total: u64,
}

impl DownloadProgress {
    /// Get percentage complete.
    pub fn percent(&self) -> f32 {
        if self.bytes_total == 0 {
            0.0
        } else {
            (self.bytes_downloaded as f32 / self.bytes_total as f32) * 100.0
        }
    }
}

/// User vote on an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserVote {
    VotedUp,
    VotedDown,
    NotVoted,
}

/// Workshop query result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkshopResult {
    Ok,
    Fail,
    InvalidParam,
    FileNotFound,
    NotSubscribed,
    AlreadySubscribed,
    AccessDenied,
    Timeout,
    Banned,
    NotLoggedIn,
    InsufficientPrivilege,
    LimitExceeded,
}

/// Mock Workshop manager for testing.
///
/// In production, this would interface with Steamworks SDK.
pub struct WorkshopManager {
    /// App ID.
    app_id: u32,
    /// Local user Steam ID.
    local_user: u64,
    /// Published items.
    items: HashMap<PublishedFileId, WorkshopItem>,
    /// Subscribed items.
    subscriptions: Vec<PublishedFileId>,
    /// Installed items.
    installed: HashMap<PublishedFileId, InstallInfo>,
    /// Item states.
    states: HashMap<PublishedFileId, ItemState>,
    /// Download progress.
    downloads: HashMap<PublishedFileId, DownloadProgress>,
    /// User votes.
    votes: HashMap<PublishedFileId, UserVote>,
    /// Next file ID for creation.
    next_file_id: PublishedFileId,
    /// Maximum subscriptions allowed.
    max_subscriptions: usize,
    /// Item dependencies.
    dependencies: HashMap<PublishedFileId, Vec<PublishedFileId>>,
}

impl WorkshopManager {
    /// Create a new workshop manager.
    pub fn new(app_id: u32, local_user: u64) -> Self {
        Self {
            app_id,
            local_user,
            items: HashMap::new(),
            subscriptions: Vec::new(),
            installed: HashMap::new(),
            states: HashMap::new(),
            downloads: HashMap::new(),
            votes: HashMap::new(),
            next_file_id: 1000,
            max_subscriptions: 1000,
            dependencies: HashMap::new(),
        }
    }

    /// Create a new workshop item.
    pub fn create_item(&mut self, title: &str) -> Result<PublishedFileId, WorkshopResult> {
        if title.is_empty() {
            return Err(WorkshopResult::InvalidParam);
        }

        let file_id = self.next_file_id;
        self.next_file_id += 1;

        let item = WorkshopItem::new(file_id, title, self.app_id);
        self.items.insert(file_id, item);
        self.states.insert(file_id, ItemState::default());

        Ok(file_id)
    }

    /// Update a workshop item.
    pub fn submit_item_update(
        &mut self,
        file_id: PublishedFileId,
        title: Option<&str>,
        description: Option<&str>,
        tags: Option<Vec<String>>,
    ) -> Result<(), WorkshopResult> {
        let item = match self.items.get_mut(&file_id) {
            Some(item) => item,
            None => return Err(WorkshopResult::FileNotFound),
        };

        if let Some(t) = title {
            item.title = t.to_string();
        }
        if let Some(d) = description {
            item.description = d.to_string();
        }
        if let Some(t) = tags {
            item.tags = t;
        }
        item.updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(())
    }

    /// Subscribe to an item.
    pub fn subscribe_item(&mut self, file_id: PublishedFileId) -> Result<(), WorkshopResult> {
        if self.subscriptions.contains(&file_id) {
            return Err(WorkshopResult::AlreadySubscribed);
        }

        if self.subscriptions.len() >= self.max_subscriptions {
            return Err(WorkshopResult::LimitExceeded);
        }

        // Create a mock item if it doesn't exist (simulating remote item).
        if !self.items.contains_key(&file_id) {
            let item = WorkshopItem::new(file_id, "Remote Item", self.app_id);
            self.items.insert(file_id, item);
        }

        self.subscriptions.push(file_id);

        // Update state.
        let state = self.states.entry(file_id).or_default();
        state.insert(ItemState::SUBSCRIBED);
        state.insert(ItemState::DOWNLOAD_PENDING);

        Ok(())
    }

    /// Unsubscribe from an item.
    pub fn unsubscribe_item(&mut self, file_id: PublishedFileId) -> Result<(), WorkshopResult> {
        if let Some(pos) = self.subscriptions.iter().position(|&id| id == file_id) {
            self.subscriptions.remove(pos);

            // Update state.
            if let Some(state) = self.states.get_mut(&file_id) {
                state.remove(ItemState::SUBSCRIBED);
                state.remove(ItemState::INSTALLED);
            }

            // Remove installation.
            self.installed.remove(&file_id);

            Ok(())
        } else {
            Err(WorkshopResult::NotSubscribed)
        }
    }

    /// Get subscribed items.
    pub fn get_subscribed_items(&self) -> &[PublishedFileId] {
        &self.subscriptions
    }

    /// Get item state.
    pub fn get_item_state(&self, file_id: PublishedFileId) -> ItemState {
        self.states.get(&file_id).copied().unwrap_or_default()
    }

    /// Download an item.
    pub fn download_item(&mut self, file_id: PublishedFileId, high_priority: bool) -> Result<(), WorkshopResult> {
        let state = self.states.entry(file_id).or_default();

        if !state.contains(ItemState::SUBSCRIBED) {
            return Err(WorkshopResult::NotSubscribed);
        }

        // Mark as downloading.
        state.remove(ItemState::DOWNLOAD_PENDING);
        state.insert(ItemState::DOWNLOADING);

        // Simulate download progress.
        let item = self.items.get(&file_id);
        let total_size = item.map(|i| i.file_size).unwrap_or(1000);

        self.downloads.insert(file_id, DownloadProgress {
            bytes_downloaded: 0,
            bytes_total: total_size.max(1000),
        });

        // For testing, immediately complete if high priority.
        if high_priority {
            self.complete_download(file_id);
        }

        Ok(())
    }

    /// Complete a download (for testing).
    pub fn complete_download(&mut self, file_id: PublishedFileId) {
        if let Some(state) = self.states.get_mut(&file_id) {
            state.remove(ItemState::DOWNLOADING);
            state.remove(ItemState::NEEDS_UPDATE);
            state.insert(ItemState::INSTALLED);
        }

        if let Some(progress) = self.downloads.get_mut(&file_id) {
            progress.bytes_downloaded = progress.bytes_total;
        }

        // Create install info.
        let item = self.items.get(&file_id);
        self.installed.insert(file_id, InstallInfo {
            folder: format!("/workshop/content/{}/{}", self.app_id, file_id),
            size_on_disk: item.map(|i| i.file_size).unwrap_or(1000),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });
    }

    /// Get download progress.
    pub fn get_item_download_info(&self, file_id: PublishedFileId) -> Option<&DownloadProgress> {
        self.downloads.get(&file_id)
    }

    /// Get installation info.
    pub fn get_item_install_info(&self, file_id: PublishedFileId) -> Option<&InstallInfo> {
        self.installed.get(&file_id)
    }

    /// Get item details.
    pub fn get_item_details(&self, file_id: PublishedFileId) -> Option<&WorkshopItem> {
        self.items.get(&file_id)
    }

    /// Set user vote on item.
    pub fn set_user_item_vote(&mut self, file_id: PublishedFileId, vote_up: bool) -> Result<(), WorkshopResult> {
        if !self.items.contains_key(&file_id) {
            return Err(WorkshopResult::FileNotFound);
        }

        let vote = if vote_up { UserVote::VotedUp } else { UserVote::VotedDown };
        self.votes.insert(file_id, vote);

        // Update vote counts.
        if let Some(item) = self.items.get_mut(&file_id) {
            if vote_up {
                item.votes_up += 1;
            } else {
                item.votes_down += 1;
            }
            item.vote_score = (item.votes_up as f32) / (item.votes_up + item.votes_down).max(1) as f32;
        }

        Ok(())
    }

    /// Get user vote on item.
    pub fn get_user_item_vote(&self, file_id: PublishedFileId) -> UserVote {
        self.votes.get(&file_id).copied().unwrap_or(UserVote::NotVoted)
    }

    /// Add dependency for an item.
    pub fn add_dependency(&mut self, file_id: PublishedFileId, dependency_id: PublishedFileId) {
        self.dependencies
            .entry(file_id)
            .or_default()
            .push(dependency_id);
    }

    /// Get item dependencies.
    pub fn get_dependencies(&self, file_id: PublishedFileId) -> Option<&Vec<PublishedFileId>> {
        self.dependencies.get(&file_id)
    }

    /// Mark item as needing update.
    pub fn mark_needs_update(&mut self, file_id: PublishedFileId) {
        if let Some(state) = self.states.get_mut(&file_id) {
            state.insert(ItemState::NEEDS_UPDATE);
        }
    }

    /// Verify content integrity.
    pub fn verify_content(&self, file_id: PublishedFileId, expected_hash: &str) -> bool {
        if let Some(item) = self.items.get(&file_id) {
            item.content_hash == expected_hash
        } else {
            false
        }
    }

    /// Set content hash for an item.
    pub fn set_content_hash(&mut self, file_id: PublishedFileId, hash: &str) {
        if let Some(item) = self.items.get_mut(&file_id) {
            item.content_hash = hash.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // WKS-001: Create Item
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#CreateItem
    // =============================================================================

    #[test]
    fn wks_001_create_item() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Map");
        assert!(file_id.is_ok());

        let file_id = file_id.unwrap();
        assert!(file_id >= 1000);

        let item = workshop.get_item_details(file_id);
        assert!(item.is_some());
        assert_eq!(item.unwrap().title, "Test Map");
    }

    #[test]
    fn wks_001_create_item_empty_title() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let result = workshop.create_item("");
        assert_eq!(result, Err(WorkshopResult::InvalidParam));
    }

    // =============================================================================
    // WKS-002: Update Item
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#SubmitItemUpdate
    // =============================================================================

    #[test]
    fn wks_002_update_item() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Original Title").unwrap();

        let result = workshop.submit_item_update(
            file_id,
            Some("Updated Title"),
            Some("New description"),
            Some(vec!["tag1".to_string(), "tag2".to_string()]),
        );
        assert!(result.is_ok());

        let item = workshop.get_item_details(file_id).unwrap();
        assert_eq!(item.title, "Updated Title");
        assert_eq!(item.description, "New description");
        assert_eq!(item.tags, vec!["tag1", "tag2"]);
    }

    #[test]
    fn wks_002_update_nonexistent() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let result = workshop.submit_item_update(99999, Some("Title"), None, None);
        assert_eq!(result, Err(WorkshopResult::FileNotFound));
    }

    // =============================================================================
    // WKS-003: Query Items
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#CreateQueryAllUGCRequest
    // =============================================================================

    #[test]
    fn wks_003_query_items() {
        let mut workshop = WorkshopManager::new(730, 12345);

        workshop.create_item("Map 1").unwrap();
        workshop.create_item("Map 2").unwrap();
        workshop.create_item("Map 3").unwrap();

        // Query by iterating items (simplified).
        let count: usize = (1000..1003)
            .filter(|&id| workshop.get_item_details(id).is_some())
            .count();
        assert_eq!(count, 3);
    }

    // =============================================================================
    // WKS-004: Subscribe to Item
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#SubscribeItem
    // =============================================================================

    #[test]
    fn wks_004_subscribe_item() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();

        let result = workshop.subscribe_item(file_id);
        assert!(result.is_ok());

        let state = workshop.get_item_state(file_id);
        assert!(state.contains(ItemState::SUBSCRIBED));

        assert!(workshop.get_subscribed_items().contains(&file_id));
    }

    #[test]
    fn wks_004_subscribe_already_subscribed() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.subscribe_item(file_id).unwrap();

        let result = workshop.subscribe_item(file_id);
        assert_eq!(result, Err(WorkshopResult::AlreadySubscribed));
    }

    // =============================================================================
    // WKS-005: Unsubscribe Item
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#UnsubscribeItem
    // =============================================================================

    #[test]
    fn wks_005_unsubscribe_item() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.subscribe_item(file_id).unwrap();

        let result = workshop.unsubscribe_item(file_id);
        assert!(result.is_ok());

        let state = workshop.get_item_state(file_id);
        assert!(!state.contains(ItemState::SUBSCRIBED));

        assert!(!workshop.get_subscribed_items().contains(&file_id));
    }

    #[test]
    fn wks_005_unsubscribe_not_subscribed() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let result = workshop.unsubscribe_item(99999);
        assert_eq!(result, Err(WorkshopResult::NotSubscribed));
    }

    // =============================================================================
    // WKS-006: Download Item
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#DownloadItem
    // =============================================================================

    #[test]
    fn wks_006_download_item() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.subscribe_item(file_id).unwrap();

        let result = workshop.download_item(file_id, true);
        assert!(result.is_ok());

        let state = workshop.get_item_state(file_id);
        assert!(state.contains(ItemState::INSTALLED));
    }

    #[test]
    fn wks_006_download_not_subscribed() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();

        let result = workshop.download_item(file_id, false);
        assert_eq!(result, Err(WorkshopResult::NotSubscribed));
    }

    // =============================================================================
    // WKS-007: Get Item State
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#GetItemState
    // =============================================================================

    #[test]
    fn wks_007_get_item_state() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();

        // Initial state.
        let state = workshop.get_item_state(file_id);
        assert_eq!(state.bits(), ItemState::NONE);

        // After subscription.
        workshop.subscribe_item(file_id).unwrap();
        let state = workshop.get_item_state(file_id);
        assert!(state.contains(ItemState::SUBSCRIBED));
        assert!(state.contains(ItemState::DOWNLOAD_PENDING));

        // After download.
        workshop.download_item(file_id, true).unwrap();
        let state = workshop.get_item_state(file_id);
        assert!(state.contains(ItemState::INSTALLED));
    }

    #[test]
    fn wks_007_needs_update_state() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.subscribe_item(file_id).unwrap();
        workshop.download_item(file_id, true).unwrap();

        workshop.mark_needs_update(file_id);

        let state = workshop.get_item_state(file_id);
        assert!(state.contains(ItemState::NEEDS_UPDATE));
    }

    // =============================================================================
    // WKS-008: Get Install Info
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#GetItemInstallInfo
    // =============================================================================

    #[test]
    fn wks_008_get_install_info() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.subscribe_item(file_id).unwrap();
        workshop.download_item(file_id, true).unwrap();

        let info = workshop.get_item_install_info(file_id);
        assert!(info.is_some());

        let info = info.unwrap();
        assert!(info.folder.contains(&file_id.to_string()));
        assert!(info.timestamp > 0);
    }

    #[test]
    fn wks_008_no_install_info_before_download() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.subscribe_item(file_id).unwrap();

        let info = workshop.get_item_install_info(file_id);
        assert!(info.is_none());
    }

    // =============================================================================
    // WKS-009: Item Metadata
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#GetQueryUGCResult
    // =============================================================================

    #[test]
    fn wks_009_item_metadata() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Map").unwrap();
        workshop.submit_item_update(
            file_id,
            Some("Cool Map"),
            Some("A very cool map"),
            Some(vec!["competitive".to_string(), "hostage".to_string()]),
        ).unwrap();

        let item = workshop.get_item_details(file_id).unwrap();
        assert_eq!(item.title, "Cool Map");
        assert_eq!(item.description, "A very cool map");
        assert!(item.tags.contains(&"competitive".to_string()));
        assert!(item.tags.contains(&"hostage".to_string()));
    }

    // =============================================================================
    // WKS-010: Vote on Item
    // Reference: https://partner.steamgames.com/doc/api/ISteamUGC#SetUserItemVote
    // =============================================================================

    #[test]
    fn wks_010_vote_on_item() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();

        let result = workshop.set_user_item_vote(file_id, true);
        assert!(result.is_ok());

        let vote = workshop.get_user_item_vote(file_id);
        assert_eq!(vote, UserVote::VotedUp);

        let item = workshop.get_item_details(file_id).unwrap();
        assert_eq!(item.votes_up, 1);
    }

    #[test]
    fn wks_010_vote_down() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();

        workshop.set_user_item_vote(file_id, false).unwrap();

        let vote = workshop.get_user_item_vote(file_id);
        assert_eq!(vote, UserVote::VotedDown);
    }

    // =============================================================================
    // WKS-VER-001: Content Hash Verification
    // =============================================================================

    #[test]
    fn wks_ver_001_content_hash() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.set_content_hash(file_id, "abc123hash");

        assert!(workshop.verify_content(file_id, "abc123hash"));
        assert!(!workshop.verify_content(file_id, "wronghash"));
    }

    // =============================================================================
    // WKS-VER-002: File Size Match
    // =============================================================================

    #[test]
    fn wks_ver_002_file_size_match() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();

        // Modify item size.
        if let Some(item) = workshop.items.get_mut(&file_id) {
            item.file_size = 5000;
        }

        workshop.subscribe_item(file_id).unwrap();
        workshop.download_item(file_id, true).unwrap();

        let item = workshop.get_item_details(file_id).unwrap();
        let install = workshop.get_item_install_info(file_id).unwrap();

        assert_eq!(item.file_size, install.size_on_disk);
    }

    // =============================================================================
    // WKS-VER-004: Dependency Resolution
    // =============================================================================

    #[test]
    fn wks_ver_004_dependency_resolution() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let base_id = workshop.create_item("Base Framework").unwrap();
        let addon_id = workshop.create_item("Addon").unwrap();

        workshop.add_dependency(addon_id, base_id);

        let deps = workshop.get_dependencies(addon_id);
        assert!(deps.is_some());
        assert!(deps.unwrap().contains(&base_id));
    }

    // =============================================================================
    // WKS-VER-005: Version Matching
    // =============================================================================

    #[test]
    fn wks_ver_005_version_matching() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.subscribe_item(file_id).unwrap();
        workshop.download_item(file_id, true).unwrap();

        // Initially installed and up to date.
        let state = workshop.get_item_state(file_id);
        assert!(state.contains(ItemState::INSTALLED));
        assert!(!state.contains(ItemState::NEEDS_UPDATE));

        // Mark as needing update.
        workshop.mark_needs_update(file_id);
        let state = workshop.get_item_state(file_id);
        assert!(state.contains(ItemState::NEEDS_UPDATE));

        // Re-download to update.
        workshop.download_item(file_id, true).unwrap();
        let state = workshop.get_item_state(file_id);
        assert!(!state.contains(ItemState::NEEDS_UPDATE));
    }

    // =============================================================================
    // Additional Tests
    // =============================================================================

    #[test]
    fn download_progress_tracking() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.subscribe_item(file_id).unwrap();
        workshop.download_item(file_id, false).unwrap();

        let progress = workshop.get_item_download_info(file_id);
        assert!(progress.is_some());
        assert_eq!(progress.unwrap().bytes_downloaded, 0);
    }

    #[test]
    fn cleanup_on_unsubscribe() {
        let mut workshop = WorkshopManager::new(730, 12345);

        let file_id = workshop.create_item("Test Item").unwrap();
        workshop.subscribe_item(file_id).unwrap();
        workshop.download_item(file_id, true).unwrap();

        assert!(workshop.get_item_install_info(file_id).is_some());

        workshop.unsubscribe_item(file_id).unwrap();

        assert!(workshop.get_item_install_info(file_id).is_none());
    }
}
