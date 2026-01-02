//! Parity test runner with HTML report generation.
//!
//! This binary runs all parity tests and generates a beautiful HTML report
//! in the Steamworks documentation style.

use std::path::PathBuf;
use std::time::Instant;

use engine_shared::steam_id::SteamId;
use engine_shared::test_report::{ReportBuilder, TestPriority, TestResult};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let output_dir = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("test-reports"));

    println!("🔧 Source Engine Parity Test Runner");
    println!("====================================\n");

    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let mut builder = ReportBuilder::new("Source Engine Parity Tests")
        .subtitle("Comprehensive validation suite for Source Engine 2014 compatibility")
        .git_info(
            std::env::var("GIT_COMMIT")
                .ok()
                .as_deref()
                .or(option_env!("GIT_COMMIT"))
                .or(Some("development")),
            std::env::var("GIT_BRANCH")
                .ok()
                .as_deref()
                .or(option_env!("GIT_BRANCH"))
                .or(Some("main")),
        );

    if let Ok(build) = std::env::var("BUILD_NUMBER") {
        builder = builder.build_number(&build);
    }

    // Run all test categories
    println!("📋 Running Steam ID tests...");
    builder = run_steam_id_tests(builder);

    println!("🔐 Running Authentication tests...");
    builder = run_auth_tests(builder);

    println!("🏠 Running Lobby tests...");
    builder = run_lobby_tests(builder);

    println!("💬 Running Chat tests...");
    builder = run_chat_tests(builder);

    println!("📡 Running GSI tests...");
    builder = run_gsi_tests(builder);

    println!("🌐 Running Network Protocol tests...");
    builder = run_network_tests(builder);

    let report = builder.build();
    let stats = report.overall_stats();

    // Print summary
    println!("\n====================================");
    println!("📊 Test Results Summary");
    println!("====================================");
    println!("Total:   {}", stats.total);
    println!("Passed:  {} ✓", stats.passed);
    println!("Failed:  {} ✗", stats.failed);
    println!("Skipped: {} ○", stats.skipped);
    println!("Pass Rate: {:.1}%", stats.pass_rate());
    println!("Duration: {:.2}s", stats.total_duration.as_secs_f64());

    // Save reports
    let html_path = output_dir.join("parity-tests.html");
    let json_path = output_dir.join("parity-tests.json");

    report
        .save_html(&html_path)
        .expect("Failed to save HTML report");
    report
        .save_json(&json_path)
        .expect("Failed to save JSON report");

    println!("\n📄 Reports saved to:");
    println!("   HTML: {}", html_path.display());
    println!("   JSON: {}", json_path.display());

    // Exit with appropriate code
    if stats.failed > 0 {
        std::process::exit(1);
    }
}

/// Helper to run a test and capture result
fn run_test<F>(id: &str, name: &str, category: &str, description: &str, f: F) -> TestResult
where
    F: FnOnce() -> Result<(), String>,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();

    let mut test = TestResult::new(id, name, category).with_description(description);

    match result {
        Ok(()) => test = test.pass(duration),
        Err(e) => test = test.fail(duration, &e),
    }

    test
}

