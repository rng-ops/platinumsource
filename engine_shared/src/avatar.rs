//! Steam Avatar system implementation.
//!
//! # Valve Documentation Reference
//! - [ISteamFriends](https://partner.steamgames.com/doc/api/ISteamFriends)
//! - [Avatar System](https://partner.steamgames.com/doc/features/avatars)
//!
//! # Features
//! - Avatar retrieval in multiple sizes (32x32, 64x64, 128x128)
//! - Async avatar loading with callbacks
//! - Avatar caching for performance

use std::collections::HashMap;

/// Avatar size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AvatarSize {
    /// 32x32 pixels.
    Small,
    /// 64x64 pixels.
    Medium,
    /// 128x128 pixels.
    Large,
}

impl AvatarSize {
    /// Get the pixel dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            AvatarSize::Small => (32, 32),
            AvatarSize::Medium => (64, 64),
            AvatarSize::Large => (128, 128),
        }
    }

    /// Get byte size for RGBA data.
    pub fn byte_size(&self) -> usize {
        let (w, h) = self.dimensions();
        (w * h * 4) as usize
    }
}

/// Result of avatar request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarResult {
    /// Avatar image handle (positive number).
    Handle(i32),
    /// Image not cached, callback will fire when ready.
    NotCached,
    /// User has no avatar set.
    NoAvatar,
    /// Error retrieving avatar.
    Error,
}

/// Avatar image data.
#[derive(Debug, Clone)]
pub struct AvatarImage {
    /// Image handle.
    pub handle: i32,
    /// RGBA pixel data.
    pub data: Vec<u8>,
    /// Image width.
    pub width: u32,
    /// Image height.
    pub height: u32,
}

impl AvatarImage {
    /// Create a new avatar image.
    pub fn new(handle: i32, size: AvatarSize) -> Self {
        let (width, height) = size.dimensions();
        let byte_size = size.byte_size();

        // Generate placeholder data (colored based on handle).
        let r = ((handle * 17) % 256) as u8;
        let g = ((handle * 31) % 256) as u8;
        let b = ((handle * 47) % 256) as u8;

        let mut data = Vec::with_capacity(byte_size);
        for _ in 0..(width * height) {
            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255); // Alpha
        }

        Self {
            handle,
            data,
            width,
            height,
        }
    }

    /// Create a default avatar (for users with no avatar).
    pub fn default_avatar(size: AvatarSize) -> Self {
        let (width, height) = size.dimensions();
        let byte_size = size.byte_size();

        // Default avatar: gray placeholder.
        let mut data = Vec::with_capacity(byte_size);
        for _ in 0..(width * height) {
            data.push(128);
            data.push(128);
            data.push(128);
            data.push(255);
        }

        Self {
            handle: 0,
            data,
            width,
            height,
        }
    }
}

/// Avatar loaded callback data.
#[derive(Debug, Clone)]
pub struct AvatarImageLoaded {
    /// Steam ID of the user.
    pub steam_id: u64,
    /// Image handle.
    pub image: i32,
    /// Image width.
    pub wide: i32,
    /// Image height.
    pub tall: i32,
}

/// Mock Avatar manager for testing.
///
/// In production, this would interface with Steamworks SDK.
pub struct AvatarManager {
    /// Cached avatar images by (steam_id, size).
    cache: HashMap<(u64, AvatarSize), AvatarImage>,
    /// Users with no avatar.
    no_avatar_users: Vec<u64>,
    /// Pending avatar loads (steam_id, size).
    pending: Vec<(u64, AvatarSize)>,
    /// Callback queue.
    callbacks: Vec<AvatarImageLoaded>,
    /// Next image handle.
    next_handle: i32,
    /// Local user Steam ID.
    local_user: u64,
    /// Cache hits counter.
    cache_hits: usize,
    /// Cache misses counter.
    cache_misses: usize,
}

