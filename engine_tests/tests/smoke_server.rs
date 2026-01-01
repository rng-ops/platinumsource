use engine_server::server::bind_ephemeral;

/// Smoke test: server can run a few ticks without panicking.
#[tokio::test]
async fn server_runs_few_ticks() -> anyhow::Result<()> {
    let (mut server, _cfg) = bind_ephemeral(64).await?;
    server.run_for_ticks(3).await?;
    Ok(())
}