fn run_steam_id_tests(mut builder: ReportBuilder) -> ReportBuilder {
    const CATEGORY: &str = "Steam ID";
    const DOC_URL: &str = "https://developer.valvesoftware.com/wiki/SteamID";

    // SID-001: SteamID64 Parsing
    builder = builder.add_test(
        run_test(
            "SID-001",
            "SteamID64 Parsing",
            CATEGORY,
            "Parse 64-bit Steam ID correctly",
            || {
                let id = SteamId::from_u64(76561198012345678);
                if !id.is_valid() {
                    return Err("Steam ID should be valid".to_string());
                }
                if id.as_u64() != 76561198012345678 {
                    return Err(format!("Expected 76561198012345678, got {}", id.as_u64()));
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // SID-002: SteamID32 Conversion
    builder = builder.add_test(
        run_test(
            "SID-002",
            "SteamID32 Conversion",
            CATEGORY,
            "Convert between 32-bit and 64-bit formats",
            || {
                let id = SteamId::from_u64(76561198012345678);
                let account_id = id.account_id();
                let reconstructed = SteamId::from_account_id(account_id);
                if reconstructed.account_id() != account_id {
                    return Err("Account ID mismatch after conversion".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // SID-003: SteamID3 Format
    builder = builder.add_test(
        run_test(
            "SID-003",
            "SteamID3 Format",
            CATEGORY,
            "Parse [U:1:XXXXX] format",
            || {
                let id = SteamId::from_account_id(52079950);
                let steam3 = id.to_steam3();
                if !steam3.starts_with("[U:1:") {
                    return Err(format!("Invalid SteamID3 format: {}", steam3));
                }
                let parsed = SteamId::parse_steam3(&steam3).ok_or("Failed to parse SteamID3")?;
                if parsed.account_id() != 52079950 {
                    return Err("SteamID3 roundtrip failed".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::High)
        .with_doc_reference(DOC_URL),
    );

    // SID-004: STEAM_X:Y:Z Format
    builder = builder.add_test(
        run_test(
            "SID-004",
            "STEAM_X:Y:Z Format",
            CATEGORY,
            "Parse legacy STEAM_0:1:XXXXX format",
            || {
                let id = SteamId::from_account_id(52079950);
                let steam2 = id.to_steam2();
                if !steam2.starts_with("STEAM_") {
                    return Err(format!("Invalid SteamID2 format: {}", steam2));
                }
                let parsed = SteamId::parse_steam2(&steam2).ok_or("Failed to parse SteamID2")?;
                if parsed.account_id() != 52079950 {
                    return Err("SteamID2 roundtrip failed".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::High)
        .with_doc_reference(DOC_URL),
    );

    // SID-005: Account Type Detection
    builder = builder.add_test(
        run_test(
            "SID-005",
            "Account Type Detection",
            CATEGORY,
            "Identify Individual/GameServer/Clan account types",
            || {
                use engine_shared::steam_id::AccountType;

                let individual = SteamId::from_account_id(12345);
                if !individual.is_individual() {
                    return Err("Should be individual account".to_string());
                }

                let gs = SteamId::from_parts(
                    12345,
                    1,
                    AccountType::GameServer,
                    engine_shared::steam_id::Universe::Public,
                );
                if !gs.is_game_server() {
                    return Err("Should be game server".to_string());
                }

                Ok(())
            },
        )
        .with_priority(TestPriority::High)
        .with_doc_reference(DOC_URL),
    );

    // SID-007: Invalid SteamID Rejection
    builder = builder.add_test(
        run_test(
            "SID-007",
            "Invalid SteamID Rejection",
            CATEGORY,
            "Reject malformed Steam IDs",
            || {
                if SteamId::NIL.is_valid() {
                    return Err("NIL should be invalid".to_string());
                }
                if SteamId::from_u64(0).is_valid() {
                    return Err("Zero should be invalid".to_string());
                }
                if "not_a_steam_id".parse::<SteamId>().is_ok() {
                    return Err("Should reject invalid format".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    builder
}

fn run_auth_tests(mut builder: ReportBuilder) -> ReportBuilder {
    const CATEGORY: &str = "Authentication";
    const DOC_URL: &str = "https://partner.steamgames.com/doc/features/auth";

    use engine_shared::auth::{
        AuthSession, AuthSessionResponse, AuthSessionState, MockAuthProvider,
    };

    // AUTH-001: Valid Steam Login
    builder = builder.add_test(
        run_test(
            "AUTH-001",
            "Valid Steam Login",
            CATEGORY,
            "Client authenticates with valid Steam credentials via ISteamUser",
            || {
                let mut provider = MockAuthProvider::new(730);
                let steam_id = SteamId::from_account_id(12345);
                let ticket = provider.get_auth_ticket(steam_id);

                if !ticket.handle.is_valid() {
                    return Err("Ticket handle should be valid".to_string());
                }
                if !ticket.is_valid_size() {
                    return Err("Ticket size should be valid".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // AUTH-002: Invalid Credentials Rejection
    builder = builder.add_test(
        run_test(
            "AUTH-002",
            "Invalid Credentials Rejection",
            CATEGORY,
            "Server rejects connections without valid Steam auth",
            || {
                let mut provider = MockAuthProvider::new(730);
                let real_owner = SteamId::from_account_id(12345);
                let fake_owner = SteamId::from_account_id(99999);
                let ticket = provider.get_auth_ticket(real_owner);

                let response = provider.validate_ticket(&ticket, fake_owner);
                if response != AuthSessionResponse::AuthTicketInvalid {
                    return Err(format!("Should reject fake owner, got {:?}", response));
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // AUTH-005: Auth Ticket Validation
    builder = builder.add_test(
        run_test(
            "AUTH-005",
            "Auth Ticket Validation",
            CATEGORY,
            "Server validates ticket via BeginAuthSession",
            || {
                let mut provider = MockAuthProvider::new(730);
                let steam_id = SteamId::from_account_id(12345);
                let ticket = provider.get_auth_ticket(steam_id);

                let response = provider.validate_ticket(&ticket, steam_id);
                if response != AuthSessionResponse::Ok {
                    return Err(format!("Should accept valid ticket, got {:?}", response));
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // AUTH-009: Auth Callback Processing
    builder = builder.add_test(
        run_test(
            "AUTH-009",
            "Auth Callback Processing",
            CATEGORY,
            "ValidateAuthTicketResponse_t callback processed correctly",
            || {
                let steam_id = SteamId::from_account_id(12345);
                let mut session = AuthSession::new(steam_id);

                if session.state != AuthSessionState::None {
                    return Err("Initial state should be None".to_string());
                }

                session.begin_validation();
                if session.state != AuthSessionState::Pending {
                    return Err("State should be Pending after begin".to_string());
                }

                session.on_validation_response(AuthSessionResponse::Ok);
                if session.state != AuthSessionState::Validated {
                    return Err("State should be Validated after Ok response".to_string());
                }

                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    builder
}

fn run_lobby_tests(mut builder: ReportBuilder) -> ReportBuilder {
    const CATEGORY: &str = "Lobby System";
    const DOC_URL: &str = "https://partner.steamgames.com/doc/features/multiplayer/matchmaking";

    use engine_shared::lobby::{LobbyError, LobbyManager, LobbySearchFilter, LobbyType};

    // LOB-001: Create Public Lobby
    builder = builder.add_test(
        run_test(
            "LOB-001",
            "Create Public Lobby",
            CATEGORY,
            "CreateLobby with k_ELobbyTypePublic",
            || {
                let mut manager = LobbyManager::new();
                let owner = SteamId::from_account_id(12345);
                let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);

                let lobby = manager.get_lobby(lobby_id).ok_or("Lobby not found")?;
                if lobby.lobby_type != LobbyType::Public {
                    return Err("Should be public lobby".to_string());
                }
                if lobby.max_members != 8 {
                    return Err("Max members should be 8".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // LOB-005: Join Lobby by ID
    builder = builder.add_test(
        run_test(
            "LOB-005",
            "Join Lobby by ID",
            CATEGORY,
            "JoinLobby with valid CSteamID",
            || {
                let mut manager = LobbyManager::new();
                let owner = SteamId::from_account_id(1);
                let joiner = SteamId::from_account_id(2);

                let lobby_id = manager.create_lobby(owner, LobbyType::Public, 8);
                let lobby = manager
                    .get_lobby_mut(lobby_id)
                    .ok_or("Lobby not found".to_string())?;
                lobby.add_member(joiner).map_err(|e| format!("{:?}", e))?;

                if !lobby.is_member(joiner) {
                    return Err("Joiner should be member".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // LOB-007: Lobby Member Limit
    builder = builder.add_test(
        run_test(
            "LOB-007",
            "Lobby Member Limit",
            CATEGORY,
            "SetLobbyMemberLimit enforcement",
            || {
                let mut manager = LobbyManager::new();
                let owner = SteamId::from_account_id(1);

                let lobby_id = manager.create_lobby(owner, LobbyType::Public, 2);
                let lobby = manager
                    .get_lobby_mut(lobby_id)
                    .ok_or("Lobby not found".to_string())?;

                lobby
                    .add_member(SteamId::from_account_id(2))
                    .map_err(|e| format!("{:?}", e))?;

                // Third should fail
                match lobby.add_member(SteamId::from_account_id(3)) {
                    Err(LobbyError::LobbyFull) => Ok(()),
                    _ => Err("Should reject when full".to_string()),
                }
            },
        )
        .with_priority(TestPriority::High)
        .with_doc_reference(DOC_URL),
    );

    // LOB-010: Lobby Search
    builder = builder.add_test(
        run_test(
            "LOB-010",
            "Lobby Search",
            CATEGORY,
            "RequestLobbyList with filters",
            || {
                let mut manager = LobbyManager::new();

                manager.create_lobby(SteamId::from_account_id(1), LobbyType::Public, 8);
                manager.create_lobby(SteamId::from_account_id(2), LobbyType::Private, 4);
                manager.create_lobby(SteamId::from_account_id(3), LobbyType::Public, 8);

                let filter = LobbySearchFilter::new();
                let results = manager.search(&filter);

                // Should only find public lobbies
                if results.len() != 2 {
                    return Err(format!("Expected 2 public lobbies, got {}", results.len()));
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    builder
}

fn run_chat_tests(mut builder: ReportBuilder) -> ReportBuilder {
    const CATEGORY: &str = "Chat System";
    const DOC_URL: &str = "https://developer.valvesoftware.com/wiki/Chat";

    use engine_shared::chat::{ChatChannel, ChatManager, ChatResult, MAX_MESSAGE_LENGTH};

    // CHAT-001: Global Chat
    builder = builder.add_test(
        run_test(
            "CHAT-001",
            "Global Chat",
            CATEGORY,
            "All-player broadcast",
            || {
                let mut manager = ChatManager::new(100);
                let sender = SteamId::from_account_id(1);
                let receiver = SteamId::from_account_id(2);

                manager.add_player(sender);
                manager.add_player(receiver);

                let recipients = manager
                    .send_message(sender, "Player1", ChatChannel::Global, "Hello!")
                    .map_err(|e| format!("{:?}", e))?;

                if !recipients.contains(&receiver) {
                    return Err("Receiver should get global message".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // CHAT-006: Message Length Limit
    builder = builder.add_test(
        run_test(
            "CHAT-006",
            "Message Length Limit",
            CATEGORY,
            "Truncation at max length",
            || {
                let mut manager = ChatManager::new(100);
                let sender = SteamId::from_account_id(1);
                manager.add_player(sender);

                let long_message = "x".repeat(MAX_MESSAGE_LENGTH + 1);
                match manager.send_message(sender, "Player1", ChatChannel::Global, &long_message) {
                    Err(ChatResult::MessageTooLong) => Ok(()),
                    _ => Err("Should reject long message".to_string()),
                }
            },
        )
        .with_priority(TestPriority::High)
        .with_doc_reference(DOC_URL),
    );

    // CHAT-007: Rate Limiting
    builder = builder.add_test(
        run_test(
            "CHAT-007",
            "Rate Limiting",
            CATEGORY,
            "Spam prevention",
            || {
                use engine_shared::chat::RateLimiter;
                use std::time::Duration;

                let mut limiter = RateLimiter::new(3, Duration::from_millis(100));

                assert!(limiter.record_message());
                assert!(limiter.record_message());
                assert!(limiter.record_message());

                if limiter.record_message() {
                    return Err("4th message should be rate limited".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::High)
        .with_doc_reference(DOC_URL),
    );

    // CHAT-008: Mute Player
    builder = builder.add_test(
        run_test(
            "CHAT-008",
            "Mute Player",
            CATEGORY,
            "Hide messages from muted player",
            || {
                let mut manager = ChatManager::new(100);
                let sender = SteamId::from_account_id(1);
                let receiver = SteamId::from_account_id(2);

                manager.add_player(sender);
                manager.add_player(receiver);

                manager
                    .get_player_mut(receiver)
                    .ok_or("Player not found".to_string())?
                    .mute_player(sender);

                let recipients = manager
                    .send_message(sender, "Player1", ChatChannel::Global, "Hello!")
                    .map_err(|e| format!("{:?}", e))?;

                if recipients.contains(&receiver) {
                    return Err("Muted sender's message should not reach receiver".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::High)
        .with_doc_reference(DOC_URL),
    );

    builder
}

fn run_gsi_tests(mut builder: ReportBuilder) -> ReportBuilder {
    const CATEGORY: &str = "Game State Integration";
    const DOC_URL: &str = "https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Game_State_Integration";

    use engine_shared::gsi::{
        GameMode, GsiError, GsiMap, GsiPayload, GsiPlayer, GsiProvider, GsiReceiver, PlayerTeam,
    };

    // GSI-001: Provider Block
    builder = builder.add_test(
        run_test(
            "GSI-001",
            "Provider Block",
            CATEGORY,
            "Game identity information",
            || {
                let provider = GsiProvider::new(
                    "Counter-Strike 2",
                    730,
                    14000,
                    SteamId::from_account_id(12345),
                );

                if provider.name != "Counter-Strike 2" {
                    return Err("Wrong game name".to_string());
                }
                if provider.appid != 730 {
                    return Err("Wrong app ID".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // GSI-009: Auth Token
    builder = builder.add_test(
        run_test(
            "GSI-009",
            "Auth Token",
            CATEGORY,
            "Token-based validation",
            || {
                let mut receiver = GsiReceiver::new(Some("secret_token".to_string()));

                let provider = GsiProvider::new("Test", 730, 1, SteamId::from_account_id(1));
                let payload = GsiPayload::new(provider).with_auth("secret_token");
                let json = payload.to_json().map_err(|e| e.to_string())?;

                receiver.process(&json).map_err(|e| format!("{:?}", e))?;
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // GSI-009b: Invalid Token Rejection
    builder = builder.add_test(
        run_test(
            "GSI-009b",
            "Invalid Token Rejection",
            CATEGORY,
            "Reject payloads with wrong auth token",
            || {
                let mut receiver = GsiReceiver::new(Some("correct_token".to_string()));

                let provider = GsiProvider::new("Test", 730, 1, SteamId::from_account_id(1));
                let payload = GsiPayload::new(provider).with_auth("wrong_token");
                let json = payload.to_json().map_err(|e| e.to_string())?;

                match receiver.process(&json) {
                    Err(GsiError::InvalidToken) => Ok(()),
                    _ => Err("Should reject invalid token".to_string()),
                }
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // GSI-010: HTTP POST Delivery
    builder = builder.add_test(
        run_test(
            "GSI-010",
            "JSON Payload Roundtrip",
            CATEGORY,
            "Payload delivery and parsing",
            || {
                let provider = GsiProvider::new("CS2", 730, 14000, SteamId::from_account_id(123));
                let mut payload = GsiPayload::new(provider);
                payload.map = Some(GsiMap::new("de_dust2", GameMode::Competitive));
                payload.player = Some(GsiPlayer::new(
                    SteamId::from_account_id(123),
                    "Player1",
                    PlayerTeam::CT,
                ));

                let json = payload.to_json_pretty().map_err(|e| e.to_string())?;
                let parsed = GsiPayload::from_json(&json).map_err(|e| e.to_string())?;

                if parsed.map.as_ref().map(|m| m.name.as_str()) != Some("de_dust2") {
                    return Err("Map name mismatch".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    builder
}

fn run_network_tests(mut builder: ReportBuilder) -> ReportBuilder {
    const CATEGORY: &str = "Network Protocol";
    const DOC_URL: &str = "https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking";

    use engine_shared::net::{decode_from_bytes, encode_to_bytes, NetMsg, PROTOCOL_VERSION};

    // NET-001: Connection Handshake
    builder = builder.add_test(
        run_test(
            "NET-001",
            "Protocol Message Encoding",
            CATEGORY,
            "NetMsg serialization roundtrip",
            || {
                let hello = NetMsg::Hello {
                    protocol: PROTOCOL_VERSION,
                };
                let bytes = encode_to_bytes(&hello).map_err(|e| e.to_string())?;
                let decoded: NetMsg = decode_from_bytes(&bytes).map_err(|e| e.to_string())?;

                if decoded != hello {
                    return Err("Hello message roundtrip failed".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // NET-002: Welcome Message
    builder = builder.add_test(
        run_test(
            "NET-002",
            "Welcome Message",
            CATEGORY,
            "Client ID assignment",
            || {
                use engine_shared::net::ClientId;

                let welcome = NetMsg::Welcome {
                    client_id: ClientId(42),
                };
                let bytes = encode_to_bytes(&welcome).map_err(|e| e.to_string())?;
                let decoded: NetMsg = decode_from_bytes(&bytes).map_err(|e| e.to_string())?;

                if decoded != welcome {
                    return Err("Welcome message roundtrip failed".to_string());
                }
                Ok(())
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // NET-003: Snapshot Message
    builder = builder.add_test(
        run_test(
            "NET-003",
            "Snapshot Message",
            CATEGORY,
            "Game state snapshot encoding",
            || {
                use engine_shared::net::Snapshot;

                let snapshot = NetMsg::Snapshot(Snapshot {
                    tick: 1000,
                    entities: vec![],
                });
                let bytes = encode_to_bytes(&snapshot).map_err(|e| e.to_string())?;
                let decoded: NetMsg = decode_from_bytes(&bytes).map_err(|e| e.to_string())?;

                match decoded {
                    NetMsg::Snapshot(s) if s.tick == 1000 => Ok(()),
                    _ => Err("Snapshot message roundtrip failed".to_string()),
                }
            },
        )
        .with_priority(TestPriority::Critical)
        .with_doc_reference(DOC_URL),
    );

    // NET-004: Map Info Message
    builder = builder.add_test(
        run_test(
            "NET-004",
            "Map Info Message",
            CATEGORY,
            "Map loading packet",
            || {
                use engine_shared::net::MapInfo;

                let map_info = NetMsg::MapInfo(MapInfo {
                    name: "de_dust2".to_string(),
                    crc: 0xDEADBEEF,
                    size: 1024,
                });
                let bytes = encode_to_bytes(&map_info).map_err(|e| e.to_string())?;
                let decoded: NetMsg = decode_from_bytes(&bytes).map_err(|e| e.to_string())?;

                match decoded {
                    NetMsg::MapInfo(info) if info.name == "de_dust2" && info.crc == 0xDEADBEEF => {
                        Ok(())
                    }
                    _ => Err("MapInfo message roundtrip failed".to_string()),
                }
            },
        )
        .with_priority(TestPriority::High)
        .with_doc_reference(DOC_URL),
    );

    builder
}
