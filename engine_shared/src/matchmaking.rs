//! Steam Matchmaking and Server Browser implementation.
//!
//! # Valve Documentation Reference
//! - [ISteamMatchmakingServers](https://partner.steamgames.com/doc/api/ISteamMatchmakingServers)
//! - [Server Browser](https://partner.steamgames.com/doc/features/multiplayer/matchmaking)
//! - [A2S Protocol](https://developer.valvesoftware.com/wiki/Server_queries)
//!
//! # Features
//! - Server browser queries (internet, LAN, favorites, history)
//! - Server filtering with key-value pairs
//! - A2S protocol queries (INFO, PLAYER, RULES)
//! - Ping measurement

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4};

use serde::{Deserialize, Serialize};

/// Server type for query requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerType {
    /// Internet servers.
    Internet,
    /// LAN servers.
    Lan,
    /// Friends' servers.
    Friends,
    /// Favorite servers.
    Favorites,
    /// History servers.
    History,
    /// Spectator servers.
    Spectator,
}

/// Filter key-value pair for matchmaking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchMakingKeyValuePair {
    /// Filter key.
    pub key: String,
    /// Filter value.
    pub value: String,
}

impl MatchMakingKeyValuePair {
    /// Create a new filter pair.
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

/// Server network address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServerNetAdr {
    /// IP address.
    pub ip: u32,
    /// Connection port.
    pub connection_port: u16,
    /// Query port.
    pub query_port: u16,
}

impl ServerNetAdr {
    /// Create from components.
    pub fn new(ip: u32, connection_port: u16, query_port: u16) -> Self {
        Self {
            ip,
            connection_port,
            query_port,
        }
    }

    /// Create from socket address.
    pub fn from_socket_addr(addr: SocketAddrV4, query_port: u16) -> Self {
        let ip = u32::from(*addr.ip());
        Self {
            ip,
            connection_port: addr.port(),
            query_port,
        }
    }

    /// Get IP as Ipv4Addr.
    pub fn ip_addr(&self) -> Ipv4Addr {
        Ipv4Addr::from(self.ip)
    }

    /// Get connection address string.
    pub fn connection_address(&self) -> String {
        format!("{}:{}", self.ip_addr(), self.connection_port)
    }

    /// Get query address string.
    pub fn query_address(&self) -> String {
        format!("{}:{}", self.ip_addr(), self.query_port)
    }
}

/// Game server info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameServerInfo {
    /// Server address.
    pub addr: String,
    /// Server name.
    pub server_name: String,
    /// Map name.
    pub map: String,
    /// Game directory.
    pub game_dir: String,
    /// Game description.
    pub game_description: String,
    /// App ID.
    pub app_id: u32,
    /// Current players.
    pub players: u8,
    /// Max players.
    pub max_players: u8,
    /// Number of bots.
    pub bots: u8,
    /// Server type (d=dedicated, l=listen, p=proxy).
    pub server_type: char,
    /// OS (l=linux, w=windows, m=mac).
    pub os: char,
    /// Password protected.
    pub password: bool,
    /// VAC secured.
    pub secure: bool,
    /// Server version.
    pub version: String,
    /// Ping in milliseconds.
    pub ping: u32,
    /// Steam ID of the server.
    pub steam_id: u64,
    /// Tags (for filtering).
    pub tags: String,
}

impl Default for GameServerInfo {
    fn default() -> Self {
        Self {
            addr: String::new(),
            server_name: String::new(),
            map: String::new(),
            game_dir: String::new(),
            game_description: String::new(),
            app_id: 0,
            players: 0,
            max_players: 0,
            bots: 0,
            server_type: 'd',
            os: 'l',
            password: false,
            secure: true,
            version: String::new(),
            ping: 0,
            steam_id: 0,
            tags: String::new(),
        }
    }
}

/// Player info from A2S_PLAYER query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    /// Player index.
    pub index: u8,
    /// Player name.
    pub name: String,
    /// Player score.
    pub score: i32,
    /// Time connected (seconds).
    pub duration: f32,
}

/// Server rule (cvar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRule {
    /// Cvar name.
    pub name: String,
    /// Cvar value.
    pub value: String,
}