impl AvatarManager {
    /// Create a new avatar manager.
    pub fn new(local_user: u64) -> Self {
        Self {
            cache: HashMap::new(),
            no_avatar_users: Vec::new(),
            pending: Vec::new(),
            callbacks: Vec::new(),
            next_handle: 1,
            local_user,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Get small avatar (32x32) for a user.
    pub fn get_small_friend_avatar(&mut self, steam_id: u64) -> AvatarResult {
        self.get_avatar(steam_id, AvatarSize::Small)
    }

    /// Get medium avatar (64x64) for a user.
    pub fn get_medium_friend_avatar(&mut self, steam_id: u64) -> AvatarResult {
        self.get_avatar(steam_id, AvatarSize::Medium)
    }

    /// Get large avatar (128x128) for a user.
    pub fn get_large_friend_avatar(&mut self, steam_id: u64) -> AvatarResult {
        self.get_avatar(steam_id, AvatarSize::Large)
    }

    /// Get avatar for a user.
    fn get_avatar(&mut self, steam_id: u64, size: AvatarSize) -> AvatarResult {
        // Check if user has no avatar.
        if self.no_avatar_users.contains(&steam_id) {
            return AvatarResult::NoAvatar;
        }

        // Check cache.
        if let Some(image) = self.cache.get(&(steam_id, size)) {
            self.cache_hits += 1;
            return AvatarResult::Handle(image.handle);
        }

        self.cache_misses += 1;

        // Check if already pending.
        if self.pending.contains(&(steam_id, size)) {
            return AvatarResult::NotCached;
        }

        // Queue for async load.
        self.pending.push((steam_id, size));
        AvatarResult::NotCached
    }

    /// Process pending avatar loads (for testing).
    pub fn process_pending(&mut self) {
        let pending = std::mem::take(&mut self.pending);

        for (steam_id, size) in pending {
            let handle = self.next_handle;
            self.next_handle += 1;

            let image = AvatarImage::new(handle, size);
            let (wide, tall) = size.dimensions();

            self.cache.insert((steam_id, size), image);

            self.callbacks.push(AvatarImageLoaded {
                steam_id,
                image: handle,
                wide: wide as i32,
                tall: tall as i32,
            });
        }
    }

    /// Pop next callback.
    pub fn pop_callback(&mut self) -> Option<AvatarImageLoaded> {
        self.callbacks.pop()
    }

    /// Get image size for a handle.
    pub fn get_image_size(&self, handle: i32) -> Option<(u32, u32)> {
        for image in self.cache.values() {
            if image.handle == handle {
                return Some((image.width, image.height));
            }
        }
        None
    }

    /// Get image RGBA data for a handle.
    pub fn get_image_rgba(&self, handle: i32) -> Option<&[u8]> {
        for image in self.cache.values() {
            if image.handle == handle {
                return Some(&image.data);
            }
        }
        None
    }

    /// Get cached avatar for a user.
    pub fn get_cached_avatar(&self, steam_id: u64, size: AvatarSize) -> Option<&AvatarImage> {
        self.cache.get(&(steam_id, size))
    }

    /// Mark a user as having no avatar.
    pub fn set_no_avatar(&mut self, steam_id: u64) {
        if !self.no_avatar_users.contains(&steam_id) {
            self.no_avatar_users.push(steam_id);
        }
    }

    /// Clear no-avatar status (avatar was set).
    pub fn clear_no_avatar(&mut self, steam_id: u64) {
        self.no_avatar_users.retain(|&id| id != steam_id);
    }

    /// Get local user's avatar.
    pub fn get_my_avatar(&mut self, size: AvatarSize) -> AvatarResult {
        self.get_avatar(self.local_user, size)
    }

    /// Get default avatar for fallback.
    pub fn get_default_avatar(&self, size: AvatarSize) -> AvatarImage {
        AvatarImage::default_avatar(size)
    }

    /// Get cache statistics.
    pub fn get_cache_stats(&self) -> (usize, usize) {
        (self.cache_hits, self.cache_misses)
    }

    /// Clear cache for a user (avatar was updated).
    pub fn invalidate_cache(&mut self, steam_id: u64) {
        self.cache.retain(|&(id, _), _| id != steam_id);
    }

    /// Check if avatar is cached.
    pub fn is_cached(&self, steam_id: u64, size: AvatarSize) -> bool {
        self.cache.contains_key(&(steam_id, size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // AVT-001: Get Small Avatar
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetSmallFriendAvatar
    // =============================================================================

    #[test]
    fn avt_001_get_small_avatar() {
        let mut mgr = AvatarManager::new(12345);

        // First request returns NotCached.
        let result = mgr.get_small_friend_avatar(67890);
        assert_eq!(result, AvatarResult::NotCached);

        // Process pending loads.
        mgr.process_pending();

        // Now should return handle.
        let result = mgr.get_small_friend_avatar(67890);
        match result {
            AvatarResult::Handle(h) => assert!(h > 0),
            _ => panic!("Expected Handle"),
        }
    }

    #[test]
    fn avt_001_small_avatar_dimensions() {
        let mut mgr = AvatarManager::new(12345);

        mgr.get_small_friend_avatar(67890);
        mgr.process_pending();

        let avatar = mgr.get_cached_avatar(67890, AvatarSize::Small).unwrap();
        assert_eq!(avatar.width, 32);
        assert_eq!(avatar.height, 32);
    }

    // =============================================================================
    // AVT-002: Get Medium Avatar
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetMediumFriendAvatar
    // =============================================================================

    #[test]
    fn avt_002_get_medium_avatar() {
        let mut mgr = AvatarManager::new(12345);

        mgr.get_medium_friend_avatar(67890);
        mgr.process_pending();

        let avatar = mgr.get_cached_avatar(67890, AvatarSize::Medium).unwrap();
        assert_eq!(avatar.width, 64);
        assert_eq!(avatar.height, 64);
    }

    // =============================================================================
    // AVT-003: Get Large Avatar
    // Reference: https://partner.steamgames.com/doc/api/ISteamFriends#GetLargeFriendAvatar
    // =============================================================================

    #[test]
    fn avt_003_get_large_avatar() {
        let mut mgr = AvatarManager::new(12345);

        mgr.get_large_friend_avatar(67890);
        mgr.process_pending();

        let avatar = mgr.get_cached_avatar(67890, AvatarSize::Large).unwrap();
        assert_eq!(avatar.width, 128);
        assert_eq!(avatar.height, 128);
    }

    // =============================================================================
    // AVT-004: Avatar Image Data
    // Reference: https://partner.steamgames.com/doc/api/ISteamUtils#GetImageSize
    // Reference: https://partner.steamgames.com/doc/api/ISteamUtils#GetImageRGBA
    // =============================================================================

    #[test]
    fn avt_004_avatar_image_data() {
        let mut mgr = AvatarManager::new(12345);

        mgr.get_small_friend_avatar(67890);
        mgr.process_pending();

        let result = mgr.get_small_friend_avatar(67890);
        let handle = match result {
            AvatarResult::Handle(h) => h,
            _ => panic!("Expected Handle"),
        };

        // Get image size.
        let size = mgr.get_image_size(handle);
        assert_eq!(size, Some((32, 32)));

        // Get RGBA data.
        let rgba = mgr.get_image_rgba(handle);
        assert!(rgba.is_some());
        assert_eq!(rgba.unwrap().len(), 32 * 32 * 4);
    }

    // =============================================================================
    // AVT-005: Avatar Cache
    // =============================================================================

    #[test]
    fn avt_005_avatar_cache() {
        let mut mgr = AvatarManager::new(12345);

        // First request: cache miss.
        mgr.get_small_friend_avatar(67890);
        mgr.process_pending();

        let (hits, misses) = mgr.get_cache_stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 1);

        // Second request: cache hit.
        mgr.get_small_friend_avatar(67890);

        let (hits, misses) = mgr.get_cache_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }

    #[test]
    fn avt_005_cache_per_size() {
        let mut mgr = AvatarManager::new(12345);

        mgr.get_small_friend_avatar(67890);
        mgr.get_medium_friend_avatar(67890);
        mgr.get_large_friend_avatar(67890);
        mgr.process_pending();

        assert!(mgr.is_cached(67890, AvatarSize::Small));
        assert!(mgr.is_cached(67890, AvatarSize::Medium));
        assert!(mgr.is_cached(67890, AvatarSize::Large));
    }

    // =============================================================================
    // AVT-006: Avatar Callback
    // Reference: AvatarImageLoaded_t callback
    // =============================================================================

    #[test]
    fn avt_006_avatar_callback() {
        let mut mgr = AvatarManager::new(12345);

        mgr.get_small_friend_avatar(67890);
        mgr.process_pending();

        let callback = mgr.pop_callback();
        assert!(callback.is_some());

        let cb = callback.unwrap();
        assert_eq!(cb.steam_id, 67890);
        assert_eq!(cb.wide, 32);
        assert_eq!(cb.tall, 32);
        assert!(cb.image > 0);
    }

    // =============================================================================
    // AVT-007: Missing Avatar
    // =============================================================================

    #[test]
    fn avt_007_missing_avatar() {
        let mut mgr = AvatarManager::new(12345);

        mgr.set_no_avatar(67890);

        let result = mgr.get_small_friend_avatar(67890);
        assert_eq!(result, AvatarResult::NoAvatar);
    }

    #[test]
    fn avt_007_missing_all_sizes() {
        let mut mgr = AvatarManager::new(12345);

        mgr.set_no_avatar(67890);

        assert_eq!(mgr.get_small_friend_avatar(67890), AvatarResult::NoAvatar);
        assert_eq!(mgr.get_medium_friend_avatar(67890), AvatarResult::NoAvatar);
        assert_eq!(mgr.get_large_friend_avatar(67890), AvatarResult::NoAvatar);
    }

    // =============================================================================
    // AVT-008: Avatar Update
    // =============================================================================

    #[test]
    fn avt_008_avatar_update() {
        let mut mgr = AvatarManager::new(12345);

        // Cache avatar.
        mgr.get_small_friend_avatar(67890);
        mgr.process_pending();

        assert!(mgr.is_cached(67890, AvatarSize::Small));

        // Invalidate cache (avatar was updated).
        mgr.invalidate_cache(67890);

        assert!(!mgr.is_cached(67890, AvatarSize::Small));
    }

    #[test]
    fn avt_008_clear_no_avatar() {
        let mut mgr = AvatarManager::new(12345);

        mgr.set_no_avatar(67890);
        assert_eq!(mgr.get_small_friend_avatar(67890), AvatarResult::NoAvatar);

        // User set an avatar.
        mgr.clear_no_avatar(67890);

        // Should now be loadable.
        let result = mgr.get_small_friend_avatar(67890);
        assert_eq!(result, AvatarResult::NotCached);
    }

    // =============================================================================
    // AVT-009: Own Avatar
    // =============================================================================

    #[test]
    fn avt_009_own_avatar() {
        let mut mgr = AvatarManager::new(12345);

        let result = mgr.get_my_avatar(AvatarSize::Medium);
        assert_eq!(result, AvatarResult::NotCached);

        mgr.process_pending();

        let result = mgr.get_my_avatar(AvatarSize::Medium);
        match result {
            AvatarResult::Handle(_) => (),
            _ => panic!("Expected Handle for own avatar"),
        }
    }

    // =============================================================================
    // AVT-010: Avatar Fallback
    // =============================================================================

    #[test]
    fn avt_010_avatar_fallback() {
        let mgr = AvatarManager::new(12345);

        let default = mgr.get_default_avatar(AvatarSize::Medium);
        assert_eq!(default.width, 64);
        assert_eq!(default.height, 64);
        assert_eq!(default.handle, 0);

        // Verify it's a valid gray placeholder.
        assert_eq!(default.data.len(), 64 * 64 * 4);
        assert_eq!(default.data[0], 128); // Gray R
        assert_eq!(default.data[1], 128); // Gray G
        assert_eq!(default.data[2], 128); // Gray B
        assert_eq!(default.data[3], 255); // Full alpha
    }

    // =============================================================================
    // Additional Tests
    // =============================================================================

    #[test]
    fn concurrent_requests_same_user() {
        let mut mgr = AvatarManager::new(12345);

        // Request multiple sizes for same user.
        mgr.get_small_friend_avatar(67890);
        mgr.get_medium_friend_avatar(67890);
        mgr.get_large_friend_avatar(67890);

        mgr.process_pending();

        // Should have 3 callbacks.
        let mut count = 0;
        while mgr.pop_callback().is_some() {
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn multiple_users_avatars() {
        let mut mgr = AvatarManager::new(12345);

        mgr.get_small_friend_avatar(111);
        mgr.get_small_friend_avatar(222);
        mgr.get_small_friend_avatar(333);

        mgr.process_pending();

        assert!(mgr.is_cached(111, AvatarSize::Small));
        assert!(mgr.is_cached(222, AvatarSize::Small));
        assert!(mgr.is_cached(333, AvatarSize::Small));
    }

    #[test]
    fn avatar_byte_sizes() {
        assert_eq!(AvatarSize::Small.byte_size(), 32 * 32 * 4);
        assert_eq!(AvatarSize::Medium.byte_size(), 64 * 64 * 4);
        assert_eq!(AvatarSize::Large.byte_size(), 128 * 128 * 4);
    }
}
