# srctest

A modular Rust game-engine skeleton inspired by Source Engine 2014 concepts.

## Crates

| Crate | Description |
|-------|-------------|
| `engine_shared` | Math (Vec3/Quat/Mat4), minimal ECS, networking primitives (TCP reliable + UDP unreliable), config, event bus, resource manager, render/physics trait abstractions, BSP loader, console system with cvars. |
| `engine_client` | Client-side connection, input handling, prediction/interpolation stubs, snapshot buffering, map loading. |
| `engine_server` | Fixed-timestep authoritative server loop, entity management, command ingestion, snapshot broadcasting, map loading. |
| `engine_tests` | Integration and smoke tests. |

## Building

```bash
cargo build --release
```

Binaries are placed in `target/release/`:
- `server` – the game server
- `client` – the game client

## Running

### Start the server

```bash
cargo run -p engine_server --release -- --addr 127.0.0.1:40000 --tick-hz 64 --maps-dir ./maps
```

Or run the binary directly:

```bash
./target/release/server --addr 127.0.0.1:40000 --tick-hz 64 --maps-dir ./maps
```

The server will print `Server listening` and provide an interactive console.

### Server Console Commands

Once the server is running, type commands at the prompt:

| Command | Description |
|---------|-------------|
| `map <mapname>` | Load a BSP map file (e.g., `map de_dust2`) |
| `status` | Show server state, tick count, connected clients |
| `kick <client_id>` | Disconnect a client |
| `say <message>` | Broadcast a message to all clients |
| `quit` | Shut down the server |
| `cvarlist` | List all console variables |
| `set <cvar> <value>` | Set a console variable |

### Start the client

In another terminal:

```bash
cargo run -p engine_client --release -- --addr 127.0.0.1:40000 --maps-dir ./maps --name "PlayerOne"
```

Or:

```bash
./target/release/client --addr 127.0.0.1:40000 --maps-dir ./maps --name "PlayerOne"
```

The client connects, receives map info from the server, loads the BSP, and sends a ready signal.

### Client Console Commands

| Command | Description |
|---------|-------------|
| `status` | Show client state, connection info |
| `disconnect` | Disconnect from server |
| `quit` | Exit the client |
| `say <message>` | Send chat message to server |
| `cvarlist` | List all console variables |
| `set <cvar> <value>` | Set a console variable |

## Multiplayer Flow

1. **Server startup**: Run the server with `--maps-dir` pointing to your BSP files
2. **Load a map**: On the server console, type `map <mapname>` (the `.bsp` extension is optional)
3. **Client connects**: Client performs TCP handshake, receives `Welcome` with its ClientId
4. **Map info sent**: Server sends `MapInfo` packet with map name and checksum
5. **Client loads map**: Client loads the BSP from its local `maps/` directory
6. **Ready signal**: Client sends `ClientReady` to indicate it's ready for gameplay
7. **Gameplay**: Server sends `Snapshot` packets, client sends `PlayerCommand` packets

## BSP Map Support

The engine supports Source Engine BSP format (versions 19-21). Place your `.bsp` files in the `maps/` directory:

```
maps/
  de_dust2.bsp
  cs_office.bsp
  ...
```

Parsed BSP data includes:
- Entity lump (spawn points, triggers, etc.)
- Geometry (planes, vertices, edges, faces)
- Brush data
- Models (brush entities)

## Running tests

```bash
cargo test
```

This runs:
- **Unit tests** in `engine_shared` (math, ECS, net codec)
- **Integration tests** in `engine_tests` (full client↔server roundtrip over sockets)
- **Smoke tests** (server runs a few ticks without panicking)

## Architecture overview

```
┌────────────┐         TCP (handshake)        ┌────────────┐
│   Client   │◄──────────────────────────────►│   Server   │
│            │         UDP (commands/snaps)   │            │
│  ┌───────┐ │◄──────────────────────────────►│ ┌────────┐ │
│  │ Input │ │                                │ │  ECS   │ │
│  │ Interp│ │                                │ │ World  │ │
│  └───────┘ │                                │ └────────┘ │
└────────────┘                                └────────────┘
        │                                            │
        └───────────── engine_shared ────────────────┘
                (math, net, config, events, ...)
```

## Notes

- **Determinism**: The server runs a fixed-timestep loop; avoid wall-clock branching in gameplay code.
- **No unsafe**: The codebase avoids `unsafe` entirely.
- **Placeholder systems**: Rendering and physics are trait-based stubs (`NullRenderer`, `NullPhysics`).
- **macOS tested**: Builds and runs on Apple Silicon and Intel Macs.

## License

MIT OR Apache-2.0