/// A2S query type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A2SQueryType {
    /// A2S_INFO (0x54).
    Info,
    /// A2S_PLAYER (0x55).
    Player,
    /// A2S_RULES (0x56).
    Rules,
    /// Challenge request.
    Challenge,
}

impl A2SQueryType {
    /// Get the query byte.
    pub fn header_byte(&self) -> u8 {
        match self {
            A2SQueryType::Info => 0x54,
            A2SQueryType::Player => 0x55,
            A2SQueryType::Rules => 0x56,
            A2SQueryType::Challenge => 0x57,
        }
    }
}

/// A2S response type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A2SResponseType {
    /// Info response (0x49).
    Info,
    /// Player response (0x44).
    Player,
    /// Rules response (0x45).
    Rules,
    /// Challenge response (0x41).
    Challenge,
}

impl A2SResponseType {
    /// Get from header byte.
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x49 => Some(A2SResponseType::Info),
            0x44 => Some(A2SResponseType::Player),
            0x45 => Some(A2SResponseType::Rules),
            0x41 => Some(A2SResponseType::Challenge),
            _ => None,
        }
    }
}

/// Query result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryResult {
    /// Success.
    Ok,
    /// Timeout.
    Timeout,
    /// Invalid response.
    InvalidResponse,
    /// Server not responding.
    NotResponding,
    /// Rate limited.
    RateLimited,
}

/// Mock server browser for testing.
///
/// In production, this would interface with Steamworks SDK.
pub struct ServerBrowser {
    /// App ID to filter.
    app_id: u32,
    /// Known servers.
    servers: HashMap<ServerNetAdr, GameServerInfo>,
    /// Favorite servers.
    favorites: Vec<ServerNetAdr>,
    /// History servers.
    history: Vec<ServerNetAdr>,
    /// Friends' servers.
    friends_servers: Vec<ServerNetAdr>,
    /// Current filters.
    filters: Vec<MatchMakingKeyValuePair>,
    /// Challenge numbers for anti-spoof.
    challenges: HashMap<ServerNetAdr, u32>,
    /// Next challenge number.
    next_challenge: u32,
}

impl ServerBrowser {
    /// Create a new server browser.
    pub fn new(app_id: u32) -> Self {
        Self {
            app_id,
            servers: HashMap::new(),
            favorites: Vec::new(),
            history: Vec::new(),
            friends_servers: Vec::new(),
            filters: Vec::new(),
            challenges: HashMap::new(),
            next_challenge: 1000,
        }
    }

    /// Add a filter.
    pub fn add_filter(&mut self, key: &str, value: &str) {
        self.filters.push(MatchMakingKeyValuePair::new(key, value));
    }

    /// Clear filters.
    pub fn clear_filters(&mut self) {
        self.filters.clear();
    }

    /// Request server list.
    pub fn request_server_list(&self, server_type: ServerType) -> Vec<&GameServerInfo> {
        let addrs: Vec<&ServerNetAdr> = match server_type {
            ServerType::Internet => self.servers.keys().collect(),
            ServerType::Lan => self.servers.keys().filter(|a| Self::is_lan_addr(*a)).collect(),
            ServerType::Friends => self.friends_servers.iter().collect(),
            ServerType::Favorites => self.favorites.iter().collect(),
            ServerType::History => self.history.iter().collect(),
            ServerType::Spectator => Vec::new(),
        };

        addrs
            .into_iter()
            .filter_map(|addr| self.servers.get(addr))
            .filter(|server| self.matches_filters(server))
            .collect()
    }

    /// Check if address is LAN.
    fn is_lan_addr(addr: &ServerNetAdr) -> bool {
        let ip = Ipv4Addr::from(addr.ip);
        ip.is_private() || ip.is_loopback()
    }

