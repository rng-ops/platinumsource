//! Full socket-based integration tests for client ↔ server communication.

use std::time::Duration;

use engine_client::input::InputState;
use engine_client::GameClient;
use engine_server::server::bind_ephemeral;
use engine_shared::config::EngineConfig;
use engine_shared::net::{decode_from_bytes, encode_to_bytes, ClientId, NetMsg, PROTOCOL_VERSION};

/// Unit-style test: protocol messages roundtrip correctly.
#[test]
fn protocol_messages_roundtrip() -> anyhow::Result<()> {
    let hello = NetMsg::Hello {
        protocol: PROTOCOL_VERSION,
    };
    assert_eq!(decode_from_bytes(&encode_to_bytes(&hello)?)?, hello);

    let udp_hello = NetMsg::UdpHello {
        client_udp_port: 50000,
    };
    assert_eq!(decode_from_bytes(&encode_to_bytes(&udp_hello)?)?, udp_hello);

    let welcome = NetMsg::Welcome {
        client_id: ClientId(1),
    };
    assert_eq!(decode_from_bytes(&encode_to_bytes(&welcome)?)?, welcome);

    Ok(())
}

/// Full integration: spawn server, connect client, exchange commands/snapshots.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_server_full_roundtrip() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_test_writer()
        .try_init();

    // Bind server to ephemeral port.
    let (mut server, cfg) = bind_ephemeral(64).await?;
    let server_addr = cfg.server_addr.clone();

    // Spawn server accept + step loop in background.
    let server_handle = tokio::spawn(async move {
        // Accept one client.
        let _cid = server.accept_one().await?;
        // Run 10 ticks (enough to send snapshots).
        for _ in 0..10 {
            server.step(1.0 / 64.0).await?;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Ok::<_, anyhow::Error>(())
    });

    // Give server a moment to start listening.
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect client.
    let mut client = GameClient::connect(&EngineConfig {
        server_addr,
        tick_hz: 64,
        maps_dir: "./maps".into(),
        player_name: "TestPlayer".to_string(),
    })
    .await?;

    // Send a few commands and try to receive snapshots.
    for _ in 0..10 {
        client
            .tick(InputState {
                forward: 1.0,
                right: 0.0,
                up: 0.0,
            })
            .await?;
        client.recv_snapshot().await?;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // Wait for server task to finish.
    server_handle.await??;

    // We should have received at least one snapshot.
    assert!(
        client.snaps.last_snapshot().is_some(),
        "expected at least one snapshot"
    );

    Ok(())
}
