//! Client implementation.
//!
//! The client maintains:
//! - A reliable control stream (handshake + map loading + critical messages)
//! - An unreliable datagram socket (snapshots, input, etc.)
//! - Snapshot history for interpolation
//! - Per-tick command generation (prediction stub)
//! - Console for user commands
//! - BSP map loading

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use anyhow::Context;
use engine_shared::{
    bsp::BspMap,
    config::EngineConfig,
    console::{Console, CvarFlags, CvarValue},
    net::{
        ClientId, EntitySpawn, MapInfo, NetMsg, PlayerCommand, ReliableConn, UnreliableConn,
        PROTOCOL_VERSION,
    },
};
use tokio::net::TcpStream;
use tracing::{debug, info, warn};

use crate::{
    input::{build_command, InputState},
    interp::SnapshotBuffer,
};

/// Client connection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientState {
    /// Not connected to any server.
    Disconnected,
    /// Connecting to server (handshake in progress).
    Connecting,
    /// Connected, waiting for map info.
    Connected,
    /// Loading map.
    LoadingMap,
    /// Map loaded, ready to play.
    Ready,
}

/// High-level game client.
pub struct GameClient {
    pub client_id: ClientId,
    pub state: ClientState,
    pub console: Console,

    reliable: ReliableConn,
    pub unreliable: UnreliableConn,
    pub snaps: SnapshotBuffer,
    tick: u32,

    /// Currently loaded map.
    pub current_map: Option<BspMap>,
    /// Expected map info from server.
    pub pending_map: Option<MapInfo>,
    /// Path to maps directory.
    maps_dir: PathBuf,

    /// Spawned entities received from server.
    pub spawned_entities: Vec<EntitySpawn>,

    /// Server messages to display.
    pub server_messages: Vec<String>,
}

impl GameClient {
    /// Connects to a server and performs handshake.
    pub async fn connect(cfg: &EngineConfig) -> anyhow::Result<Self> {
        let server_addr: SocketAddr = cfg.server_addr.parse().context("parse server_addr")?;

        info!(server = %server_addr, "Connecting to server");

        // Bind UDP first so we can tell the server where to send snapshots.
        let bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let unreliable = UnreliableConn::connect(bind, server_addr).await?;
        let client_udp_port = unreliable.local_addr().context("udp local_addr")?.port();

        let stream = TcpStream::connect(server_addr)
            .await
            .context("tcp connect")?;
        let mut reliable = ReliableConn::new(stream);

        reliable
            .send(&NetMsg::Hello {
                protocol: PROTOCOL_VERSION,
            })
            .await?;

        reliable.send(&NetMsg::UdpHello { client_udp_port }).await?;

        let welcome = reliable.recv().await?;
        let client_id = match welcome {
            NetMsg::Welcome { client_id } => client_id,
            other => anyhow::bail!("expected Welcome, got {other:?}"),
        };

        info!(client_id = ?client_id, "Connected to server");

        let mut console = Console::new();
        Self::register_cvars(&mut console);

        let mut client = Self {
            client_id,
            state: ClientState::Connected,
            console,
            reliable,
            unreliable,
            snaps: SnapshotBuffer::new(32),
            tick: 0,
            current_map: None,
            pending_map: None,
            maps_dir: PathBuf::from(&cfg.maps_dir),
            spawned_entities: Vec::new(),
            server_messages: Vec::new(),
        };

        // Check for immediate MapInfo.
        client.poll_reliable().await?;

        Ok(client)
    }

    fn register_cvars(console: &mut Console) {
        console.register_cvar(
            "cl_interp",
            CvarValue::Float(0.1),
            "Interpolation delay",
            CvarFlags::NONE,
        );
        console.register_cvar(
            "cl_predict",
            CvarValue::Bool(true),
            "Enable client prediction",
            CvarFlags::NONE,
        );
        console.register_cvar(
            "name",
            CvarValue::String("Player".to_string()),
            "Player name",
            CvarFlags::NONE,
        );
    }

    /// Polls the reliable connection for messages.
    pub async fn poll_reliable(&mut self) -> anyhow::Result<()> {
        // Use a short timeout to avoid blocking.
        match tokio::time::timeout(std::time::Duration::from_millis(10), self.reliable.recv()).await
        {
            Ok(Ok(msg)) => {
                self.handle_reliable_message(msg).await?;
            }
            Ok(Err(e)) => {
                warn!(error = %e, "Reliable connection error");
                self.state = ClientState::Disconnected;
            }
            Err(_) => {
                // Timeout, no message available.
            }
        }
        Ok(())
    }

