//! Standalone client binary.
//!
//! Usage:
//!   cargo run -p engine_client -- [--addr 127.0.0.1:40000] [--maps-dir maps]
//!
//! The client connects to the server, loads the map, sends input commands,
//! and displays received snapshots.
//!
//! Console commands:
//!   connect <host:port> - Connect to server (not yet implemented, pass --addr)
//!   disconnect          - Disconnect from server
//!   status              - Show client status
//!   map <mapname>       - Load a map locally (for testing)
//!   say <message>       - Send chat message
//!   quit                - Exit client

use std::env;
use std::io::{BufRead, Write};
use std::time::Duration;

use anyhow::Context;
use engine_client::client::{ClientState, GameClient};
use engine_client::input::InputState;
use engine_shared::config::EngineConfig;
use tokio::sync::mpsc;
use tracing::info;

fn parse_args() -> EngineConfig {
    let mut cfg = EngineConfig::default();
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--addr" if i + 1 < args.len() => {
                cfg.server_addr = args[i + 1].clone();
                i += 2;
            }
            "--maps-dir" if i + 1 < args.len() => {
                cfg.maps_dir = args[i + 1].clone();
                i += 2;
            }
            "--name" if i + 1 < args.len() => {
                cfg.player_name = args[i + 1].clone();
                i += 2;
            }
            _ => i += 1,
        }
    }
    cfg
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = parse_args();
    info!(server = %cfg.server_addr, maps_dir = %cfg.maps_dir, "Starting client");

    let mut client = GameClient::connect(&cfg).await.context("connect")?;
    info!(client_id = ?client.client_id, "Connected to server");

    // Set up console input channel.
    let (console_tx, mut console_rx) = mpsc::channel::<String>(32);

    // Spawn stdin reader thread.
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        loop {
            print!("] ");
            let _ = stdout.flush();
            let mut line = String::new();
            if stdin.lock().read_line(&mut line).is_err() {
                break;
            }
            let line = line.trim().to_string();
            if !line.is_empty() {
                if console_tx.blocking_send(line).is_err() {
                    break;
                }
            }
        }
    });

    println!("Client connected. Type 'status' for info, 'quit' to exit.");
    println!();

    // If we're in LoadingMap state, try to load and send ready.
    if client.state == ClientState::LoadingMap {
        let map_name = client.pending_map.as_ref().map(|m| m.name.clone());
        if let Some(name) = map_name {
            if client.load_map(&name).is_ok() {
                client.send_ready().await?;
            }
        }
    }

    // If already ready (map was already loaded), send ready signal.
    if client.state == ClientState::Ready {
        client.send_ready().await?;
    }

    let tick_interval = Duration::from_secs_f32(1.0 / cfg.tick_hz as f32);

    loop {
        // Process console commands.
        while let Ok(line) = console_rx.try_recv() {
            match client.exec_console(&line).await {
                Ok(output) => {
                    for line in output {
                        println!("{}", line);
                    }
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }

        // Check for reliable messages (map changes, etc.).
        client.poll_reliable().await?;

        // If disconnected, exit.
        if client.state == ClientState::Disconnected {
            println!("Disconnected from server.");
            break;
        }

        // If ready, run game loop.
        if client.state == ClientState::Ready {
            // Fake input for now - in a real client this would come from keyboard/mouse.
            let input = InputState {
                forward: 0.0,
                right: 0.0,
                up: 0.0,
            };

            if let Err(e) = client.tick(input).await {
                println!("Tick error: {}", e);
            }

            // Receive snapshots.
            if let Err(e) = client.recv_snapshot().await {
                println!("Snapshot error: {}", e);
            }

            // Print snapshot info occasionally.
            if let Some(snap) = client.snaps.last_snapshot() {
                if snap.tick % 64 == 0 {
                    info!(tick = snap.tick, entities = snap.entities.len(), "Snapshot");
                }
            }
        }

        tokio::time::sleep(tick_interval).await;
    }

    Ok(())
}
