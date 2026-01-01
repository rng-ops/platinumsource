//! Server implementation.
//!
//! This is an authoritative server loop inspired by Source-style tick-based
//! simulation. It supports:
//! - BSP map loading
//! - Console commands (map, status, kick, quit)
//! - Client connection with map transfer flow
//! - Entity spawning from BSP entities
//! - Snapshot replication
//!
//! Determinism notes:
//! - Keep simulation in a fixed timestep.
//! - Avoid wall-clock-dependent branching in gameplay code.
//! - Use stable ordering when iterating collections.

use anyhow::Context;
use engine_shared::{
    bsp::BspMap,
    config::EngineConfig,
    console::{Console, CvarFlags, CvarValue},
    ecs::{EntityId, Position, World},
    math::Vec3,
    net::{
        ClientId, EntitySpawn, EntityState, MapInfo, NetMsg, PlayerCommand, ReliableConn,
        ReliableListener, Snapshot, PROTOCOL_VERSION,
    },
};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};
use tokio::{net::UdpSocket, sync::mpsc, time::Instant};
use tracing::{debug, info, warn};

/// Connected client state.
struct ClientState {
    _id: ClientId,
    reliable: ReliableConn,
    udp_peer: SocketAddr,
    last_cmd_tick: u32,
    /// Whether the client has finished loading the map.
    ready: bool,
    /// Entity ID assigned to this client's player.
    player_entity: Option<EntityId>,
}

/// Server state enum for connection flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerState {
    /// No map loaded, waiting for `map` command.
    Idle,
    /// Map is loading.
    LoadingMap,
    /// Map loaded, accepting clients and running simulation.
    Running,
}

/// Game server.
pub struct GameServer {
    pub cfg: EngineConfig,
    pub console: Console,
    world: World,
    clients: HashMap<ClientId, ClientState>,

    tcp: ReliableListener,
    udp: UdpSocket,

    tick: u32,
    state: ServerState,

    /// Currently loaded map.
    current_map: Option<BspMap>,
    /// Path to maps directory.
    maps_dir: PathBuf,

    /// Channel for console commands from stdin.
    console_rx: Option<mpsc::Receiver<String>>,
}

impl GameServer {
    /// Creates a new server with the given config.
    pub async fn new(cfg: EngineConfig, maps_dir: PathBuf) -> anyhow::Result<Self> {
        let addr: SocketAddr = cfg.server_addr.parse().context("parse server_addr")?;
        let tcp = ReliableListener::bind(addr).await?;
        let udp = UdpSocket::bind(addr).await.context("udp bind")?;

        let mut console = Console::new();
        Self::register_cvars(&mut console);

        Ok(Self {
            cfg,
            console,
            world: World::default(),
            clients: HashMap::new(),
            tcp,
            udp,
            tick: 0,
            state: ServerState::Idle,
            current_map: None,
            maps_dir,
            console_rx: None,
        })
    }

    /// Binds server sockets (legacy API for compatibility).
    pub async fn bind(cfg: EngineConfig) -> anyhow::Result<Self> {
        Self::new(cfg, PathBuf::from("maps")).await
    }

    fn register_cvars(console: &mut Console) {
        console.register_cvar(
            "sv_tickrate",
            CvarValue::Int(64),
            "Server tick rate",
            CvarFlags::NONE,
        );
        console.register_cvar(
            "sv_maxclients",
            CvarValue::Int(16),
            "Max connected clients",
            CvarFlags::NONE,
        );
        console.register_cvar(
            "sv_cheats",
            CvarValue::Bool(false),
            "Allow cheat commands",
            CvarFlags::REPLICATED,
        );
    }

    /// Sets the console input receiver.
    pub fn set_console_input(&mut self, rx: mpsc::Receiver<String>) {
        self.console_rx = Some(rx);
    }

    /// Returns the local address (after binding).
    pub fn local_addr(&self) -> anyhow::Result<SocketAddr> {
        self.tcp.local_addr()
    }

    /// Returns the current server state.
    pub fn state(&self) -> &ServerState {
        &self.state
    }

