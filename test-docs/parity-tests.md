# Source Engine Parity Tests Specification

**Purpose**: Comprehensive test specifications for verifying custom Source Engine builds and mods meet official server compatibility requirements. Designed for RLHF training, test framework generation, and coverage analysis.

---

## Table of Contents

1. [Authentication & Identity](#1-authentication--identity)
2. [VAC (Valve Anti-Cheat) Integration](#2-vac-valve-anti-cheat-integration)
3. [Steam ID System](#3-steam-id-system)
4. [Session Tickets & Tokens](#4-session-tickets--tokens)
5. [Lobby System](#5-lobby-system)
6. [Matchmaking](#6-matchmaking)
7. [Party System](#7-party-system)
8. [Chat System](#8-chat-system)
9. [Game State Integration (GSI)](#9-game-state-integration-gsi)
10. [Social Integration](#10-social-integration)
11. [Workshop Integration](#11-workshop-integration)
12. [Avatar System](#12-avatar-system)
13. [Account Linking](#13-account-linking)
14. [Network Protocol Compliance](#14-network-protocol-compliance)
15. [Server Browser & Query Protocol](#15-server-browser--query-protocol)
16. [Rich Presence](#16-rich-presence)
17. [Leaderboards & Statistics](#17-leaderboards--statistics)
18. [Cloud Saves](#18-cloud-saves)
19. [DLC & Entitlements](#19-dlc--entitlements)
20. [Voice Chat](#20-voice-chat)

---

## 1. Authentication & Identity

### 1.1 Steam Authentication Flow

#### Test Cases

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| AUTH-001 | Valid Steam Login | Client authenticates with valid Steam credentials via ISteamUser | Critical | Core Auth |
| AUTH-002 | Invalid Credentials Rejection | Server rejects connections without valid Steam auth | Critical | Security |
| AUTH-003 | Offline Mode Handling | Client handles Steam offline mode gracefully | High | Edge Cases |
| AUTH-004 | Auth Ticket Generation | ISteamUser::GetAuthSessionTicket() returns valid ticket | Critical | Core Auth |
| AUTH-005 | Auth Ticket Validation | Server validates ticket via ISteamUser::BeginAuthSession() | Critical | Core Auth |
| AUTH-006 | Auth Ticket Cancellation | ISteamUser::CancelAuthTicket() properly invalidates session | High | Session Mgmt |
| AUTH-007 | Auth Timeout Handling | Connection fails gracefully when Steam auth times out | High | Error Handling |
| AUTH-008 | Concurrent Auth Sessions | Multiple auth sessions from same account handled correctly | Medium | Edge Cases |
| AUTH-009 | Auth Callback Processing | ValidateAuthTicketResponse_t callback processed correctly | Critical | Callbacks |
| AUTH-010 | Encrypted App Ticket | ISteamUser::RequestEncryptedAppTicket() for secure auth | High | Security |

#### Verification Prompts

```
PROMPT: Generate a test that verifies Steam authentication ticket generation
follows the ISteamUser::GetAuthSessionTicket() specification, including:
- Ticket size validation (max 1024 bytes)
- Handle validity checking
- Callback registration for async completion
- Error code handling for k_EResultFail cases
```

```
PROMPT: Create an integration test that simulates the full client-server
authentication handshake:
1. Client generates auth ticket
2. Client sends ticket to server
3. Server calls BeginAuthSession with ticket
4. Server receives ValidateAuthTicketResponse_t callback
5. Server confirms/rejects client based on EAuthSessionResponse
```

### 1.2 Auth Response Codes

| Code | Enum Value | Description | Test Requirement |
|------|------------|-------------|------------------|
| OK | k_EAuthSessionResponseOK | Valid ticket | Accept connection |
| UserNotConnectedToSteam | k_EAuthSessionResponseUserNotConnectedToSteam | Client offline | Reject with message |
| NoLicenseOrExpired | k_EAuthSessionResponseNoLicenseOrExpired | No game license | Reject with purchase prompt |
| VACBanned | k_EAuthSessionResponseVACBanned | VAC banned | Reject immediately |
| LoggedInElseWhere | k_EAuthSessionResponseLoggedInElseWhere | Account in use | Disconnect gracefully |
| VACCheckTimedOut | k_EAuthSessionResponseVACCheckTimedOut | VAC timeout | Retry or reject |
| AuthTicketCanceled | k_EAuthSessionResponseAuthTicketCanceled | Ticket revoked | Disconnect |
| AuthTicketInvalidAlreadyUsed | k_EAuthSessionResponseAuthTicketInvalidAlreadyUsed | Replay attack | Reject + log |
| AuthTicketInvalid | k_EAuthSessionResponseAuthTicketInvalid | Malformed ticket | Reject |
| PublisherIssuedBan | k_EAuthSessionResponsePublisherIssuedBan | Game ban | Reject |

---

## 2. VAC (Valve Anti-Cheat) Integration

### 2.1 VAC Module Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| VAC-001 | VAC Module Load | VAC module loads on secure server startup | Critical | Core VAC |
| VAC-002 | VAC Secure Mode Flag | sv_vac_secure cvar correctly reported | Critical | Server Config |
| VAC-003 | VAC Ban Check | Server queries VAC ban status during auth | Critical | Security |
| VAC-004 | VAC Ban Rejection | VAC-banned players rejected from secure servers | Critical | Security |
| VAC-005 | Insecure Server Bypass | VAC-banned players allowed on insecure servers | High | Edge Cases |
| VAC-006 | VAC Status Query | ISteamUser::BIsBehindNAT() and GetPlayerSteamLevel() | Medium | API Coverage |
| VAC-007 | Memory Scan Compliance | Client memory regions accessible to VAC scan | Critical | Anti-Cheat |
| VAC-008 | Module Integrity Check | Loaded modules match expected signatures | Critical | Anti-Cheat |
| VAC-009 | VAC Kick Callback | Server receives VAC kick notification | High | Callbacks |
| VAC-010 | Real-time VAC Status | Mid-session VAC ban detection | High | Security |

### 2.2 VAC-Secured Server Requirements

```
PROMPT: Generate a server compliance test suite that verifies:
1. sv_vac_secure is set to 1 for official server parity
2. VAC module successfully initializes
3. All connecting clients undergo VAC validation
4. VAC ban status is checked against Steam backend
5. Banned players receive EAuthSessionResponseVACBanned
6. Server logs VAC-related events for auditing
```

### 2.3 VAC Bypass Detection Tests

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| VAC-DET-001 | Signature Scan Evasion | Detect attempts to hide modified memory | Critical |
| VAC-DET-002 | DLL Injection Detection | Detect foreign DLL injection | Critical |
| VAC-DET-003 | Hook Detection | Detect API hooking attempts | Critical |
| VAC-DET-004 | Debugger Presence | Detect attached debuggers on secure servers | High |
| VAC-DET-005 | Timing Attack Detection | Detect artificial delay in VAC responses | Medium |

---

## 3. Steam ID System

### 3.1 Steam ID Format Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| SID-001 | SteamID64 Parsing | Parse 64-bit Steam ID correctly | Critical | Core |
| SID-002 | SteamID32 Conversion | Convert between 32-bit and 64-bit formats | Critical | Core |
| SID-003 | SteamID3 Format | Parse [U:1:XXXXX] format | High | Compatibility |
| SID-004 | STEAM_X:Y:Z Format | Parse legacy STEAM_0:1:XXXXX format | High | Legacy |
| SID-005 | Account Type Detection | Identify Individual/Multiseat/GameServer/Clan | High | Type System |
| SID-006 | Universe Detection | Detect Public/Beta/Internal/Dev universe | Medium | Type System |
| SID-007 | Invalid SteamID Rejection | Reject malformed Steam IDs | Critical | Validation |
| SID-008 | Anonymous GameServer ID | Handle anonymous dedicated server IDs | Medium | Server Auth |
| SID-009 | Pending ID Handling | Handle k_steamIDNil during connection | High | Edge Cases |
| SID-010 | SteamID Uniqueness | Verify no collision in ID generation | Critical | Security |

### 3.2 Steam ID Structure

```
64-bit Steam ID Layout:
┌─────────────────────────────────────────────────────────────────┐
│ Universe (8) │ Type (4) │ Instance (20) │ Account ID (32)       │
└─────────────────────────────────────────────────────────────────┘

Test coverage requirements:
- Bit manipulation accuracy
- Overflow handling
- Sign extension correctness
- Cross-platform consistency
```

#### Verification Prompts

```
PROMPT: Generate a comprehensive SteamID parsing test suite that covers:
- All 8 universe values (Invalid, Public, Beta, Internal, Dev, RC)
- All account types (Invalid, Individual, Multiseat, GameServer, AnonGameServer, Pending, ContentServer, Clan, Chat, P2PSuperSeeder, AnonUser)
- Instance ID variations (Desktop, Console, Web)
- Conversion between all string formats
- Edge cases: zero ID, max ID, invalid combinations
```

---

## 4. Session Tickets & Tokens

### 4.1 Auth Session Ticket Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| TKT-001 | Ticket Generation | GetAuthSessionTicket returns valid handle | Critical | Core |
| TKT-002 | Ticket Size Bounds | Ticket within 0-1024 byte range | Critical | Validation |
| TKT-003 | Ticket Expiration | Tickets expire after reasonable duration | High | Security |
| TKT-004 | Ticket Single Use | Ticket cannot be reused after validation | Critical | Security |
| TKT-005 | Ticket Cancellation | CancelAuthTicket invalidates ticket | High | Lifecycle |
| TKT-006 | Callback Timing | GetAuthSessionTicketResponse_t arrives promptly | High | Performance |
| TKT-007 | Multiple Tickets | Can hold multiple active tickets | Medium | Concurrency |
| TKT-008 | Ticket for Server | GetAuthTicketForWebApi for web services | High | Web Integration |
| TKT-009 | Identity Binding | Ticket bound to requesting SteamID | Critical | Security |
| TKT-010 | Network Identity | SteamNetworkingIdentity from ticket | High | Networking |

### 4.2 Encrypted App Ticket Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| EAT-001 | Request Encrypted Ticket | RequestEncryptedAppTicket succeeds | High | Core |
| EAT-002 | Ticket Decryption | Server decrypts with app secret key | Critical | Security |
| EAT-003 | Custom Data Embedding | Optional user data in encrypted ticket | Medium | Features |
| EAT-004 | Ticket Verification | DecryptedAppTicket validation | Critical | Security |
| EAT-005 | App ID Verification | Ticket contains correct AppID | Critical | Validation |
| EAT-006 | Timestamp Validation | Ticket timestamp within acceptable window | High | Security |

#### Verification Prompts

```
PROMPT: Create a test harness for encrypted app ticket flow:
1. Client requests encrypted ticket with optional userData
2. Client sends encrypted blob to game server
3. Game server forwards to backend
4. Backend decrypts using SteamEncryptedAppTicket library
5. Backend extracts SteamID, AppID, ownership info
6. Backend returns validation result to game server
Include error cases: expired ticket, wrong app ID, tampered data
```

---

## 5. Lobby System

### 5.1 Lobby Lifecycle Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| LOB-001 | Create Public Lobby | CreateLobby with k_ELobbyTypePublic | Critical | Core |
| LOB-002 | Create Private Lobby | CreateLobby with k_ELobbyTypePrivate | High | Core |
| LOB-003 | Create Friends-Only Lobby | CreateLobby with k_ELobbyTypeFriendsOnly | High | Core |
| LOB-004 | Create Invisible Lobby | CreateLobby with k_ELobbyTypeInvisible | Medium | Core |
| LOB-005 | Join Lobby by ID | JoinLobby with valid CSteamID | Critical | Core |
| LOB-006 | Leave Lobby | LeaveLobby cleanup | Critical | Lifecycle |
| LOB-007 | Lobby Member Limit | SetLobbyMemberLimit enforcement | High | Configuration |
| LOB-008 | Lobby Owner Transfer | New owner on disconnect | High | Ownership |
| LOB-009 | Lobby Deletion | Lobby removed when empty | High | Cleanup |
| LOB-010 | Lobby Search | RequestLobbyList with filters | Critical | Discovery |

### 5.2 Lobby Data & Metadata Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| LOB-DATA-001 | Set Lobby Data | SetLobbyData key-value pairs | Critical | Data |
| LOB-DATA-002 | Get Lobby Data | GetLobbyData retrieval | Critical | Data |
| LOB-DATA-003 | Lobby Data Limits | 255 char key, 8KB value limits | High | Validation |
| LOB-DATA-004 | Member Data | SetLobbyMemberData per-member | High | Data |
| LOB-DATA-005 | Data Sync | LobbyDataUpdate_t callback | Critical | Callbacks |
| LOB-DATA-006 | Game Server Info | SetLobbyGameServer association | Critical | Server Link |
| LOB-DATA-007 | Lobby Metadata Query | GetLobbyDataByIndex iteration | Medium | Enumeration |
| LOB-DATA-008 | Data Persistence | Data survives member churn | High | Reliability |

### 5.3 Lobby Matchmaking Filters

| Filter Type | Description | Test Requirement |
|-------------|-------------|------------------|
| StringFilter | Exact/substring match on lobby data | Verify all comparison types |
| NumericalFilter | <, <=, ==, >=, > on numeric data | Boundary condition tests |
| SlotsAvailable | Filter by open slots | Accuracy verification |
| Distance | Geographic filtering | Latency correlation |
| ResultCount | Limit returned lobbies | Pagination handling |
| NearValue | Sort by proximity to value | Ordering verification |

#### Verification Prompts

```
PROMPT: Generate a lobby integration test suite covering:
1. Full lobby lifecycle: create → configure → invite → join → play → leave
2. All lobby types (public, private, friends-only, invisible)
3. Member limit enforcement (test boundary: max-1, max, max+1)
4. Owner crash/disconnect with automatic transfer
5. Lobby data replication to all members
6. Search filters with complex query combinations
7. Game server association and retrieval
```

---

## 6. Matchmaking

### 6.1 Server Browser Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| MM-001 | Request Internet Servers | RequestInternetServerList | Critical | Discovery |
| MM-002 | Request LAN Servers | RequestLANServerList | High | Discovery |
| MM-003 | Request Friends Servers | RequestFriendsServerList | High | Social |
| MM-004 | Request Favorites | RequestFavoritesServerList | Medium | User Data |
| MM-005 | Request History | RequestHistoryServerList | Medium | User Data |
| MM-006 | Request Spectator | RequestSpectatorServerList | Low | Features |
| MM-007 | Filter Application | MatchMakingKeyValuePair filters | Critical | Filtering |
| MM-008 | Server Response Parsing | servernetadr_t parsing | Critical | Protocol |
| MM-009 | Ping Measurement | Server latency calculation | High | Performance |
| MM-010 | Server Rules Query | ISteamMatchmakingRules | High | Protocol |

### 6.2 Server Query Protocol (A2S)

| Query Type | Description | Test Cases |
|------------|-------------|------------|
| A2S_INFO | Server info query | Response parsing, timeout handling |
| A2S_PLAYER | Player list query | Multi-packet response, player data |
| A2S_RULES | Server rules query | Cvar enumeration, value parsing |
| A2S_SERVERQUERY_GETCHALLENGE | Challenge request | Anti-spoof verification |

#### Verification Prompts

```
PROMPT: Create A2S protocol compliance tests:
1. Send A2S_INFO (0x54) and parse response
2. Handle challenge-response for A2S_PLAYER
3. Verify multi-packet reassembly for large responses
4. Test timeout behavior (no response, partial response)
5. Validate all response fields against Valve specification
6. Test packet size limits (1400 bytes single, multi-packet)
```

---

## 7. Party System

### 7.1 Party Management Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| PTY-001 | Create Party | Initialize party with leader | Critical | Core |
| PTY-002 | Invite to Party | Send party invite to friend | Critical | Invites |
| PTY-003 | Accept Invite | Join party from invite | Critical | Invites |
| PTY-004 | Decline Invite | Reject party invite | High | Invites |
| PTY-005 | Leave Party | Member leaves gracefully | Critical | Lifecycle |
| PTY-006 | Kick from Party | Leader removes member | High | Moderation |
| PTY-007 | Party Leader Change | Transfer leadership | High | Ownership |
| PTY-008 | Party Auto-Disband | Empty party cleanup | High | Cleanup |
| PTY-009 | Party Size Limit | Maximum member enforcement | High | Validation |
| PTY-010 | Cross-Game Party | Party persistence across games | Medium | Platform |

### 7.2 Party State Synchronization

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| PTY-SYNC-001 | Member List Sync | All members see same roster | Critical |
| PTY-SYNC-002 | Ready State | Ready-up synchronization | High |
| PTY-SYNC-003 | Game Mode Selection | Shared game mode state | High |
| PTY-SYNC-004 | Map Voting | Party map vote system | Medium |
| PTY-SYNC-005 | Party Chat | Internal party messaging | High |

---

## 8. Chat System

### 8.1 In-Game Chat Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| CHAT-001 | Global Chat | All-player broadcast | Critical | Core |
| CHAT-002 | Team Chat | Team-only messages | Critical | Core |
| CHAT-003 | Squad Chat | Squad-only messages | High | Features |
| CHAT-004 | Private Message | Player-to-player DM | High | Features |
| CHAT-005 | Console Commands | Chat-based commands (/all, /team) | High | UX |
| CHAT-006 | Message Length Limit | Truncation at max length | High | Validation |
| CHAT-007 | Rate Limiting | Spam prevention | High | Anti-Abuse |
| CHAT-008 | Mute Player | Hide messages from muted player | High | Moderation |
| CHAT-009 | Unicode Support | International character handling | Medium | I18n |
| CHAT-010 | Chat History | Recent message buffer | Medium | Features |

### 8.2 Steam Friend Chat Integration

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| CHAT-STM-001 | Friend Message Send | ReplyToFriendMessage | High | Social |
| CHAT-STM-002 | Friend Message Receive | GameConnectedFriendChatMsg_t | High | Callbacks |
| CHAT-STM-003 | Clan Chat | Clan/group messaging | Medium | Social |
| CHAT-STM-004 | Rich Presence Chat | Game invite from chat | High | Social |

### 8.3 Chat Moderation Tests

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| CHAT-MOD-001 | Profanity Filter | Blocked word replacement | High |
| CHAT-MOD-002 | Link Filtering | URL stripping/warning | Medium |
| CHAT-MOD-003 | Admin Mute | Server admin mutes player | High |
| CHAT-MOD-004 | Report Player | Chat abuse reporting | High |
| CHAT-MOD-005 | Chat Log Export | Server-side chat logging | Medium |

---

## 9. Game State Integration (GSI)

### 9.1 GSI Payload Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| GSI-001 | Provider Block | Game identity information | Critical | Core |
| GSI-002 | Map Block | Current map state | Critical | Core |
| GSI-003 | Player Block | Local player data | Critical | Core |
| GSI-004 | All Players Block | All player states | High | Spectator |
| GSI-005 | Round Block | Round state and timing | High | Core |
| GSI-006 | Phase Countdowns | Bomb timer, round timer | High | Timing |
| GSI-007 | Previously Block | Changed values tracking | High | Optimization |
| GSI-008 | Added Block | New value tracking | Medium | Optimization |
| GSI-009 | Auth Token | Token-based validation | Critical | Security |
| GSI-010 | HTTP POST Delivery | Payload delivery to endpoint | Critical | Transport |

### 9.2 GSI Configuration Tests

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| GSI-CFG-001 | Config File Parse | gamestate_integration_*.cfg | Critical |
| GSI-CFG-002 | URI Validation | Valid HTTP/HTTPS endpoints | High |
| GSI-CFG-003 | Throttle Settings | buffer/throttle timing | High |
| GSI-CFG-004 | Data Subscriptions | Selective data blocks | High |
| GSI-CFG-005 | Multiple Endpoints | Multi-config support | Medium |

### 9.3 GSI Payload Structure (CS:GO/CS2 Example)

```json
{
  "provider": {
    "name": "Counter-Strike 2",
    "appid": 730,
    "version": 14000,
    "steamid": "76561198012345678",
    "timestamp": 1700000000
  },
  "map": {
    "mode": "competitive",
    "name": "de_dust2",
    "phase": "live",
    "round": 5,
    "team_ct": { "score": 3 },
    "team_t": { "score": 2 }
  },
  "player": {
    "steamid": "76561198012345678",
    "clan": "TestClan",
    "name": "PlayerName",
    "team": "CT",
    "activity": "playing",
    "state": {
      "health": 100,
      "armor": 100,
      "helmet": true,
      "money": 4750,
      "round_kills": 2
    }
  },
  "auth": {
    "token": "your_auth_token_here"
  }
}
```

#### Verification Prompts

```
PROMPT: Generate a GSI receiver test harness that:
1. Accepts HTTP POST on configurable port
2. Validates auth token against expected value
3. Parses all standard GSI blocks (provider, map, player, round)
4. Tracks state changes using "previously" block
5. Handles throttled/buffered payloads correctly
6. Logs received data for test verification
7. Simulates timeout and connection failure scenarios
```

---

## 10. Social Integration

### 10.1 Friends System Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| SOC-001 | Get Friends List | GetFriendCount + GetFriendByIndex | Critical | Core |
| SOC-002 | Friend Relationship | GetFriendRelationship types | High | Core |
| SOC-003 | Friend Persona Name | GetFriendPersonaName | Critical | Display |
| SOC-004 | Friend Game Info | GetFriendGamePlayed | High | Presence |
| SOC-005 | Friend State | GetFriendPersonaState (online/away/etc) | High | Presence |
| SOC-006 | Friend Rich Presence | GetFriendRichPresence | High | Presence |
| SOC-007 | Invite Friend | InviteUserToGame | Critical | Social |
| SOC-008 | Accept Invite | GameRichPresenceJoinRequested_t | Critical | Callbacks |
| SOC-009 | Block Player | Relationship state changes | Medium | Moderation |
| SOC-010 | Recently Played | GetCoplayFriend* APIs | Medium | History |

### 10.2 Friend Relationship Types

| Type | Enum Value | Description | Test Requirement |
|------|------------|-------------|------------------|
| None | k_EFriendRelationshipNone | Not friends | Verify invite flow |
| Blocked | k_EFriendRelationshipBlocked | User blocked | Verify no interaction |
| RequestRecipient | k_EFriendRelationshipRequestRecipient | Pending request received | UI state test |
| Friend | k_EFriendRelationshipFriend | Confirmed friends | All friend features |
| RequestInitiator | k_EFriendRelationshipRequestInitiator | Pending request sent | UI state test |
| Ignored | k_EFriendRelationshipIgnored | Ignored user | Verify filtering |
| IgnoredFriend | k_EFriendRelationshipIgnoredFriend | Ignored but friend | Edge case |

### 10.3 Clan/Group Integration Tests

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| SOC-CLAN-001 | Get Clan Count | GetClanCount | High |
| SOC-CLAN-002 | Get Clan Details | GetClanByIndex, GetClanName | High |
| SOC-CLAN-003 | Clan Officer List | GetClanOfficerCount | Medium |
| SOC-CLAN-004 | Clan Activity | GetClanActivityCounts | Medium |
| SOC-CLAN-005 | Clan Chat | OpenClanChatWindowInSteam | Low |

---

## 11. Workshop Integration

### 11.1 Workshop Item Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| WKS-001 | Create Item | CreateItem for new upload | Critical | Publishing |
| WKS-002 | Update Item | SubmitItemUpdate with changes | Critical | Publishing |
| WKS-003 | Query Items | CreateQueryAllUGCRequest | Critical | Discovery |
| WKS-004 | Subscribe to Item | SubscribeItem | Critical | User Action |
| WKS-005 | Unsubscribe Item | UnsubscribeItem | High | User Action |
| WKS-006 | Download Item | DownloadItem | Critical | Content |
| WKS-007 | Get Item State | GetItemState flags | High | State |
| WKS-008 | Get Install Info | GetItemInstallInfo | Critical | Installation |
| WKS-009 | Item Metadata | GetQueryUGCResult | High | Data |
| WKS-010 | Vote on Item | SetUserItemVote | Medium | Engagement |

### 11.2 Workshop Item States

| State Flag | Description | Test Requirement |
|------------|-------------|------------------|
| k_EItemStateNone | Not tracked | Initial state verification |
| k_EItemStateSubscribed | User subscribed | Subscription flow |
| k_EItemStateLegacyItem | Legacy item | Backward compatibility |
| k_EItemStateInstalled | Fully installed | Installation verification |
| k_EItemStateNeedsUpdate | Update available | Update detection |
| k_EItemStateDownloading | Currently downloading | Progress tracking |
| k_EItemStateDownloadPending | Queued for download | Queue management |

### 11.3 Workshop Content Verification

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| WKS-VER-001 | Content Hash | Verify downloaded content integrity | Critical |
| WKS-VER-002 | File Size Match | Actual size matches metadata | High |
| WKS-VER-003 | File Permissions | Correct read/execute permissions | High |
| WKS-VER-004 | Dependency Resolution | Required items downloaded | High |
| WKS-VER-005 | Version Matching | Installed version is current | High |

#### Verification Prompts

```
PROMPT: Generate a Workshop integration test suite:
1. Subscribe to a known test item
2. Wait for download completion callback
3. Verify GetItemState returns k_EItemStateInstalled
4. Get installation path via GetItemInstallInfo
5. Verify content files exist and are readable
6. Check content hash matches published hash
7. Test update detection when item is modified
8. Unsubscribe and verify cleanup
```

---

## 12. Avatar System

### 12.1 Avatar Retrieval Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| AVT-001 | Get Small Avatar | GetSmallFriendAvatar (32x32) | Critical | Core |
| AVT-002 | Get Medium Avatar | GetMediumFriendAvatar (64x64) | Critical | Core |
| AVT-003 | Get Large Avatar | GetLargeFriendAvatar (128x128) | High | Core |
| AVT-004 | Avatar Image Data | GetImageSize + GetImageRGBA | Critical | Rendering |
| AVT-005 | Avatar Cache | Repeated requests use cache | High | Performance |
| AVT-006 | Avatar Callback | AvatarImageLoaded_t for async load | Critical | Callbacks |
| AVT-007 | Missing Avatar | Handle users without avatar | High | Edge Cases |
| AVT-008 | Avatar Update | Detect avatar changes | Medium | Freshness |
| AVT-009 | Own Avatar | Get local user's avatar | High | Core |
| AVT-010 | Avatar Fallback | Default avatar for missing | High | UX |

### 12.2 Avatar Image Handling

```
PROMPT: Create avatar handling tests covering:
1. Request all three sizes (small/medium/large)
2. Handle -1 return (image not cached, wait for callback)
3. Handle 0 return (no avatar set, use default)
4. Process AvatarImageLoaded_t callback
5. Extract RGBA data via GetImageRGBA
6. Verify image dimensions match expected size
7. Test concurrent requests for multiple users
8. Memory management for image data
```

---

## 13. Account Linking

### 13.1 Third-Party Account Linking Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| LINK-001 | Web API Key Validation | Authenticate publisher API key | Critical | Security |
| LINK-002 | Ownership Verification | ISteamUser::UserHasLicenseForApp | Critical | DRM |
| LINK-003 | DLC Check | BIsSubscribedApp for DLC AppID | High | Entitlements |
| LINK-004 | Free Weekend | BIsSubscribedFromFreeWeekend | Medium | Promotions |
| LINK-005 | Family Sharing | BIsSubscribedFromFamilySharing | Medium | Sharing |
| LINK-006 | Micro-transactions | RequestPrices + StartPurchase | High | Economy |
| LINK-007 | Inventory | GetItemsByID for Steam Inventory | High | Items |
| LINK-008 | Web Ticket Auth | GetAuthTicketForWebApi | Critical | Web Services |
| LINK-009 | Publisher ID Mapping | Link SteamID to publisher account | High | Cross-Platform |
| LINK-010 | UnLink Account | Remove publisher association | Medium | Account Mgmt |

### 13.2 Web API Integration

| API Endpoint | Description | Test Requirements |
|--------------|-------------|-------------------|
| ISteamUserAuth/AuthenticateUserTicket | Validate game ticket | Response parsing, error codes |
| ISteamUser/GetPlayerSummaries | Get user profile | Data completeness, privacy |
| IPlayerService/GetOwnedGames | Get user's games | Pagination, filtering |
| ISteamMicroTxn/InitTxn | Start purchase | Transaction flow |
| ISteamEconomy/GetAssetPrices | Get item prices | Currency handling |

---

## 14. Network Protocol Compliance

### 14.1 Source Engine Network Protocol Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| NET-001 | Connection Handshake | 4-way handshake completion | Critical | Core |
| NET-002 | Challenge-Response | Anti-spoof challenge | Critical | Security |
| NET-003 | Packet Fragmentation | Large packet handling | High | Transport |
| NET-004 | Reliable Channel | Ordered delivery guarantee | Critical | Transport |
| NET-005 | Unreliable Channel | Best-effort delivery | Critical | Transport |
| NET-006 | Sequencing | Packet sequence numbers | Critical | Ordering |
| NET-007 | Acknowledgment | ACK/NACK handling | Critical | Reliability |
| NET-008 | Congestion Control | Rate limiting/throttling | High | Performance |
| NET-009 | Encryption | Packet encryption (if enabled) | High | Security |
| NET-010 | Compression | Delta compression | High | Bandwidth |

### 14.2 Tick Rate & Timing

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| NET-TICK-001 | Server Tick Rate | Consistent tick timing | Critical |
| NET-TICK-002 | Client Update Rate | cl_updaterate compliance | High |
| NET-TICK-003 | Command Rate | cl_cmdrate compliance | High |
| NET-TICK-004 | Interpolation | cl_interp timing | High |
| NET-TICK-005 | Lag Compensation | Server-side rewind | High |

### 14.3 Protocol Version Compatibility

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| NET-VER-001 | Version Handshake | Protocol version exchange | Critical |
| NET-VER-002 | Version Mismatch | Graceful rejection | High |
| NET-VER-003 | Backward Compat | Old client support (if any) | Medium |
| NET-VER-004 | Forward Compat | Unknown fields ignored | Medium |

---

## 15. Server Browser & Query Protocol

### 15.1 Master Server Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| MST-001 | Heartbeat Send | Regular heartbeat to master | Critical | Registration |
| MST-002 | Heartbeat Response | Parse master response | High | Protocol |
| MST-003 | Server Deregister | Clean shutdown notification | High | Lifecycle |
| MST-004 | Region Filter | Geographic filtering | Medium | Discovery |
| MST-005 | Tag Filtering | Custom tag queries | High | Discovery |

### 15.2 A2S Query Response Tests

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| A2S-001 | A2S_INFO Response | Correct server info format | Critical |
| A2S-002 | A2S_PLAYER Response | Player list accuracy | High |
| A2S-003 | A2S_RULES Response | Server cvars list | High |
| A2S-004 | Challenge Protocol | Anti-reflection challenge | Critical |
| A2S-005 | Rate Limiting | Query flood protection | High |

---

## 16. Rich Presence

### 16.1 Rich Presence Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| RP-001 | Set Rich Presence | SetRichPresence key-value | Critical | Core |
| RP-002 | Clear Rich Presence | ClearRichPresence | High | Lifecycle |
| RP-003 | Get Friend RP | GetFriendRichPresence | High | Social |
| RP-004 | RP Key Count | GetFriendRichPresenceKeyCount | Medium | Enumeration |
| RP-005 | Steam Group Key | #group token for joinable | High | Social |
| RP-006 | Connect String | +connect for join game | Critical | Joining |
| RP-007 | Status String | #status for display text | High | Display |
| RP-008 | Localization | Localized RP strings | Medium | I18n |
| RP-009 | RP Update Rate | Rate limiting on updates | High | Performance |
| RP-010 | RP Callback | FriendRichPresenceUpdate_t | High | Callbacks |

### 16.2 Rich Presence Keys

| Key | Description | Test Requirement |
|-----|-------------|------------------|
| status | Display string in friends list | Verify display format |
| connect | Server connection string | Verify +connect parsing |
| steam_display | Localization token | Verify localization lookup |
| steam_player_group | Group identifier | Verify grouping display |
| steam_player_group_size | Group member count | Verify numeric display |

---

## 17. Leaderboards & Statistics

### 17.1 Leaderboard Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| LDB-001 | Find Leaderboard | FindLeaderboard by name | Critical | Core |
| LDB-002 | Find or Create | FindOrCreateLeaderboard | High | Core |
| LDB-003 | Upload Score | UploadLeaderboardScore | Critical | Core |
| LDB-004 | Download Entries | DownloadLeaderboardEntries | Critical | Retrieval |
| LDB-005 | Entry Range | Global/friends/around user | High | Filtering |
| LDB-006 | Score Details | Attached UGC handle | Medium | Extended |
| LDB-007 | Leaderboard Count | GetLeaderboardEntryCount | Medium | Metadata |
| LDB-008 | Sort Method | Ascending/descending | High | Configuration |
| LDB-009 | Display Type | Numeric/seconds/milliseconds | High | Display |
| LDB-010 | Keep Best | Force update vs keep best | High | Behavior |

### 17.2 Statistics Tests

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| STAT-001 | Request Stats | RequestCurrentStats | Critical |
| STAT-002 | Get Int Stat | GetStat for integers | Critical |
| STAT-003 | Get Float Stat | GetStat for floats | Critical |
| STAT-004 | Set Stat | SetStat modification | Critical |
| STAT-005 | Store Stats | StoreStats persistence | Critical |
| STAT-006 | Reset Stats | ResetAllStats | Medium |
| STAT-007 | User Stats | GetUserStat for other users | High |
| STAT-008 | Achievement Progress | IndicateAchievementProgress | High |

---

## 18. Cloud Saves

### 18.1 Steam Cloud Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| CLD-001 | File Write | FileWrite to cloud | Critical | Core |
| CLD-002 | File Read | FileRead from cloud | Critical | Core |
| CLD-003 | File Delete | FileDelete from cloud | High | Management |
| CLD-004 | File Exists | FileExists check | High | Verification |
| CLD-005 | File Count | GetFileCount enumeration | Medium | Enumeration |
| CLD-006 | Quota Check | GetQuota remaining space | High | Limits |
| CLD-007 | Cloud Enabled | IsCloudEnabledForAccount/App | High | Configuration |
| CLD-008 | Conflict Resolution | Cloud sync conflicts | High | Edge Cases |
| CLD-009 | Async Write | FileWriteAsync | Medium | Performance |
| CLD-010 | File Share | FileShare for UGC | Medium | Sharing |

---

## 19. DLC & Entitlements

### 19.1 DLC Verification Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| DLC-001 | Check DLC Owned | BIsDlcInstalled | Critical | Core |
| DLC-002 | Get DLC Count | GetDLCCount | High | Enumeration |
| DLC-003 | Get DLC Data | GetDLCDataByIndex | High | Metadata |
| DLC-004 | Install DLC | InstallDLC trigger | Medium | Installation |
| DLC-005 | Uninstall DLC | UninstallDLC | Medium | Management |
| DLC-006 | DLC Download | DlcInstalled_t callback | High | Callbacks |
| DLC-007 | Early Access | IsSubscribedFromFreeWeekend | Medium | Promotions |
| DLC-008 | App ID Chains | GetAppInstallDir for DLC | High | Paths |

### 19.2 License Verification

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| LIC-001 | Game Ownership | BIsSubscribedApp | Critical |
| LIC-002 | License Type | Permanent/temporary/borrowed | High |
| LIC-003 | Family Sharing | BIsSubscribedFromFamilySharing | High |
| LIC-004 | Timed License | Free weekend expiry | Medium |
| LIC-005 | VAC Game Ban | Game-specific bans | High |

---

## 20. Voice Chat

### 20.1 Voice Communication Tests

| Test ID | Name | Description | Priority | Coverage Area |
|---------|------|-------------|----------|---------------|
| VOX-001 | Start Voice | StartVoiceRecording | Critical | Core |
| VOX-002 | Stop Voice | StopVoiceRecording | Critical | Core |
| VOX-003 | Get Voice | GetVoice | Critical | Core |
| VOX-004 | Decompress Voice | DecompressVoice | Critical | Playback |
| VOX-005 | Voice Optimal Rate | GetVoiceOptimalSampleRate | High | Quality |
| VOX-006 | Voice Available | GetAvailableVoice | High | Buffering |
| VOX-007 | Voice Quality | OPUS codec quality | Medium | Quality |
| VOX-008 | Push-to-Talk | PTT key handling | High | Input |
| VOX-009 | Voice Activity | VAD detection | High | Features |
| VOX-010 | Mute Self | Microphone mute state | High | UX |

### 20.2 Voice Networking

| Test ID | Name | Description | Priority |
|---------|------|-------------|----------|
| VOX-NET-001 | Voice Packet Format | Correct header format | Critical |
| VOX-NET-002 | Voice Relay | Server voice relay | High |
| VOX-NET-003 | Voice P2P | Direct voice if available | Medium |
| VOX-NET-004 | Voice Quality Adapt | Bandwidth adaptation | Medium |
| VOX-NET-005 | Voice Jitter Buffer | Smooth playback | High |

---

## Test Framework Generation Prompts

### Prompt Template: Unit Test Generation

```
PROMPT: Generate a comprehensive unit test for [FEATURE_NAME] that:

Context:
- Tests the [SPECIFIC_API] from ISteam[Interface]
- Must handle both success and failure paths
- Should mock Steam API callbacks where needed

Requirements:
1. Test initialization and cleanup
2. Test normal operation flow
3. Test error conditions: [LIST_ERROR_CODES]
4. Test timeout behavior
5. Test concurrent access (if applicable)
6. Test state transitions

Expected Coverage:
- Line coverage: >90%
- Branch coverage: >80%
- All error codes handled

Output Format:
- Rust test module with #[test] functions
- Helper functions for common setup/teardown
- Mock implementations for Steam callbacks
```

### Prompt Template: Integration Test Generation

```
PROMPT: Generate an integration test for [SUBSYSTEM] that verifies
end-to-end functionality:

Test Scenario: [DESCRIBE_SCENARIO]

Steps:
1. [STEP_1]
2. [STEP_2]
...
N. [STEP_N]

Verification Points:
- [CHECKPOINT_1]
- [CHECKPOINT_2]
...

Environment Requirements:
- Steam client running: [YES/NO]
- Network access: [YES/NO]
- Test Steam account: [REQUIRED/OPTIONAL]

Expected Behavior:
- Success criteria: [DESCRIBE]
- Failure modes: [LIST]

Timeout: [DURATION]
Retry Policy: [DESCRIBE]
```

### Prompt Template: Compliance Test Suite

```
PROMPT: Generate a compliance test suite for connecting to official
[GAME_NAME] servers that verifies:

Authentication:
- [ ] Valid Steam authentication ticket
- [ ] Correct protocol version
- [ ] VAC secure status check

Network Protocol:
- [ ] Handshake sequence
- [ ] Packet format compliance
- [ ] Timing requirements

Game State:
- [ ] Required cvars match expected values
- [ ] Content validation (maps, models)
- [ ] Version compatibility

Generate test code that:
1. Simulates each compliance check
2. Logs failures with specific reasons
3. Produces machine-readable results
4. Can be run in CI/CD pipeline
```

---

## Coverage Metrics

### Overall Coverage Requirements

| Area | Line Coverage | Branch Coverage | Integration Tests |
|------|---------------|-----------------|-------------------|
| Authentication | 95% | 90% | Full flow tested |
| VAC Integration | 90% | 85% | Secure server tested |
| Steam ID | 100% | 95% | All formats tested |
| Lobby System | 90% | 85% | Multi-client tested |
| Chat System | 85% | 80% | All channels tested |
| Workshop | 80% | 75% | Upload/download tested |
| Voice | 80% | 75% | Record/playback tested |
| Network Protocol | 95% | 90% | Full roundtrip tested |

### Test Prioritization Matrix

| Priority | Description | SLA | Example |
|----------|-------------|-----|---------|
| P0 - Critical | Security/Auth failures | Must pass 100% | AUTH-*, VAC-* |
| P1 - High | Core functionality | Must pass 99% | LOB-*, NET-* |
| P2 - Medium | Features | Must pass 95% | CHAT-*, SOC-* |
| P3 - Low | Nice-to-have | Best effort | Edge cases |

---

## RLHF Training Data Format

### Test Case Training Example

```json
{
  "id": "AUTH-001",
  "prompt": "Generate a test that verifies Steam authentication ticket generation",
  "expected_output": {
    "test_code": "...",
    "assertions": ["ticket_size <= 1024", "handle != 0"],
    "error_handling": ["k_EResultFail", "timeout"],
    "coverage_areas": ["authentication", "security"]
  },
  "quality_signals": {
    "completeness": 0.95,
    "correctness": 1.0,
    "style_compliance": 0.9
  }
}
```

### Rating Criteria for Generated Tests

| Criterion | Weight | Description |
|-----------|--------|-------------|
| Correctness | 30% | Test accurately verifies specification |
| Completeness | 25% | Covers all required scenarios |
| Edge Cases | 20% | Handles boundary conditions |
| Code Quality | 15% | Clean, maintainable test code |
| Performance | 10% | Efficient test execution |

---

## Appendix A: Steam API Callback Reference

| Callback | Interface | Description |
|----------|-----------|-------------|
| ValidateAuthTicketResponse_t | ISteamUser | Auth ticket validated |
| GetAuthSessionTicketResponse_t | ISteamUser | Ticket ready |
| SteamServersConnected_t | ISteamUser | Connected to Steam |
| SteamServerConnectFailure_t | ISteamUser | Connection failed |
| LobbyCreated_t | ISteamMatchmaking | Lobby created |
| LobbyEnter_t | ISteamMatchmaking | Joined lobby |
| LobbyDataUpdate_t | ISteamMatchmaking | Lobby data changed |
| LobbyChatUpdate_t | ISteamMatchmaking | Member change |
| PersonaStateChange_t | ISteamFriends | Friend state change |
| GameRichPresenceJoinRequested_t | ISteamFriends | Join request |
| AvatarImageLoaded_t | ISteamFriends | Avatar ready |
| DownloadItemResult_t | ISteamUGC | Workshop download done |
| ItemInstalled_t | ISteamUGC | Workshop item installed |
| CreateItemResult_t | ISteamUGC | Workshop item created |

---

## Appendix B: Error Code Reference

### EResult Values (Common)

| Code | Name | Description |
|------|------|-------------|
| 1 | k_EResultOK | Success |
| 2 | k_EResultFail | Generic failure |
| 3 | k_EResultNoConnection | No network |
| 5 | k_EResultInvalidPassword | Bad credentials |
| 6 | k_EResultLoggedInElsewhere | Concurrent session |
| 14 | k_EResultInvalidParam | Bad parameter |
| 15 | k_EResultFileNotFound | Missing file |
| 20 | k_EResultNotLoggedOn | Steam not running |
| 25 | k_EResultTimeout | Request timed out |
| 27 | k_EResultBanned | Account banned |
| 29 | k_EResultAccountNotFound | Invalid account |
| 42 | k_EResultLimitExceeded | Rate limited |
| 50 | k_EResultAccountDisabled | Account disabled |

---

## Appendix C: Test Data Sets

### Sample Steam IDs for Testing

| Description | SteamID64 | Format Examples |
|-------------|-----------|-----------------|
| Valid Public User | 76561198012345678 | STEAM_0:0:26039975 |
| Anonymous GS | 90071996842377216 | [A:1:0:0] |
| Pending | 76561197960265728 | [U:1:0] |
| Invalid | 0 | (empty) |

### Sample Lobby IDs

| Description | LobbyID |
|-------------|---------|
| Valid Public | 109775240012345678 |
| Invalid | 0 |

---

*Document Version: 1.0*
*Last Updated: January 2026*
*Maintainer: [Your Team]*
