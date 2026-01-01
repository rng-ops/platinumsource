//! Standalone server binary.
//!
//! Usage:
//!   cargo run -p engine_server -- [--addr 127.0.0.1:40000] [--tick-hz 64] [--maps-dir maps]
//!
//! The server listens for client connections, runs a fixed timestep simulation,
//! and broadcasts snapshots to connected clients.
//!
//! Console commands:
//!   map <mapname>  - Load a BSP map
//!   status         - Show server status
//!   quit           - Shutdown server

use std::env;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use anyhow::Context;
use engine_server::server::{GameServer, ServerState};
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
            "--tick-hz" if i + 1 < args.len() => {
                cfg.tick_hz = args[i + 1].parse().unwrap_or(64);
                i += 2;
            }
            "--maps-dir" if i + 1 < args.len() => {
                cfg.maps_dir = args[i + 1].clone();
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
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = parse_args();
    info!(addr = %cfg.server_addr, tick_hz = cfg.tick_hz, maps_dir = %cfg.maps_dir, "Starting server");

    let mut server = GameServer::new(cfg.clone(), PathBuf::from(&cfg.maps_dir))
        .await
        .context("create server")?;
    let local = server.local_addr()?;
    info!(%local, "Server listening");

    // Set up console input channel.
    let (console_tx, console_rx) = mpsc::channel::<String>(32);
    server.set_console_input(console_rx);

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
            if !line.is_empty() && console_tx.blocking_send(line).is_err() {
                break;
            }
        }
    });

    println!(
        "Server ready. Type 'map <mapname>' to load a map, 'status' for info, 'quit' to exit."
    );
    println!();

    // Main server loop.
    let tick_interval = std::time::Duration::from_secs_f32(1.0 / cfg.tick_hz as f32);
    let mut next_tick = tokio::time::Instant::now();

    loop {
        // Accept new clients (non-blocking).
        if let Ok(Some(cid)) = server.try_accept(std::time::Duration::from_millis(1)).await {
            info!(client_id = ?cid, "New client accepted");

            // If map is loaded, client will receive MapInfo and can load.
            // Mark client ready after they send ClientReady.
        }

        // Step simulation if running.
        if *server.state() == ServerState::Running {
            server.step(tick_interval.as_secs_f32()).await?;
        } else {
            // Still process console commands when idle.
            server.step(0.0).await?;
        }

        // Wait for next tick.
        next_tick += tick_interval;
        tokio::time::sleep_until(next_tick).await;
    }
}
