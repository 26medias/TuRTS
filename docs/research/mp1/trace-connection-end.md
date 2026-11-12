# Trace: how the server learns that a client connection ended (MP1 open point 21)

Task `L-ROOM0`, backend-engineer, 2026-11-12. Lane head `d7d7e6d`. Design: MP1 design revision 6.2, section 15 point 21 (D:2617) and the section 14 row "The server detects a disconnect late" (D:2534). Registry sources: `lightyear` 0.31.0 and its sub-crates, `aeronet_io` 0.22.0-rc.1 (cargo registry, `~/.cargo/registry/src/*/`); `aeronet_websocket` 0.22.0-rc.1 is the vendored copy `vendor/aeronet_websocket`.

## Result in six lines

1. **Two signals exist.** The transport signal (`Add<Unlinked>`, then `Add<Disconnected>`, both on the `ClientOf` entity) comes 2 to 3 ms after a client process ends, killed or clean. The despawn of the `ClientOf` entity comes 10.0 s later, from the netcode timeout.
2. **Schedule.** Both are in `PreUpdate`. The transport signal is in the `aeronet_io::IoSystems::Poll` set, before `LinkSystems::Receive`. The despawn is in `ConnectionSystems::Receive`, after `LinkSystems::Receive` (probe, below).
3. **Today's only handler** is the observer `on_session_removed` on `On<Remove<Session>>` (`crates/turts_server/src/session_diagnostics.rs:383-384`, added only with `--stats` at `crates/turts_server/src/stats.rs:105`). It fires at the despawn, so it sees the 10 s signal and not the 2 ms signal.
4. **Measured L (release, native):** 9995.7 to 9998.9 ms for SIGKILL and for a clean exit; 10000.3 ms for a frozen client (SIGSTOP). `docs/measurements/mp1.md`, section "Disconnect notice L, native release".
5. **Hook for `L-MATCH-SRV1` (review ruling, section below):** an observer `On<Add<lightyear Disconnected>>` with `With<ClientOf>` that queues the entity and its slot (read inside the observer); one `PreUpdate` system before `receive_hello` calls `Room::connection_ended` (and, under `Immediate`, writes the slot change from the queued slot).
6. **Do not remove `Session`.** The liveness test at `session.rs:212` becomes `Query<&Session, Without<Disconnected>>`; the `Session` of an ended entity stays until the despawn, so today a reload still gets "This game is open in another window" for 10 s.

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

Ruling of the network-engineer review of `L-ROOM0`, accepted by the orchestrator for the wiring task. It replaces the first version of this section (which advised removing `Session`).

(a) **The observer.** `On<Add<lightyear::prelude::Disconnected>>` with `With<ClientOf>`. Name the lightyear type in full: `aeronet_io` has an event with the same name (`aeronet_io::connection::Disconnected`, `aeronet_io-0.22.0-rc.1/src/connection.rs:206`), and `lightyear_aeronet-0.31.0/src/lib.rs:137` observes that one. The lightyear component fires on the transport path (through `Unlinked`, `lightyear_connection-0.31.0/src/client.rs:205-218`) and on the netcode path (`server_plugin.rs:397-401`). It fires in all three measured cases. The observer queues `(entity, Option<SlotId> read from Session)` and reads the slot **inside the observer**: on the netcode path the entity is despawned in the same flush (`server_plugin.rs:397-401`), so a later system cannot read the `Session`. Do not use `Add<Unlinked>` (it misses SIGSTOP) and do not use `Remove<Session>` (10 s late).

(b) **One `PreUpdate` system before `receive_hello`** (`session.rs:123-133`). It drains the queue. For each entry it calls `Room::connection_ended(ConnectionKey(entity.to_bits()))`. Under `Immediate` the room has no seat (review sequence R9: `Unknown`; test `r9_under_immediate_the_room_has_no_seat_and_names_no_slot`), so the system writes the slot connection change (`SlotConnection { slot, connected: false }`, design 6.7.8) from the slot that the observer queued. Under a lobby policy the slot comes from `ConnectionEnd::SlotDisconnected`. The observer runs inside `PreUpdate` (segments above), so a connection end and a `Hello` of the same frame arrive in the order that design 6.7.5 needs.

(c) **Do not remove `Session`.** Make the liveness test at `session.rs:212` one filter: `Query<&Session, Without<Disconnected>>` (lightyear type). Removing `Session` has three costs: staged commands of the closed connection then fail as `BadSequence` with player id 0 (`session.rs:389-412`, `:583-585`: both read `Session` from the connection entity); the `--stats` removal record moves from the despawn to the removal; and it needs a command on an entity that the netcode path may already have despawned.

Filter: a connection with no seat and no `Session` also becomes `Disconnected`, for example a kicked connection (`session.rs:712-720`, `kick_pending` triggers `Unlink`, measured: `PROBE add_unlinked` and `add_disconnected` with reason `User requested` +1022.8 ms in the reload case). `Room::connection_ended` returns `Unknown` for such a key; the observer queues `None` for the slot.

A transport stall cannot release a seat by mistake: the WebSocket backend ends a session only on a close frame, the end of the stream or an error; there is no idle timer in it (review reading of `vendor/aeronet_websocket/src/session/backend.rs`), and a reconnect is a new entity, so an old key never names the new connection (`r5_unknown_key_double_end_and_the_old_key_after_a_reconnect`).

### The review probe: transport signal and reload

Source: the network-engineer review probe of `L-ROOM0` (not a committed test): `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l-room0-review/` (`probe-net.diff`, `drive.py`, `probe-drive.log`). Times are from the end of the client.

| Case | `Add<Unlinked>` | `Add<Disconnected>` (reason) | `Remove<Session>` |
| --- | --- | --- | --- |
| SIGKILL of the headless client | +4.2 ms | +4.3 ms (`ResetWithoutClosingHandshake`) | +9998.1 ms |
| Clean exit of the headless client | -3.9 ms | -3.9 ms (`ResetWithoutClosingHandshake`: the client sends no close frame) | +9994.3 ms |
| SIGSTOP of the headless client | none | +9998.7 ms (`Disconnected by peer`, netcode) | +9998.8 ms |
| Raw WebSocket: close frame, then FIN | +0.3 ms | +0.4 ms (`Disconnected by peer`) | not a `ClientOf` |
| Raw WebSocket: close frame, TCP held open | +0.8 ms | +0.9 ms (`Disconnected by peer`) | not a `ClientOf` |
| Raw WebSocket: FIN, no close frame | +1.0 ms | +1.0 ms (`ResetWithoutClosingHandshake`) | not a `ClientOf` |
| Raw WebSocket: RST | +0.9 ms | +0.9 ms (`ConnectionReset`) | not a `ClientOf` |

Reload: a client with the same id that starts 500 ms after a SIGKILL gets "This game is open in another window" and is not welcomed (the `Session` of the dead entity still counts, `session.rs:225-227`, until +9999.1 ms). This is today's code, and it is why (c) is needed.

The first version of this note measured the same two signals with an own probe (table above, section "The run that proves it"); the two probes agree within about 1 ms.

## What stays NOT TRACED

- The browser (NOT RUN in a real browser): the WHATWG WebSockets standard says that an unloading page starts the closing handshake with code 1001, so a tab close or reload should give a close frame; a crash or a lost network gives no close frame, and then the signal is the transport error or the 10 s netcode timeout. The raw WebSocket rows above show that every one of these server-side endings is seen within 1 ms of the packet. No browser measured L (D:2534 "The browser lag is NOT TRACED").
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
