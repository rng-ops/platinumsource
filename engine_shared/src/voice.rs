//! Voice chat implementation.
//!
//! # Valve Documentation Reference
//! - [Steam Voice](https://partner.steamgames.com/doc/features/voice)
//! - [ISteamUser Voice](https://partner.steamgames.com/doc/api/ISteamUser#StartVoiceRecording)
//!
//! # Features
//! - Voice recording control
//! - Voice data compression/decompression
//! - Push-to-talk support
//! - Voice activity detection

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::steam_id::SteamId;

/// Voice recording state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceRecordingState {
    /// Not recording.
    Idle,
    /// Recording is active.
    Recording,
    /// Recording paused.
    Paused,
}

/// Voice availability result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceResult {
    Ok,
    NotInitialized,
    NotRecording,
    NoData,
    BufferTooSmall,
    DataCorrupted,
    Restricted,
    UnsupportedCodec,
}

/// Voice data packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicePacket {
    /// Sender's Steam ID.
    pub sender: SteamId,
    /// Sequence number.
    pub sequence: u32,
    /// Compressed voice data.
    pub data: Vec<u8>,
    /// Timestamp (milliseconds since start of recording session).
    pub timestamp_ms: u32,
}

/// Voice quality settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceQuality {
    /// Low quality, low bandwidth.
    Low,
    /// Normal quality (default).
    Normal,
    /// High quality.
    High,
}

impl VoiceQuality {
    /// Get sample rate for this quality level.
    pub fn sample_rate(&self) -> u32 {
        match self {
            VoiceQuality::Low => 8000,
            VoiceQuality::Normal => 16000,
            VoiceQuality::High => 24000,
        }
    }
}

/// Voice recording manager.
pub struct VoiceRecorder {
    /// Current state.
    state: VoiceRecordingState,
    /// Whether initialized.
    initialized: bool,
    /// Recording start time.
    recording_start: Option<Instant>,
    /// Buffered voice data.
    buffer: VecDeque<Vec<u8>>,
    /// Max buffer size.
    max_buffer_size: usize,
    /// Next sequence number.
    next_sequence: u32,
    /// Voice quality setting.
    quality: VoiceQuality,
    /// Push-to-talk state.
    ptt_active: bool,
    /// Voice activity detection enabled.
    vad_enabled: bool,
    /// Last voice activity time.
    last_activity: Option<Instant>,
    /// Microphone muted.
    muted: bool,
}

impl VoiceRecorder {
    /// Create a new voice recorder.
    pub fn new() -> Self {
        VoiceRecorder {
            state: VoiceRecordingState::Idle,
            initialized: true,
            recording_start: None,
            buffer: VecDeque::new(),
            max_buffer_size: 100,
            next_sequence: 0,
            quality: VoiceQuality::Normal,
            ptt_active: false,
            vad_enabled: true,
            last_activity: None,
            muted: false,
        }
    }

    /// Check if initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get current recording state.
    pub fn state(&self) -> VoiceRecordingState {
        self.state
    }

    /// Start voice recording.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUser#StartVoiceRecording>
    pub fn start_recording(&mut self) -> VoiceResult {
        if !self.initialized {
            return VoiceResult::NotInitialized;
        }
        if self.muted {
            return VoiceResult::Restricted;
        }

        self.state = VoiceRecordingState::Recording;
        self.recording_start = Some(Instant::now());
        VoiceResult::Ok
    }

    /// Stop voice recording.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUser#StopVoiceRecording>
    pub fn stop_recording(&mut self) -> VoiceResult {
        if !self.initialized {
            return VoiceResult::NotInitialized;
        }

        self.state = VoiceRecordingState::Idle;
        VoiceResult::Ok
    }

    /// Get available voice data.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUser#GetAvailableVoice>
    pub fn get_available_voice(&self) -> (VoiceResult, u32) {
        if !self.initialized {
            return (VoiceResult::NotInitialized, 0);
        }
        if self.state != VoiceRecordingState::Recording {
            return (VoiceResult::NotRecording, 0);
        }
        if self.buffer.is_empty() {
            return (VoiceResult::NoData, 0);
        }

        let total_bytes: usize = self.buffer.iter().map(|b| b.len()).sum();
        (VoiceResult::Ok, total_bytes as u32)
    }