    /// Check if server matches current filters.
    fn matches_filters(&self, server: &GameServerInfo) -> bool {
        for filter in &self.filters {
            match filter.key.as_str() {
                "appid" => {
                    if server.app_id.to_string() != filter.value {
                        return false;
                    }
                }
                "map" => {
                    if !server.map.contains(&filter.value) {
                        return false;
                    }
                }
                "gamedir" => {
                    if server.game_dir != filter.value {
                        return false;
                    }
                }
                "secure" => {
                    let want_secure = filter.value == "1";
                    if server.secure != want_secure {
                        return false;
                    }
                }
                "notfull" => {
                    if filter.value == "1" && server.players >= server.max_players {
                        return false;
                    }
                }
                "hasplayers" => {
                    if filter.value == "1" && server.players == 0 {
                        return false;
                    }
                }
                "noplayers" => {
                    if filter.value == "1" && server.players > 0 {
                        return false;
                    }
                }
                "gametype" => {
                    if !server.tags.contains(&filter.value) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }

    /// Get server count for a type.
    pub fn get_server_count(&self, server_type: ServerType) -> usize {
        self.request_server_list(server_type).len()
    }

    /// Add a server (for testing).
    pub fn add_server(&mut self, addr: ServerNetAdr, info: GameServerInfo) {
        self.servers.insert(addr, info);
    }

    /// Add to favorites.
    pub fn add_to_favorites(&mut self, addr: ServerNetAdr) {
        if !self.favorites.contains(&addr) {
            self.favorites.push(addr);
        }
    }

    /// Remove from favorites.
    pub fn remove_from_favorites(&mut self, addr: ServerNetAdr) {
        self.favorites.retain(|a| *a != addr);
    }

    /// Add to history.
    pub fn add_to_history(&mut self, addr: ServerNetAdr) {
        // Remove if already present.
        self.history.retain(|a| *a != addr);
        // Add to front.
        self.history.insert(0, addr);
        // Limit history size.
        if self.history.len() > 100 {
            self.history.truncate(100);
        }
    }

    /// Add friend's server.
    pub fn add_friend_server(&mut self, addr: ServerNetAdr) {
        if !self.friends_servers.contains(&addr) {
            self.friends_servers.push(addr);
        }
    }

    /// Get challenge for a server.
    pub fn get_challenge(&mut self, addr: ServerNetAdr) -> u32 {
        if let Some(&challenge) = self.challenges.get(&addr) {
            challenge
        } else {
            let challenge = self.next_challenge;
            self.next_challenge += 1;
            self.challenges.insert(addr, challenge);
            challenge
        }
    }

    /// Ping a server (simulated).
    pub fn ping_server(&self, addr: &ServerNetAdr) -> Option<u32> {
        self.servers.get(addr).map(|s| s.ping)
    }
}

/// A2S query builder and parser.
pub struct A2SQuery;

impl A2SQuery {
    /// Build A2S_INFO query packet.
    pub fn build_info_query() -> Vec<u8> {
        let mut packet = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x54]; // Header + 'T'
        packet.extend_from_slice(b"Source Engine Query\0");
        packet
    }

    /// Build A2S_PLAYER query packet with challenge.
    pub fn build_player_query(challenge: u32) -> Vec<u8> {
        let mut packet = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x55]; // Header + 'U'
        packet.extend_from_slice(&challenge.to_le_bytes());
        packet
    }

    /// Build A2S_RULES query packet with challenge.
    pub fn build_rules_query(challenge: u32) -> Vec<u8> {
        let mut packet = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x56]; // Header + 'V'
        packet.extend_from_slice(&challenge.to_le_bytes());
        packet
    }

    /// Build challenge request packet.
    pub fn build_challenge_request(query_type: A2SQueryType) -> Vec<u8> {
        let mut packet = vec![0xFF, 0xFF, 0xFF, 0xFF, query_type.header_byte()];
        packet.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // -1 challenge
        packet
    }

    /// Parse response header.
    pub fn parse_response_type(packet: &[u8]) -> Option<A2SResponseType> {
        if packet.len() < 5 {
            return None;
        }
        if &packet[0..4] != &[0xFF, 0xFF, 0xFF, 0xFF] {
            return None;
        }
        A2SResponseType::from_byte(packet[4])
    }

    /// Check if response is multi-packet.
    pub fn is_multi_packet(packet: &[u8]) -> bool {
        if packet.len() < 4 {
            return false;
        }
        &packet[0..4] == &[0xFE, 0xFF, 0xFF, 0xFF]
    }