    /// Loads a map by name.
    pub fn load_map(&mut self, map_name: &str) -> anyhow::Result<()> {
        self.state = ServerState::LoadingMap;
        info!(map = %map_name, "Loading map");

        let path = self.maps_dir.join(format!("{}.bsp", map_name));
        let bsp = BspMap::load(&path).with_context(|| format!("load map {}", path.display()))?;

        info!(
            map = %bsp.name,
            entities = bsp.entities.len(),
            vertices = bsp.vertices.len(),
            faces = bsp.faces.len(),
            "Map loaded"
        );

        // Clear world and spawn entities from BSP.
        self.world = World::default();
        self.spawn_bsp_entities(&bsp);

        self.current_map = Some(bsp);
        self.tick = 0;
        self.state = ServerState::Running;

        // Notify connected clients about map change.
        // (Clients will need to reconnect or we send MapInfo).

        Ok(())
    }

    fn spawn_bsp_entities(&mut self, bsp: &BspMap) {
        for ent in &bsp.entities {
            // Skip worldspawn.
            if ent.classname == "worldspawn" {
                continue;
            }

            // Spawn entity in ECS.
            let id = self.world.spawn();
            if let Some(origin) = ent.origin() {
                self.world.insert(
                    id,
                    Position {
                        x: origin.x,
                        y: origin.y,
                        z: origin.z,
                    },
                );
            }

            debug!(id = ?id, classname = %ent.classname, "Spawned BSP entity");
        }
    }

    /// Returns map info for network transmission.
    pub fn map_info(&self) -> Option<MapInfo> {
        self.current_map.as_ref().map(|m| MapInfo {
            name: m.name.clone(),
            crc: 0, // TODO: compute CRC
            size: 0,
        })
    }

    /// Accepts exactly one client (handshake + map info).
    pub async fn accept_one(&mut self) -> anyhow::Result<ClientId> {
        let (mut conn, peer) = self.tcp.accept().await?;
        let msg = conn.recv().await?;
        match msg {
            NetMsg::Hello { protocol } if protocol == PROTOCOL_VERSION => {
                // Expect the client to announce its UDP port next.
                let udp_hello = conn.recv().await?;
                let client_udp_port = match udp_hello {
                    NetMsg::UdpHello { client_udp_port } => client_udp_port,
                    other => anyhow::bail!("expected UdpHello, got {other:?}"),
                };

                let id = ClientId::new_unique();
                conn.send(&NetMsg::Welcome { client_id: id }).await?;

                // Send map info if a map is loaded.
                if let Some(map_info) = self.map_info() {
                    conn.send(&NetMsg::MapInfo(map_info)).await?;
                }

                let udp_peer = SocketAddr::new(peer.ip(), client_udp_port);
                self.clients.insert(
                    id,
                    ClientState {
                        _id: id,
                        reliable: conn,
                        udp_peer,
                        last_cmd_tick: 0,
                        ready: false,
                        player_entity: None,
                    },
                );

                info!(client_id = ?id, %udp_peer, "Client connected");
                Ok(id)
            }
            other => anyhow::bail!("unexpected handshake msg: {other:?}"),
        }
    }