    /// Get voice data.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUser#GetVoice>
    pub fn get_voice(&mut self, max_bytes: usize) -> (VoiceResult, Vec<u8>) {
        if !self.initialized {
            return (VoiceResult::NotInitialized, Vec::new());
        }
        if self.state != VoiceRecordingState::Recording {
            return (VoiceResult::NotRecording, Vec::new());
        }
        if self.buffer.is_empty() {
            return (VoiceResult::NoData, Vec::new());
        }

        let mut result = Vec::new();
        while result.len() < max_bytes {
            if let Some(chunk) = self.buffer.pop_front() {
                if result.len() + chunk.len() <= max_bytes {
                    result.extend(chunk);
                } else {
                    // Put back partial chunk
                    let take = max_bytes - result.len();
                    result.extend(&chunk[..take]);
                    self.buffer.push_front(chunk[take..].to_vec());
                    break;
                }
            } else {
                break;
            }
        }

        if result.is_empty() {
            (VoiceResult::NoData, result)
        } else {
            (VoiceResult::Ok, result)
        }
    }

    /// Get optimal sample rate.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUser#GetVoiceOptimalSampleRate>
    pub fn get_optimal_sample_rate(&self) -> u32 {
        self.quality.sample_rate()
    }

    /// Set voice quality.
    pub fn set_quality(&mut self, quality: VoiceQuality) {
        self.quality = quality;
    }

    /// Get voice quality.
    pub fn quality(&self) -> VoiceQuality {
        self.quality
    }

    /// Set push-to-talk state.
    pub fn set_ptt(&mut self, active: bool) {
        self.ptt_active = active;
    }

    /// Check push-to-talk state.
    pub fn is_ptt_active(&self) -> bool {
        self.ptt_active
    }

    /// Enable/disable voice activity detection.
    pub fn set_vad_enabled(&mut self, enabled: bool) {
        self.vad_enabled = enabled;
    }

    /// Check if VAD is enabled.
    pub fn is_vad_enabled(&self) -> bool {
        self.vad_enabled
    }

    /// Mute microphone.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if muted && self.state == VoiceRecordingState::Recording {
            self.state = VoiceRecordingState::Paused;
        }
    }

    /// Check if muted.
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Simulate adding voice data (for testing).
    pub fn add_voice_data(&mut self, data: Vec<u8>) {
        if self.state == VoiceRecordingState::Recording && !self.muted {
            if self.buffer.len() < self.max_buffer_size {
                self.buffer.push_back(data);
            }
            self.last_activity = Some(Instant::now());
        }
    }

    /// Get recording duration.
    pub fn recording_duration(&self) -> Option<Duration> {
        self.recording_start.map(|start| start.elapsed())
    }

    /// Create a voice packet.
    pub fn create_packet(&mut self, sender: SteamId, data: Vec<u8>) -> VoicePacket {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);

        let timestamp_ms = self
            .recording_start
            .map(|s| s.elapsed().as_millis() as u32)
            .unwrap_or(0);

        VoicePacket {
            sender,
            sequence,
            data,
            timestamp_ms,
        }
    }
}

impl Default for VoiceRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Voice decompressor for playback.
pub struct VoiceDecompressor {
    /// Initialized state.
    initialized: bool,
    /// Output sample rate.
    output_sample_rate: u32,
    /// Jitter buffer.
    jitter_buffer: VecDeque<VoicePacket>,
    /// Max jitter buffer size.
    max_jitter_buffer: usize,
    /// Expected next sequence.
    expected_sequence: Option<u32>,
}

impl VoiceDecompressor {
    /// Create a new decompressor.
    pub fn new(sample_rate: u32) -> Self {
        VoiceDecompressor {
            initialized: true,
            output_sample_rate: sample_rate,
            jitter_buffer: VecDeque::new(),
            max_jitter_buffer: 20,
            expected_sequence: None,
        }
    }

    /// Decompress voice data.
    /// Reference: <https://partner.steamgames.com/doc/api/ISteamUser#DecompressVoice>
    pub fn decompress(&self, compressed: &[u8], output: &mut [u8]) -> (VoiceResult, usize) {
        if !self.initialized {
            return (VoiceResult::NotInitialized, 0);
        }
        if compressed.is_empty() {
            return (VoiceResult::NoData, 0);
        }

        // Mock decompression: in reality this would use OPUS codec
        // Assume 4:1 compression ratio for testing
        let decompressed_size = compressed.len() * 4;
        let bytes_written = decompressed_size.min(output.len());

        // Fill output with mock PCM data
        for i in 0..bytes_written {
            output[i] = ((i % 256) as u8).wrapping_add(compressed[i % compressed.len()]);
        }

        if output.len() < decompressed_size {
            (VoiceResult::BufferTooSmall, bytes_written)
        } else {
            (VoiceResult::Ok, bytes_written)
        }
    }

