# Trace: how the server learns that a client connection ended (MP1 open point 21)

Task `L-ROOM0`, backend-engineer, 2026-11-12. Lane head `d7d7e6d`. Design: MP1 design revision 6.2, section 15 point 21 (D:2617) and the section 14 row "The server detects a disconnect late" (D:2534). Registry sources: `lightyear` 0.31.0 and its sub-crates, `aeronet_io` 0.22.0-rc.1 (cargo registry, `~/.cargo/registry/src/*/`); `aeronet_websocket` 0.22.0-rc.1 is the vendored copy `vendor/aeronet_websocket`.

## Result in six lines

1. **Two signals exist.** The transport signal (`Add<Unlinked>`, then `Add<Disconnected>`, both on the `ClientOf` entity) comes 2 to 3 ms after a client process ends, killed or clean. The despawn of the `ClientOf` entity comes 10.0 s later, from the netcode timeout.
2. **Schedule.** Both are in `PreUpdate`. The transport signal is in the `aeronet_io::IoSystems::Poll` set, before `LinkSystems::Receive`. The despawn is in `ConnectionSystems::Receive`, after `LinkSystems::Receive` (probe, below).
3. **Today's only handler** is the observer `on_session_removed` on `On<Remove<Session>>` (`crates/turts_server/src/session_diagnostics.rs:383-384`, added only with `--stats` at `crates/turts_server/src/stats.rs:105`). It fires at the despawn, so it sees the 10 s signal and not the 2 ms signal.
4. **Measured L (release, native):** 9995.7 to 9998.9 ms for SIGKILL and for a clean exit; 10000.3 ms for a frozen client (SIGSTOP). `docs/measurements/mp1.md`, section "Disconnect notice L, native release".
5. **Hook for `L-MATCH-SRV1`:** an observer `On<Add<Disconnected>>` on entities `With<ClientOf>` (it covers the transport path and the netcode path). It queues the entity; one room system in `PreUpdate`, before `receive_hello`, calls `Room::connection_ended`.
6. **A second change is needed with the hook:** `receive_hello` decides "a live session" from `Query<&Session>` (`crates/turts_server/src/session.rs:225-227`), and the `Session` component stays on the entity until the despawn. The hook must remove the `Session` component, or the 10 s remains for the reload.

## Facts, with the lines that show them

### What the server code does today

- `Session` is put on the `ClientOf` entity at an accepted `Hello` (`crates/turts_server/src/session.rs:269`). No code in `crates/turts_server/src` removes it. A grep for `Disconnected`, `Unlinked` and `on_disconnect` in non-room, non-test files of `crates/turts_server/src` finds none (run, below).
- `has_live_session` is true when any entity has a `Session` with the same `player_id` (`session.rs:225-227`). `judge_hello` turns it into "This game is open in another window" (`session.rs:155-157`).
- The server sets no timeout. With `Manual` tokens the timeout comes from the token; the client sets 10 s (`crates/turts_server/src/net.rs:36-37`, `crates/turts_client/src/join.rs:345`). The server reads it at `lightyear_netcode-0.31.0/src/server.rs:786`.
- The refused-connection path triggers `Unlink` on the entity after `KICK_DELAY` (`session.rs:712-720`). That is a server-side close, not a client end.

### The path of a transport close (killed or exited client)