    /// Accepts a client with timeout (non-blocking).
    pub async fn try_accept(&mut self, timeout: Duration) -> anyhow::Result<Option<ClientId>> {
        match tokio::time::timeout(timeout, self.tcp.accept()).await {
            Ok(Ok((conn, peer))) => {
                // Handle handshake inline.
                self.handle_new_connection(conn, peer).await.map(Some)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(None), // Timeout
        }
    }

    async fn handle_new_connection(
        &mut self,
        mut conn: ReliableConn,
        peer: SocketAddr,
    ) -> anyhow::Result<ClientId> {
        let msg = conn.recv().await?;
        match msg {
            NetMsg::Hello { protocol } if protocol == PROTOCOL_VERSION => {
                let udp_hello = conn.recv().await?;
                let client_udp_port = match udp_hello {
                    NetMsg::UdpHello { client_udp_port } => client_udp_port,
                    other => anyhow::bail!("expected UdpHello, got {other:?}"),
                };

                let id = ClientId::new_unique();
                conn.send(&NetMsg::Welcome { client_id: id }).await?;

                if let Some(map_info) = self.map_info() {
                    conn.send(&NetMsg::MapInfo(map_info)).await?;
                }

                let udp_peer = SocketAddr::new(peer.ip(), client_udp_port);
                self.clients.insert(
                    id,
                    ClientState {
                        _id: id,
                        reliable: conn,
                        udp_peer,
                        last_cmd_tick: 0,
                        ready: false,
                        player_entity: None,
                    },
                );

                info!(client_id = ?id, %udp_peer, "Client connected");
                Ok(id)
            }
            other => anyhow::bail!("unexpected handshake msg: {other:?}"),
        }
    }

    /// Marks a client as ready and spawns their player entity.
    pub fn client_ready(&mut self, client_id: ClientId) -> anyhow::Result<EntityId> {
        let spawn_points = self
            .current_map
            .as_ref()
            .map(|m| m.spawn_points())
            .unwrap_or_default();

        let spawn_pos = spawn_points.first().copied().unwrap_or(Vec3::ZERO);

        let ent = self.world.spawn();
        self.world.insert(
            ent,
            Position {
                x: spawn_pos.x,
                y: spawn_pos.y,
                z: spawn_pos.z,
            },
        );

        if let Some(client) = self.clients.get_mut(&client_id) {
            client.ready = true;
            client.player_entity = Some(ent);
        }

        info!(client_id = ?client_id, entity = ?ent, "Client ready, player spawned");
        Ok(ent)
    }

    /// Runs the server for a number of ticks.
    pub async fn run_for_ticks(&mut self, ticks: u32) -> anyhow::Result<()> {
        let dt = Duration::from_secs_f32(1.0 / self.cfg.tick_hz as f32);
        let mut next = Instant::now();

        for _ in 0..ticks {
            next += dt;
            self.step(dt.as_secs_f32()).await?;
            tokio::time::sleep_until(next).await;
        }
        Ok(())
    }

    /// Executes one fixed simulation step.
    pub async fn step(&mut self, dt_sec: f32) -> anyhow::Result<()> {
        self.process_console_commands().await?;
        self.recv_commands().await?;
        self.simulate(dt_sec);
        if self.state == ServerState::Running {
            self.send_snapshots().await?;
        }
        self.tick += 1;
        Ok(())
    }

    async fn process_console_commands(&mut self) -> anyhow::Result<()> {
        // Collect lines first to avoid borrow conflict
        let lines: Vec<String> = if let Some(ref mut rx) = self.console_rx {
            let mut collected = Vec::new();
            while let Ok(line) = rx.try_recv() {
                collected.push(line);
            }
            collected
        } else {
            Vec::new()
        };

        for line in lines {
            self.exec_console(&line)?;
        }
        Ok(())
    }

    /// Executes a console command.
    pub fn exec_console(&mut self, line: &str) -> anyhow::Result<Vec<String>> {
        let line = line.trim();

        // Handle built-in server commands first.
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        match tokens[0] {
            "map" => {
                if tokens.len() < 2 {
                    return Ok(vec!["Usage: map <mapname>".to_string()]);
                }
                match self.load_map(tokens[1]) {
                    Ok(()) => Ok(vec![format!("Map '{}' loaded", tokens[1])]),
                    Err(e) => Ok(vec![format!("Failed to load map: {}", e)]),
                }
            }
            "status" => {
                let mut out = Vec::new();
                out.push(format!("Server state: {:?}", self.state));
                out.push(format!("Tick: {}", self.tick));
                if let Some(ref map) = self.current_map {
                    out.push(format!("Map: {}", map.name));
                }
                out.push(format!("Clients: {}", self.clients.len()));
                for (id, client) in &self.clients {
                    out.push(format!(
                        "  {:?}: udp={} ready={} entity={:?}",
                        id, client.udp_peer, client.ready, client.player_entity
                    ));
                }
                Ok(out)
            }
            "quit" | "exit" => {
                info!("Server shutting down");
                std::process::exit(0);
            }
            _ => {
                // Delegate to console system.
                self.console.exec(line)
            }
        }
    }

    async fn recv_commands(&mut self) -> anyhow::Result<()> {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match self.udp.try_recv_from(&mut buf) {
                Ok((n, from)) => {
                    if let Ok(msg) = serde_json::from_slice::<NetMsg>(&buf[..n]) {
                        self.handle_udp_message(from, msg).await;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e).context("udp recv")?,
            }
        }
        Ok(())
    }

    async fn handle_udp_message(&mut self, from: SocketAddr, msg: NetMsg) {
        match msg {
            NetMsg::PlayerCommand(cmd) => {
                self.on_command(from, cmd);
            }
            NetMsg::ClientReady { client_id } => {
                if let Err(e) = self.client_ready(client_id) {
                    warn!(client_id = ?client_id, error = %e, "Failed to mark client ready");
                }
            }
            NetMsg::ClientCommand { command } => {
                debug!(command = %command, "Client command received");
                // TODO: handle client console commands (say, etc.)
            }
            _ => {
                debug!(?msg, "Unexpected UDP message");
            }
        }
    }

    fn on_command(&mut self, from: SocketAddr, cmd: PlayerCommand) {
        if let Some(c) = self.clients.get_mut(&cmd.client_id) {
            c.udp_peer = from;
            c.last_cmd_tick = cmd.tick;

            // Apply movement to client's player entity.
            if let Some(eid) = c.player_entity {
                if let Some(pos) = self.world.get_mut::<Position>(eid) {
                    pos.x += cmd.wish.x * 0.1;
                    pos.y += cmd.wish.y * 0.1;
                    pos.z += cmd.wish.z * 0.1;
                }
            }
        }
    }

    fn simulate(&mut self, _dt_sec: f32) {
        // Placeholder for deterministic simulation systems.
    }

    async fn send_snapshots(&self) -> anyhow::Result<()> {
        let mut entities = Vec::new();
        for (eid, pos) in self.world.iter::<Position>() {
            entities.push(EntityState {
                id: eid,
                position: Vec3::new(pos.x, pos.y, pos.z),
            });
        }

        let snap = NetMsg::Snapshot(Snapshot {
            tick: self.tick,
            entities,
        });
        let payload = serde_json::to_vec(&snap).context("serialize snapshot")?;

        for c in self.clients.values() {
            if c.ready {
                let _ = self.udp.send_to(&payload, c.udp_peer).await;
            }
        }
        Ok(())
    }

    /// Sends entity spawn packets to a client.
    pub async fn send_entity_spawns(&mut self, client_id: ClientId) -> anyhow::Result<()> {
        let Some(map) = &self.current_map else {
            return Ok(());
        };

        let client = self
            .clients
            .get_mut(&client_id)
            .context("client not found")?;

        for ent in &map.entities {
            if ent.classname == "worldspawn" {
                continue;
            }

            let spawn = EntitySpawn {
                id: EntityId(0), // TODO: proper ID mapping
                classname: ent.classname.clone(),
                position: ent.origin().unwrap_or(Vec3::ZERO),
                properties: ent
                    .properties
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            };

            client.reliable.send(&NetMsg::EntitySpawn(spawn)).await?;
        }

        Ok(())
    }
}

/// Helper for tests: bind to an ephemeral port.
pub async fn bind_ephemeral(tick_hz: u32) -> anyhow::Result<(GameServer, EngineConfig)> {
    let cfg = EngineConfig {
        server_addr: format!("{}:{}", IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        tick_hz,
        ..Default::default()
    };

    // Bind TCP first to get an ephemeral port, then bind UDP to that same port.
    let tcp = ReliableListener::bind(cfg.server_addr.parse()?).await?;
    let addr = tcp.local_addr()?;
    let mut cfg = cfg;
    cfg.server_addr = addr.to_string();

    let udp_bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), addr.port());
    let udp = UdpSocket::bind(udp_bind).await?;

    let mut console = Console::new();
    GameServer::register_cvars(&mut console);

    Ok((
        GameServer {
            cfg: cfg.clone(),
            console,
            world: World::default(),
            clients: HashMap::new(),
            tcp,
            udp,
            tick: 0,
            state: ServerState::Running, // For tests, assume running
            current_map: None,
            maps_dir: PathBuf::from("maps"),
            console_rx: None,
        },
        cfg,
    ))
}