    async fn handle_reliable_message(&mut self, msg: NetMsg) -> anyhow::Result<()> {
        match msg {
            NetMsg::MapInfo(info) => {
                info!(map = %info.name, "Server sent map info");
                self.pending_map = Some(info.clone());
                self.state = ClientState::LoadingMap;

                // Try to load the map.
                self.load_map(&info.name)?;
            }
            NetMsg::EntitySpawn(spawn) => {
                debug!(classname = %spawn.classname, "Entity spawn received");
                self.spawned_entities.push(spawn);
            }
            NetMsg::ServerPrint { message } => {
                info!(message = %message, "Server message");
                self.server_messages.push(message);
            }
            NetMsg::Disconnect { reason } => {
                info!(reason = %reason, "Disconnected from server");
                self.state = ClientState::Disconnected;
            }
            other => {
                debug!(?other, "Unhandled reliable message");
            }
        }
        Ok(())
    }

    /// Loads a map by name.
    pub fn load_map(&mut self, map_name: &str) -> anyhow::Result<()> {
        info!(map = %map_name, "Loading map");

        let path = self.maps_dir.join(format!("{}.bsp", map_name));
        let bsp = BspMap::load(&path).with_context(|| format!("load map {}", path.display()))?;

        info!(
            map = %bsp.name,
            entities = bsp.entities.len(),
            vertices = bsp.vertices.len(),
            "Map loaded on client"
        );

        self.current_map = Some(bsp);
        self.spawned_entities.clear();
        self.snaps = SnapshotBuffer::new(32);
        self.state = ClientState::Ready;

        Ok(())
    }

    /// Sends a "ready" signal to the server.
    pub async fn send_ready(&mut self) -> anyhow::Result<()> {
        self.unreliable
            .send(&NetMsg::ClientReady {
                client_id: self.client_id,
            })
            .await?;
        info!("Sent ready signal to server");
        Ok(())
    }

    /// Advances one client tick: build input command and send.
    pub async fn tick(&mut self, input: InputState) -> anyhow::Result<PlayerCommand> {
        let cmd = build_command(self.client_id, self.tick, input);
        self.unreliable
            .send(&NetMsg::PlayerCommand(cmd.clone()))
            .await?;
        self.tick += 1;
        Ok(cmd)
    }

    /// Receives messages over unreliable channel.
    pub async fn recv_snapshot(&mut self) -> anyhow::Result<()> {
        if let Some(msg) = self
            .unreliable
            .recv_timeout(std::time::Duration::from_millis(20))
            .await?
        {
            match msg {
                NetMsg::Snapshot(s) => {
                    self.snaps.push(s);
                }
                other => {
                    debug!(?other, "Unexpected UDP message");
                }
            }
        }
        Ok(())
    }

    /// Executes a console command.
    pub async fn exec_console(&mut self, line: &str) -> anyhow::Result<Vec<String>> {
        let line = line.trim();
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        match tokens[0] {
            "connect" => {
                if tokens.len() < 2 {
                    return Ok(vec!["Usage: connect <host:port>".to_string()]);
                }
                Ok(vec![format!("Would connect to {}", tokens[1])])
            }
            "disconnect" => {
                self.state = ClientState::Disconnected;
                Ok(vec!["Disconnected".to_string()])
            }
            "status" => {
                let mut out = Vec::new();
                out.push(format!("State: {:?}", self.state));
                out.push(format!("Client ID: {:?}", self.client_id));
                out.push(format!("Tick: {}", self.tick));
                if let Some(ref map) = self.current_map {
                    out.push(format!("Map: {}", map.name));
                }
                out.push(format!("Snapshots buffered: {}", self.snaps.len()));
                Ok(out)
            }
            "map" => {
                if tokens.len() < 2 {
                    return Ok(vec!["Usage: map <mapname>".to_string()]);
                }
                match self.load_map(tokens[1]) {
                    Ok(()) => Ok(vec![format!("Map '{}' loaded locally", tokens[1])]),
                    Err(e) => Ok(vec![format!("Failed to load map: {}", e)]),
                }
            }
            "say" => {
                let msg = tokens[1..].join(" ");
                self.unreliable
                    .send(&NetMsg::ClientCommand {
                        command: format!("say {}", msg),
                    })
                    .await?;
                Ok(vec![])
            }
            "quit" | "exit" => {
                std::process::exit(0);
            }
            _ => {
                // Delegate to console system.
                self.console.exec(line)
            }
        }
    }

    /// Returns the underlying reliable connection peer.
    pub fn server_peer(&self) -> anyhow::Result<SocketAddr> {
        self.reliable.peer_addr()
    }
}