1. `aeronet_websocket::server::poll_connected` runs in `PreUpdate`, set `IoSystems::Poll` (`vendor/aeronet_websocket/src/server/mod.rs:37-41`, `:279`). `try_disconnect` triggers `Disconnected { entity, reason }` on the aeronet session entity when the backend task ends (`mod.rs:288-300`).
2. `aeronet_io` despawns the aeronet session entity (`aeronet_io-0.22.0-rc.1/src/connection.rs:206-220`).
3. `lightyear_aeronet` observer `on_disconnected` does `try_insert(Unlinked { reason })` on the lightyear `Link` entity, which is the `ClientOf` entity (`lightyear_aeronet-0.31.0/src/lib.rs:137-157`). The `Link` entity is made per accepted session in `lightyear_websocket-0.31.0/src/endpoint.rs:76-96`. `LinkSystems::Receive` runs after `IoSystems::Poll` (`lightyear_aeronet-0.31.0/src/lib.rs:233`).
4. `lightyear_connection` observer `disconnect_if_link_fails` inserts `Disconnected { reason: LinkFailed }` on an entity that has no `Server` (`lightyear_connection-0.31.0/src/client.rs:205-218`). `Disconnected::on_add` removes `Connected` (`client.rs:128-145`).
5. Nothing in this path despawns the `Link` entity. The only despawns of it are: the netcode `receive` system (`lightyear_netcode-0.31.0/src/server_plugin.rs:384-402`), the endpoint `Unlinked` observer (`lightyear_link-0.31.0/src/endpoint.rs:75-90`, server stop), and the `Disconnecting` system (`lightyear_connection-0.31.0/src/server.rs:145-161`, server stop). So the entity and its `Session` stay.

### The path of the netcode timeout (and of a netcode `Disconnect` packet)

- `NetcodeServerPlugin::receive` runs in `PreUpdate`, set `ConnectionSystems::Receive` (`server_plugin.rs:496`). The sets are chained `LinkSystems::Receive`, `ConnectionSystems::Receive`, `TransportSystems::Receive` (`server_plugin.rs:477-485`).
- `receive` calls `update_state` (`server_plugin.rs:319`), which calls `check_for_timeouts` (`lightyear_netcode-0.31.0/src/server.rs:845-863, 981-986`). A client whose `last_receive_time + timeout < time` gets `on_disconnect` (`server.rs:859`), which queues `(id, entity)` in `disconnections`.
- Then `receive` drains `disconnections` and does `try_insert(Disconnected { reason: ByPeer(None) })` and `despawn()` on the entity (`server_plugin.rs:384-402`). A client `Disconnect` packet takes the same queue (`server.rs:583-587`).

### The run that proves it

Tool: the new process test `crates/turts_server/tests/connection_end.rs` (release, `--stats`), and a temporary logging patch that is NOT in the commit. The patch adds observers for `Add<Unlinked>`, `Add<Disconnected>`, `Remove<ClientOf>` and `Remove<Session>`, and four marker systems that write the schedule segment into a resource; each observer prints the unix microseconds and the segment. The patch is kept in the scratchpad (`probe_ce_final.rs`) and in the evidence below; it only logs.

| Case | Client end (unix µs) | `Add<Unlinked>` | `Add<Disconnected>` | Despawn and `Remove<Session>` |
| --- | --- | --- | --- | --- |
| SIGKILL | 1794470879068775 | +3.1 ms, segment `before-LinkSystems::Receive` | +3.1 ms, same segment | +9998.8 ms, segment `after-LinkSystems::Receive-before-ConnectionSystems::Receive` |
| Clean exit (`--run-secs 4`) | 1794470868794441 | -2.5 ms (before the test read the exit), same segment | -2.4 ms | +9995.7 ms, same segment as above |
| SIGSTOP (no transport close) | 1794470948918957 | none | +10.0 s, segment after `LinkSystems::Receive` | +10.0 s, same instant |

Evidence in the runner folder `/home/julien/Projects/TuRTS/target/mc-evidence/` (outside the repository): `mp1-lroom0-conn-end-probe3/command.log` (SIGKILL and clean), `mp1-lroom0-conn-end-probe-sigstop/command.log`; the `PROBE` lines are in the `server.log` of the test directories `target/mc-mp1-l-room0/tmp/server_2491069_0`, `server_2491069_1`, `server_2511941_0`. Copies are in the scratchpad `probe-evidence/`.

What the table says: for a closed transport, the end is known in `PreUpdate` at once; the `Session` is removed only 10 s later. For a silent loss, the first signal is the netcode timeout at 10 s, in the same flush as the despawn. `Add<Disconnected>` fired in all three cases. `Add<Unlinked>` did not fire for the frozen client.