    /// Maximum single packet size.
    pub const MAX_PACKET_SIZE: usize = 1400;
}

/// Mock server rules query.
pub struct ServerRules {
    /// Rules/cvars.
    rules: Vec<ServerRule>,
}

impl ServerRules {
    /// Create new rules query result.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule.
    pub fn add_rule(&mut self, name: &str, value: &str) {
        self.rules.push(ServerRule {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    /// Get rule count.
    pub fn count(&self) -> usize {
        self.rules.len()
    }

    /// Get rule by index.
    pub fn get(&self, index: usize) -> Option<&ServerRule> {
        self.rules.get(index)
    }

    /// Find rule by name.
    pub fn find(&self, name: &str) -> Option<&ServerRule> {
        self.rules.iter().find(|r| r.name == name)
    }
}

impl Default for ServerRules {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_server(name: &str, map: &str, players: u8, max_players: u8) -> GameServerInfo {
        GameServerInfo {
            addr: "192.168.1.1:27015".to_string(),
            server_name: name.to_string(),
            map: map.to_string(),
            game_dir: "csgo".to_string(),
            game_description: "Counter-Strike: Global Offensive".to_string(),
            app_id: 730,
            players,
            max_players,
            bots: 0,
            server_type: 'd',
            os: 'l',
            password: false,
            secure: true,
            version: "1.0.0".to_string(),
            ping: 25,
            steam_id: 12345,
            tags: "competitive".to_string(),
        }
    }

    // =============================================================================
    // MM-001: Request Internet Servers
    // Reference: https://partner.steamgames.com/doc/api/ISteamMatchmakingServers
    // =============================================================================

    #[test]
    fn mm_001_request_internet_servers() {
        let mut browser = ServerBrowser::new(730);

        let addr = ServerNetAdr::new(0xC0A80101, 27015, 27015); // 192.168.1.1
        browser.add_server(addr, create_test_server("Test Server", "de_dust2", 10, 24));

        let servers = browser.request_server_list(ServerType::Internet);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].server_name, "Test Server");
    }

    // =============================================================================
    // MM-002: Request LAN Servers
    // =============================================================================

    #[test]
    fn mm_002_request_lan_servers() {
        let mut browser = ServerBrowser::new(730);

        // LAN address (192.168.x.x).
        let lan_addr = ServerNetAdr::new(0xC0A80101, 27015, 27015);
        browser.add_server(lan_addr, create_test_server("LAN Server", "de_dust2", 5, 10));

        // Public address.
        let pub_addr = ServerNetAdr::new(0x08080808, 27015, 27015);
        browser.add_server(pub_addr, create_test_server("Public Server", "de_dust2", 5, 10));

        let servers = browser.request_server_list(ServerType::Lan);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].server_name, "LAN Server");
    }

    // =============================================================================
    // MM-003: Request Friends Servers
    // =============================================================================

    #[test]
    fn mm_003_request_friends_servers() {
        let mut browser = ServerBrowser::new(730);

        let addr = ServerNetAdr::new(0xC0A80101, 27015, 27015);
        browser.add_server(addr, create_test_server("Friend's Server", "de_dust2", 8, 16));
        browser.add_friend_server(addr);

        let servers = browser.request_server_list(ServerType::Friends);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].server_name, "Friend's Server");
    }

    // =============================================================================
    // MM-004: Request Favorites
    // =============================================================================

    #[test]
    fn mm_004_request_favorites() {
        let mut browser = ServerBrowser::new(730);

        let addr = ServerNetAdr::new(0xC0A80101, 27015, 27015);
        browser.add_server(addr, create_test_server("Favorite Server", "de_dust2", 10, 20));
        browser.add_to_favorites(addr);

        let servers = browser.request_server_list(ServerType::Favorites);
        assert_eq!(servers.len(), 1);

        browser.remove_from_favorites(addr);
        let servers = browser.request_server_list(ServerType::Favorites);
        assert_eq!(servers.len(), 0);
    }

    // =============================================================================
    // MM-005: Request History
    // =============================================================================

    #[test]
    fn mm_005_request_history() {
        let mut browser = ServerBrowser::new(730);

        let addr1 = ServerNetAdr::new(0xC0A80101, 27015, 27015);
        let addr2 = ServerNetAdr::new(0xC0A80102, 27015, 27015);

        browser.add_server(addr1, create_test_server("Server 1", "de_dust2", 5, 10));
        browser.add_server(addr2, create_test_server("Server 2", "de_inferno", 8, 16));

        browser.add_to_history(addr1);
        browser.add_to_history(addr2);

        let servers = browser.request_server_list(ServerType::History);
        assert_eq!(servers.len(), 2);
    }

    // =============================================================================
    // MM-007: Filter Application
    // =============================================================================

    #[test]
    fn mm_007_filter_application() {
        let mut browser = ServerBrowser::new(730);

        let addr1 = ServerNetAdr::new(0xC0A80101, 27015, 27015);
        let addr2 = ServerNetAdr::new(0xC0A80102, 27015, 27015);

        let mut server1 = create_test_server("Dust2 Server", "de_dust2", 10, 24);
        server1.app_id = 730;
        browser.add_server(addr1, server1);

        let mut server2 = create_test_server("Inferno Server", "de_inferno", 12, 24);
        server2.app_id = 730;
        browser.add_server(addr2, server2);

        // Filter by map.
        browser.add_filter("map", "dust2");
        let servers = browser.request_server_list(ServerType::Internet);
        assert_eq!(servers.len(), 1);
        assert!(servers[0].map.contains("dust2"));

        browser.clear_filters();

        // Filter by not full.
        browser.add_filter("notfull", "1");
        let servers = browser.request_server_list(ServerType::Internet);
        assert_eq!(servers.len(), 2);
    }

    #[test]
    fn mm_007_filter_secure() {
        let mut browser = ServerBrowser::new(730);

        let addr1 = ServerNetAdr::new(0xC0A80101, 27015, 27015);
        let addr2 = ServerNetAdr::new(0xC0A80102, 27015, 27015);

        let mut secure_server = create_test_server("Secure", "de_dust2", 10, 24);
        secure_server.secure = true;
        browser.add_server(addr1, secure_server);

        let mut insecure_server = create_test_server("Insecure", "de_dust2", 10, 24);
        insecure_server.secure = false;
        browser.add_server(addr2, insecure_server);

        browser.add_filter("secure", "1");
        let servers = browser.request_server_list(ServerType::Internet);
        assert_eq!(servers.len(), 1);
        assert!(servers[0].secure);
    }

    // =============================================================================
    // MM-008: Server Response Parsing
    // =============================================================================

    #[test]
    fn mm_008_server_net_adr_parsing() {
        let addr = ServerNetAdr::new(0xC0A80101, 27015, 27016);

        assert_eq!(addr.ip_addr(), Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(addr.connection_address(), "192.168.1.1:27015");
        assert_eq!(addr.query_address(), "192.168.1.1:27016");
    }

    #[test]
    fn mm_008_from_socket_addr() {
        let socket = SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 27015);
        let addr = ServerNetAdr::from_socket_addr(socket, 27016);

        assert_eq!(addr.ip_addr(), Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(addr.connection_port, 27015);
        assert_eq!(addr.query_port, 27016);
    }

    // =============================================================================
    // MM-009: Ping Measurement
    // =============================================================================

    #[test]
    fn mm_009_ping_measurement() {
        let mut browser = ServerBrowser::new(730);

        let addr = ServerNetAdr::new(0xC0A80101, 27015, 27015);
        let mut server = create_test_server("Test", "de_dust2", 10, 24);
        server.ping = 42;
        browser.add_server(addr, server);

        let ping = browser.ping_server(&addr);
        assert_eq!(ping, Some(42));
    }

    // =============================================================================
    // MM-010: Server Rules Query
    // =============================================================================

    #[test]
    fn mm_010_server_rules_query() {
        let mut rules = ServerRules::new();

        rules.add_rule("sv_cheats", "0");
        rules.add_rule("sv_maxrate", "0");
        rules.add_rule("mp_friendlyfire", "0");

        assert_eq!(rules.count(), 3);

        let cheats = rules.find("sv_cheats");
        assert!(cheats.is_some());
        assert_eq!(cheats.unwrap().value, "0");
    }

    // =============================================================================
    // A2S Protocol Tests
    // =============================================================================

    #[test]
    fn a2s_info_query_format() {
        let packet = A2SQuery::build_info_query();

        assert!(packet.len() > 5);
        assert_eq!(&packet[0..4], &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(packet[4], 0x54); // 'T'
    }

    #[test]
    fn a2s_player_query_format() {
        let packet = A2SQuery::build_player_query(12345);

        assert_eq!(&packet[0..4], &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(packet[4], 0x55); // 'U'
        assert_eq!(&packet[5..9], &12345u32.to_le_bytes());
    }

    #[test]
    fn a2s_rules_query_format() {
        let packet = A2SQuery::build_rules_query(67890);

        assert_eq!(&packet[0..4], &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(packet[4], 0x56); // 'V'
    }

    #[test]
    fn a2s_challenge_request() {
        let packet = A2SQuery::build_challenge_request(A2SQueryType::Player);

        assert_eq!(&packet[0..4], &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(packet[4], 0x55);
        assert_eq!(&packet[5..9], &[0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn a2s_response_type_parsing() {
        let info_response = [0xFF, 0xFF, 0xFF, 0xFF, 0x49, 0x00];
        assert_eq!(
            A2SQuery::parse_response_type(&info_response),
            Some(A2SResponseType::Info)
        );

        let player_response = [0xFF, 0xFF, 0xFF, 0xFF, 0x44];
        assert_eq!(
            A2SQuery::parse_response_type(&player_response),
            Some(A2SResponseType::Player)
        );

        let challenge_response = [0xFF, 0xFF, 0xFF, 0xFF, 0x41];
        assert_eq!(
            A2SQuery::parse_response_type(&challenge_response),
            Some(A2SResponseType::Challenge)
        );
    }

    #[test]
    fn a2s_multi_packet_detection() {
        let single = [0xFF, 0xFF, 0xFF, 0xFF, 0x49];
        assert!(!A2SQuery::is_multi_packet(&single));

        let multi = [0xFE, 0xFF, 0xFF, 0xFF, 0x49];
        assert!(A2SQuery::is_multi_packet(&multi));
    }

    #[test]
    fn a2s_max_packet_size() {
        assert_eq!(A2SQuery::MAX_PACKET_SIZE, 1400);
    }

    // =============================================================================
    // Challenge Anti-Spoof Tests
    // =============================================================================

    #[test]
    fn challenge_anti_spoof() {
        let mut browser = ServerBrowser::new(730);

        let addr = ServerNetAdr::new(0xC0A80101, 27015, 27015);

        let challenge1 = browser.get_challenge(addr);
        let challenge2 = browser.get_challenge(addr);

        // Same address should get same challenge.
        assert_eq!(challenge1, challenge2);

        let addr2 = ServerNetAdr::new(0xC0A80102, 27015, 27015);
        let challenge3 = browser.get_challenge(addr2);

        // Different address should get different challenge.
        assert_ne!(challenge1, challenge3);
    }

    // =============================================================================
    // Additional Tests
    // =============================================================================

    #[test]
    fn history_limit() {
        let mut browser = ServerBrowser::new(730);

        // Add more than 100 servers to history.
        for i in 0..150 {
            let addr = ServerNetAdr::new(0xC0A80000 + i, 27015, 27015);
            browser.add_server(addr, create_test_server(&format!("Server {}", i), "de_dust2", 5, 10));
            browser.add_to_history(addr);
        }

        // History should be limited to 100.
        assert!(browser.history.len() <= 100);
    }

    #[test]
    fn filter_has_players() {
        let mut browser = ServerBrowser::new(730);

        let addr1 = ServerNetAdr::new(0xC0A80101, 27015, 27015);
        let addr2 = ServerNetAdr::new(0xC0A80102, 27015, 27015);

        browser.add_server(addr1, create_test_server("Empty", "de_dust2", 0, 24));
        browser.add_server(addr2, create_test_server("Active", "de_dust2", 10, 24));

        browser.add_filter("hasplayers", "1");
        let servers = browser.request_server_list(ServerType::Internet);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].server_name, "Active");
    }
}