    /// Add packet to jitter buffer.
    pub fn add_packet(&mut self, packet: VoicePacket) {
        if self.jitter_buffer.len() < self.max_jitter_buffer {
            // Insert in order by sequence
            let insert_pos = self
                .jitter_buffer
                .iter()
                .position(|p| p.sequence > packet.sequence)
                .unwrap_or(self.jitter_buffer.len());
            self.jitter_buffer.insert(insert_pos, packet);
        }
    }

    /// Get next packet from jitter buffer.
    pub fn get_next_packet(&mut self) -> Option<VoicePacket> {
        self.jitter_buffer.pop_front()
    }

    /// Get jitter buffer size.
    pub fn buffer_size(&self) -> usize {
        self.jitter_buffer.len()
    }

    /// Get output sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.output_sample_rate
    }
}

impl Default for VoiceDecompressor {
    fn default() -> Self {
        Self::new(16000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_steam_id() -> SteamId {
        SteamId::from_account_id(12345)
    }

    // =============================================================================
    // VOX-001: Start Voice Recording
    // Reference: https://partner.steamgames.com/doc/api/ISteamUser#StartVoiceRecording
    // =============================================================================

    #[test]
    fn vox_001_start_recording() {
        let mut recorder = VoiceRecorder::new();

        assert_eq!(recorder.state(), VoiceRecordingState::Idle);

        let result = recorder.start_recording();
        assert_eq!(result, VoiceResult::Ok);
        assert_eq!(recorder.state(), VoiceRecordingState::Recording);
    }

    // =============================================================================
    // VOX-002: Stop Voice Recording
    // Reference: https://partner.steamgames.com/doc/api/ISteamUser#StopVoiceRecording
    // =============================================================================

    #[test]
    fn vox_002_stop_recording() {
        let mut recorder = VoiceRecorder::new();

        recorder.start_recording();
        assert_eq!(recorder.state(), VoiceRecordingState::Recording);

        let result = recorder.stop_recording();
        assert_eq!(result, VoiceResult::Ok);
        assert_eq!(recorder.state(), VoiceRecordingState::Idle);
    }

    // =============================================================================
    // VOX-003: Get Voice
    // Reference: https://partner.steamgames.com/doc/api/ISteamUser#GetVoice
    // =============================================================================

    #[test]
    fn vox_003_get_voice() {
        let mut recorder = VoiceRecorder::new();

        recorder.start_recording();
        recorder.add_voice_data(vec![1, 2, 3, 4, 5]);

        let (result, data) = recorder.get_voice(1024);
        assert_eq!(result, VoiceResult::Ok);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn vox_003_get_voice_no_data() {
        let mut recorder = VoiceRecorder::new();

        recorder.start_recording();

        let (result, _data) = recorder.get_voice(1024);
        assert_eq!(result, VoiceResult::NoData);
    }

    // =============================================================================
    // VOX-004: Decompress Voice
    // Reference: https://partner.steamgames.com/doc/api/ISteamUser#DecompressVoice
    // =============================================================================

    #[test]
    fn vox_004_decompress_voice() {
        let decompressor = VoiceDecompressor::new(16000);

        let compressed = vec![10, 20, 30, 40];
        let mut output = vec![0u8; 1024];

        let (result, bytes_written) = decompressor.decompress(&compressed, &mut output);
        assert_eq!(result, VoiceResult::Ok);
        assert!(bytes_written > 0);
    }

    #[test]
    fn vox_004_decompress_buffer_too_small() {
        let decompressor = VoiceDecompressor::new(16000);

        let compressed = vec![10, 20, 30, 40];
        let mut output = vec![0u8; 8]; // Too small

        let (result, _) = decompressor.decompress(&compressed, &mut output);
        assert_eq!(result, VoiceResult::BufferTooSmall);
    }

    // =============================================================================
    // VOX-005: Optimal Sample Rate
    // Reference: https://partner.steamgames.com/doc/api/ISteamUser#GetVoiceOptimalSampleRate
    // =============================================================================

    #[test]
    fn vox_005_optimal_sample_rate() {
        let mut recorder = VoiceRecorder::new();

        recorder.set_quality(VoiceQuality::Normal);
        assert_eq!(recorder.get_optimal_sample_rate(), 16000);

        recorder.set_quality(VoiceQuality::High);
        assert_eq!(recorder.get_optimal_sample_rate(), 24000);

        recorder.set_quality(VoiceQuality::Low);
        assert_eq!(recorder.get_optimal_sample_rate(), 8000);
    }

    // =============================================================================
    // VOX-006: Voice Available
    // Reference: https://partner.steamgames.com/doc/api/ISteamUser#GetAvailableVoice
    // =============================================================================

    #[test]
    fn vox_006_available_voice() {
        let mut recorder = VoiceRecorder::new();

        recorder.start_recording();
        recorder.add_voice_data(vec![1, 2, 3, 4, 5]);
        recorder.add_voice_data(vec![6, 7, 8]);

        let (result, bytes) = recorder.get_available_voice();
        assert_eq!(result, VoiceResult::Ok);
        assert_eq!(bytes, 8);
    }

    // =============================================================================
    // VOX-008: Push-to-Talk
    // =============================================================================

    #[test]
    fn vox_008_push_to_talk() {
        let mut recorder = VoiceRecorder::new();

        assert!(!recorder.is_ptt_active());

        recorder.set_ptt(true);
        assert!(recorder.is_ptt_active());

        recorder.set_ptt(false);
        assert!(!recorder.is_ptt_active());
    }

    // =============================================================================
    // VOX-009: Voice Activity Detection
    // =============================================================================

    #[test]
    fn vox_009_voice_activity_detection() {
        let mut recorder = VoiceRecorder::new();

        assert!(recorder.is_vad_enabled());

        recorder.set_vad_enabled(false);
        assert!(!recorder.is_vad_enabled());
    }

    // =============================================================================
    // VOX-010: Mute Self
    // =============================================================================

    #[test]
    fn vox_010_mute_self() {
        let mut recorder = VoiceRecorder::new();

        assert!(!recorder.is_muted());

        recorder.set_muted(true);
        assert!(recorder.is_muted());

        // Starting recording while muted should fail
        let result = recorder.start_recording();
        assert_eq!(result, VoiceResult::Restricted);
    }

    #[test]
    fn vox_010_mute_during_recording() {
        let mut recorder = VoiceRecorder::new();

        recorder.start_recording();
        assert_eq!(recorder.state(), VoiceRecordingState::Recording);

        recorder.set_muted(true);
        assert_eq!(recorder.state(), VoiceRecordingState::Paused);
    }

    // =============================================================================
    // Additional Tests
    // =============================================================================

    #[test]
    fn voice_packet_creation() {
        let mut recorder = VoiceRecorder::new();
        recorder.start_recording();

        let packet1 = recorder.create_packet(test_steam_id(), vec![1, 2, 3]);
        let packet2 = recorder.create_packet(test_steam_id(), vec![4, 5, 6]);

        assert_eq!(packet1.sequence, 0);
        assert_eq!(packet2.sequence, 1);
        assert_eq!(packet1.sender, test_steam_id());
    }

    #[test]
    fn jitter_buffer_ordering() {
        let mut decompressor = VoiceDecompressor::new(16000);

        // Add packets out of order
        let p2 = VoicePacket {
            sender: test_steam_id(),
            sequence: 2,
            data: vec![2],
            timestamp_ms: 200,
        };
        let p0 = VoicePacket {
            sender: test_steam_id(),
            sequence: 0,
            data: vec![0],
            timestamp_ms: 0,
        };
        let p1 = VoicePacket {
            sender: test_steam_id(),
            sequence: 1,
            data: vec![1],
            timestamp_ms: 100,
        };

        decompressor.add_packet(p2);
        decompressor.add_packet(p0);
        decompressor.add_packet(p1);

        // Should come out in order
        assert_eq!(decompressor.get_next_packet().unwrap().sequence, 0);
        assert_eq!(decompressor.get_next_packet().unwrap().sequence, 1);
        assert_eq!(decompressor.get_next_packet().unwrap().sequence, 2);
    }

    #[test]
    fn no_data_when_muted() {
        let mut recorder = VoiceRecorder::new();

        recorder.start_recording();
        recorder.set_muted(true);
        recorder.add_voice_data(vec![1, 2, 3]); // Should be ignored

        // Unmute and try to record again
        recorder.set_muted(false);
        recorder.start_recording();
        
        let (result, _) = recorder.get_available_voice();
        assert_eq!(result, VoiceResult::NoData);
    }
}
