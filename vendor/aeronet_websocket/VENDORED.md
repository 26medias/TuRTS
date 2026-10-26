# Vendored: aeronet_websocket

- Upstream: crates.io `aeronet_websocket` 0.22.0-rc.1, checksum
  `f0353f8c73cf65587a74056be1961f1ebb36607430dc3356b7e4e2038a4014a1`, git `82b2090e6f63d2f3a321a8fdcab2d823b2eee002`
  (`crates/aeronet_websocket`). Licence: MIT OR Apache-2.0.
- Used through `[patch.crates-io]` in the workspace `Cargo.toml`. The directory is in the workspace
  `exclude` list, so workspace fmt, clippy and test do not run on it.
- Decision: `vault/Decisions/2026-10-26 Server TCP no-delay and tighter T5 bound.md`.

## Why

The server does not disable Nagle's algorithm on accepted WebSocket sockets, and 0.22.0-rc.1 has no
option to do it (the client has `ClientConfig::disable_nagle`). On localhost at 20 Hz, a delta waited
for the ACK of the previous small segment: server send to client receive p90 15.8 ms, p99 39.2 ms.
T5 lag p99 was 1.62 to 1.96 ticks. With this patch it is 1.33 to 1.35 ticks.

## The exact change

The only change from the published crate. Everything else is byte-identical, except that the
registry marker `.cargo-ok` is not copied.

```diff
--- registry/aeronet_websocket-0.22.0-rc.1/src/server/backend.rs
+++ vendor/aeronet_websocket/src/server/backend.rs
@@ -60,6 +60,10 @@
             }
         };
         let (stream, peer_addr) = result.map_err(ServerError::AcceptConnection)?;
+        // Disable Nagle's algorithm: small, latency-sensitive frames must leave at once.
+        if let Err(err) = stream.set_nodelay(true) {
+            debug!("Failed to set TCP_NODELAY on {peer_addr}: {err:?}");
+        }
 
         tokio::spawn({
             let tx_connecting = tx_connecting.clone();
```

## Removal

Drop this directory, the `[patch.crates-io]` entry and the `exclude` entry when upstream ships a
server-side no-delay option, then set that option in `crates/turts_server/src/net.rs`.
