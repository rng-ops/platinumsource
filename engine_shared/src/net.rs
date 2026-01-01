//! Networking primitives.
//!
//! Goals:
//! - Provide a simple reliable (TCP) and unreliable (UDP) channel.
//! - Provide snapshot and command message types used by client/server.
//! - Keep serialization explicit and versionable.
//!
//! This is not a full Source-style netcode implementation; it is a scaffold.

use anyhow::Context;
use bytes::{BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicU32, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    time,
};

use crate::{ecs::EntityId, math::Vec3};

/// Protocol version for compatibility checks.
pub const PROTOCOL_VERSION: u32 = 1;

static NEXT_CLIENT_ID: AtomicU32 = AtomicU32::new(1);

/// Identifies a connected client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u32);

impl ClientId {
    pub fn new_unique() -> Self {
        ClientId(NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// High-level message envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetMsg {
    // ─── Connection handshake ───
    Hello {
        protocol: u32,
    },
    /// Client announces its UDP port to the server.
    UdpHello {
        client_udp_port: u16,
    },
    Welcome {
        client_id: ClientId,
    },

    // ─── Map loading ───
    /// Server tells client which map to load.
    MapInfo(MapInfo),
    /// Client confirms map is loaded and ready.
    ClientReady {
        client_id: ClientId,
    },

    // ─── Entity replication ───
    /// Server spawns an entity on the client.
    EntitySpawn(EntitySpawn),
    /// Server updates entity state (delta or full).
    EntityUpdate(EntityState),
    /// Server removes an entity.
    EntityDelete {
        id: EntityId,
    },

    // ─── Gameplay ───
    /// Client -> server: input commands for a given tick.
    PlayerCommand(PlayerCommand),
    /// Server -> client: world snapshot for interpolation.
    Snapshot(Snapshot),

    // ─── Console/chat ───
    /// Server -> client: print message to console.
    ServerPrint {
        message: String,
    },
    /// Client -> server: console command (e.g., "say hello").
    ClientCommand {
        command: String,
    },

    // ─── Disconnect ───
    Disconnect {
        reason: String,
    },
}

/// Map information sent to clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapInfo {
    /// Map name (e.g., "de_dust2").
    pub name: String,
    /// CRC32 checksum for integrity (optional, 0 if not computed).
    pub crc: u32,
    /// Map file size in bytes (for validation).
    pub size: u64,
}

/// Entity spawn packet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntitySpawn {
    pub id: EntityId,
    pub classname: String,
    pub position: Vec3,
    /// Additional properties as key-value pairs.
    pub properties: Vec<(String, String)>,
}

/// Client input for one tick.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerCommand {
    pub client_id: ClientId,
    pub tick: u32,
    /// Wish move/accel in local space (placeholder).
    pub wish: Vec3,
}

/// A minimal entity state for replication.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityState {
    pub id: EntityId,
    pub position: Vec3,
}

/// World snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub tick: u32,
    pub entities: Vec<EntityState>,
}

/// Reliable connection over TCP with length-prefixed frames.
#[derive(Debug)]
pub struct ReliableConn {
    stream: TcpStream,
}

impl ReliableConn {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub async fn send(&mut self, msg: &NetMsg) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(msg).context("serialize msg")?;
        let mut buf = BytesMut::with_capacity(4 + payload.len());
        buf.put_u32(payload.len() as u32);
        buf.extend_from_slice(&payload);
        self.stream.write_all(&buf).await.context("tcp write")?;
        Ok(())
    }

    pub async fn recv(&mut self) -> anyhow::Result<NetMsg> {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .await
            .context("tcp read len")?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        self.stream
            .read_exact(&mut payload)
            .await
            .context("tcp read payload")?;
        let msg = serde_json::from_slice(&payload).context("deserialize msg")?;
        Ok(msg)
    }

    pub fn peer_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.stream.peer_addr()?)
    }
}

/// Unreliable channel over UDP.
#[derive(Debug)]
pub struct UnreliableConn {
    socket: UdpSocket,
    peer: SocketAddr,
}

impl UnreliableConn {
    pub async fn connect(bind_addr: SocketAddr, peer: SocketAddr) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(bind_addr).await.context("udp bind")?;
        socket.connect(peer).await.context("udp connect")?;
        Ok(Self { socket, peer })
    }

    pub async fn send(&self, msg: &NetMsg) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(msg).context("serialize udp msg")?;
        self.socket.send(&payload).await.context("udp send")?;
        Ok(())
    }

    pub async fn recv(&self) -> anyhow::Result<NetMsg> {
        let mut buf = vec![0u8; 64 * 1024];
        let n = self.socket.recv(&mut buf).await.context("udp recv")?;
        let msg = serde_json::from_slice(&buf[..n]).context("deserialize udp msg")?;
        Ok(msg)
    }

    /// Receives a datagram within the given timeout.
    pub async fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> anyhow::Result<Option<NetMsg>> {
        let mut buf = vec![0u8; 64 * 1024];
        match time::timeout(timeout, self.socket.recv(&mut buf)).await {
            Ok(Ok(n)) => {
                let msg = serde_json::from_slice(&buf[..n]).context("deserialize udp msg")?;
                Ok(Some(msg))
            }
            Ok(Err(e)) => Err(e).context("udp recv")?,
            Err(_) => Ok(None),
        }
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    pub fn local_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }
}

/// TCP server listener.
pub struct ReliableListener {
    listener: TcpListener,
}

impl ReliableListener {
    pub async fn bind(addr: SocketAddr) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await.context("tcp bind")?;
        Ok(Self { listener })
    }

    pub async fn accept(&self) -> anyhow::Result<(ReliableConn, SocketAddr)> {
        let (stream, addr) = self.listener.accept().await.context("tcp accept")?;
        Ok((ReliableConn::new(stream), addr))
    }

    pub fn local_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }
}

/// Convenience codec helpers.
pub fn encode_to_bytes(msg: &NetMsg) -> anyhow::Result<Bytes> {
    let payload = serde_json::to_vec(msg).context("serialize")?;
    Ok(Bytes::from(payload))
}

pub fn decode_from_bytes(b: &[u8]) -> anyhow::Result<NetMsg> {
    serde_json::from_slice(b).context("deserialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netmsg_roundtrip_bytes() {
        let msg = NetMsg::Hello {
            protocol: PROTOCOL_VERSION,
        };
        let bytes = encode_to_bytes(&msg).unwrap();
        let back = decode_from_bytes(&bytes).unwrap();
        assert_eq!(msg, back);
    }
}