## Recommended hook for `L-MATCH-SRV1` (release the seat on connection end)

- Observe `On<Add<Disconnected>>` and keep the entities that are `With<ClientOf>`. Do not use `Remove<Session>`: it is 10 s late in every measured case. Do not use `Add<Unlinked>`: it misses the frozen client.
- The observer only queues the entity into a resource (`ConnectionEnds(Vec<Entity>)`). A system in `PreUpdate` drains it before `receive_hello` (`session.rs:123-133` is the receive chain) and calls `Room::connection_ended(ConnectionKey(entity.to_bits()))` (`crates/turts_server/src/room.rs`). The observer runs inside `PreUpdate` (segments above), so a connection end and the `Hello` of the same frame arrive in the order that design 6.7.5 needs ("a seat whose connection ended in that frame is released first").
- The same system removes the `Session` component from the ended entity (a command), so that `session.rs:225-227` no longer counts it as a live session. Or `receive_hello` asks `Room` for liveness (`Seat::connection`) in place of `Query<&Session>`. Without one of these, the reload of a player still gets "This game is open in another window" for 10 s, although the room released the seat. The `Remove<Session>` observer of `--stats` then fires at that removal and not at the despawn; the observer file records change, so `connection_end.rs` and the `session_authority_removed` readers need a second look (`L-MATCH-SRV1` decides).
- Filter: the observer must ignore connections that have no seat and no `Session` (a refused connection that gets a kick, `session.rs:712-720`, also becomes `Disconnected`). `Room::connection_ended` returns `ConnectionEnd::Unknown` for them.
- `ConnectionEnd::SlotDisconnected` in Running is the value that `L-MATCH-SRV1` turns into `SlotConnection { slot, connected: false }` (design 6.7.8).

What this changes in the design: the 10 s "L" of 6.7.8 is the netcode timeout of the entity, and it is the notice time only for a silent loss (frozen process, cable pulled, as far as TCP does not report it). For a closed transport the notice can be 2 to 3 ms if the hook above is used. That is a design input, not a decision: the 6.7.14 item 4 ("keep the 10 s netcode timeout") is not touched. Needs the owner of 6.7.8 to say whether LOB1 and MAT1 keep their 11 s allowance (it still holds).

## What stays NOT TRACED

- The browser: whether a tab close or a reload sends a WebSocket close frame or a TCP FIN to the server before the page is gone (D:2534 "The browser lag is NOT TRACED"). The path above starts from the end of the TCP session, so a close that the server sees should give the 2 to 3 ms signal; a page that is frozen or a network that drops silently should give 10 s. Not run in a browser.
- A client that sends the netcode `Disconnect` packet (the headless `turts_client` did not: its clean exit gave 9995.7 ms and a transport signal first). Lightyear client code that sends it on `Disconnect` was not traced.
- A server under load, a release build on the 4-vCPU target, and other operating systems.
- The case of an `Unlinked` that is followed by a new link on the same entity: `Linked::on_insert` removes `Unlinked` (`lightyear_link-0.31.0/src/lib.rs:436-447`), but no run made a `ClientOf` link again.
- Server stop (`Stop`, `Unlink` of the endpoint): the endpoint observer despawns each `ClientOf` (`lightyear_link-0.31.0/src/endpoint.rs:75-90`); no run.

## Runs of the trace

| Label (`/home/julien/Projects/TuRTS/target/mc-evidence/<label>`) | Command | Exit |
| --- | --- | --- |
| `mp1-lroom0-conn-end-run1` | `cargo test --release --locked -p turts_server --test connection_end -- --ignored --test-threads=1 --nocapture` | 0, 2 passed |
| `mp1-lroom0-conn-end-probe2`, `-probe3` | same command, with the logging patch | 0 |
| `mp1-lroom0-conn-end-probe-sigstop` | same, test `probe_sigstop` (a scratch copy of the test file) | 0 |
