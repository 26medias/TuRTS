# MP1 design revision 4 re-gate: network and host

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=1

I reviewed design revision 4 (head 5158f96) through two lenses, network and host. The lifecycle structure is sound. Every gate-1 finding has a real text change. One instrument defect is High: row ORDW1 cannot go red on its named plant. Six Medium gaps need short text repairs. All repairs are text, but a re-gate of the repair is still necessary.

**Gate-1 findings (network report, then backend report)**
- net 1, lobby vs claim: CLOSED. See 6.7.3:727-743, 6.7.5:821-832, LOB1.
- net 2, disconnect defeat in the log: CLOSED. See 6.7.8:941-991, MAT1 (h).
- net 3, sessionStorage: CLOSED. D-H (a); step 19 says "same tab". The id is saved at `Seated` (6.7.7:921-929).
- net 4, `unit_orders` as changes: PARTIAL. The text is right (6.8:1178-1188), but ORDW1 cannot go red. See N1.
- net 5, bucket first: CLOSED (6.8:1198-1201, D-E2). One gap remains (N10).
- net 6, visibility classes: CLOSED (6.8:1128-1150, 6.1:453-456).
- net 7, net 8, net 9, net 11, net 12: CLOSED. See FX1 (d), RLY1, CON2, 6.8:1203, 6.7.5:854-856.
- net 10, no statistics before the end: PARTIAL. The RES1 detector only finds fields that it knows. See N6.
- backend H1, state machine and next match: CLOSED (6.7.2, 6.7.5). The exit rule in Ended contradicts itself. See N3.
- backend H2, H4, M5, M6, L9, L10: CLOSED.
- backend H3, `Immediate` default: CLOSED. I counted the ignored process tests: 31 `#[ignore`, which matches 6.7.4:757.
- backend M7, disconnect lag: CLOSED (6.7.8:948-964, `--rejoin`).
- backend M8, test hooks: PARTIAL. 6.7.12 lists the hooks, but no scope item or lane builds them. See N5.

**Lifecycle trace (A, B = browsers; S = server)**
1. **Hello.** A sends `Hello`. S puts A in seat 0 with the lowest free colour, then sends `LobbyState{you:0}` to A and the next revision to all members. The sim does not tick (6.7.5:816). Today it ticks: my probe showed 60 ticks with 0 hellos. A second `Hello` on a seated connection is undefined. Today `has_session` blocks it (session.rs:216), but a seated connection has no `Session`. (L)
2. **Ready.** The client sends `LobbyAction{revision, SetReady}` and S decides. `lobby_changed` blocks a stale ready. A no-op action (ready when already ready) is undefined: does it raise the revision, and does it end the countdown? (L)
3. **Countdown.** S counts fixed steps; the client counts `remaining_ms` itself. A reload during the countdown is not seen until L (about 10 s). So the start claims the slot for B's dead connection, and B joins Running late. The tables define this, but 7.6:1624 says "the same seat, Not ready". (L)
4. **Start.** S claims seat i to slot i (Slots::claim takes the first open slot: slots.rs:46-57). Then `Welcome{tick 0}`, the tick-0 Snapshot and `MatchConfig`. The schedule of the start step is undefined. See N4.
5. **Orders before the start.** S answers `match_not_started` and charges a token. But the buckets are keyed by slot (session.rs:47-61), and a seat has no slot. (N10)
6. **Reload in Running.** For up to L, S sends "open in another window" (session.rs:157) and the client retries every 1 s for 15 s (rejoin.rs:5-6). At the notice, `SlotConnection false` enters the log. The retry gets `Welcome`, and `connected:true` goes in at the next tick. The other client sees `MatchState.connections` in the deltas. This step is defined.
7. **Disconnect past the grace time.** The sim counts from `since_tick` and gives the defeat at d+N in the delta of tick t. This step is defined.
8. **Surrender.** The order goes through the normal path with the slot from the `Session`. `OrderEnvelope` has no slot field (messages.rs:61-67), so a client cannot surrender another slot. But today the server refuses an envelope with no units. See N7.
9. **End.** The delta of tick t rides the unreliable channel; `MatchResult` rides the reliable one. Recovery of a lost end delta is undefined. See N2.
10. **After the result.** `LeaveMatch` is defined only in Ended (N8). The exit rule conflicts with the reconnect rule (N3).

**Authority:** no client decides ready, slot, colour, start, another slot's surrender, or the result. The countdown and the banner count are presentation only.

**New findings**

N1 (High). The ORDW1 red plant (1) cannot go red. The Snapshot path builds the order table from an empty map and then replaces it (replica.rs:403-418, :563). So after the resync the stale record is gone, also on the plant build. Repair: "Case A, no drop: on the next report after the clearing delta, the unit has no current order (red on plant 1). Case B, drop and resync: plant (3) the replica merges the Snapshot table into the old table."

N2 (Medium). The end-tick delta can be lost, and 6.7.7:903 does not say how the client recovers. The client asks for a resync only when the next delta or snapshot shows a gap (ingest.rs:462, :796), and no tick runs after the end. Today `send_deltas` has no "already sent" guard (outbound.rs:14, :51-84) and publishing stops (plugin.rs:139-150). So the end delta is sent again at each fixed step until exit, by accident. That is up to 1,200 extra deltas per client in 60 s, which contradicts "costs almost nothing" (6.7.11:1054). Repair: "At the end tick, S sends the Snapshot of tick t on the reliable channel, then `MatchResult`, and sends no more deltas. The client shows the outcome only when its replica tick is at or after `end_tick`."

N3 (Medium, needs decision; 6.7.14 item 3). "Exit when no connection with a `Session` remains" (6.7.10:1018) defeats "an owner that did not leave gets `Welcome` and `MatchResult`" (6.7.5:869). A reload on the result screen ends the process. LEAVE1 passes only if A stays connected. Also, two of the three options in item 3 read the same. Options:
- (a) exit when every claimed slot has sent `LeaveMatch`, or at `after_match_secs`;
- (b) keep the text and delete the reconnect cell.

N4 (Medium). `SnapshotDue` is served in `PreUpdate` (session.rs:123-133, :685). If the start step is a fixed-step system and one frame runs two fixed steps, the Snapshot is of tick 1. Then LOB1 "the first snapshot tick is 0" can flake. Repair: "The start step builds and sends the tick-0 Snapshot itself, after `Welcome`."

N5 (Medium). No owner builds the hooks and the test package. `--start-now`, `--input-log`, the `connection` script kind, the `on:` timing, `--rejoin`, the report fields, `mods/standoff` and `mp1-serve.sh` appear only in 6.7.12. X11, X11b, X11c and X12 (4:21-24) and the lanes L-MATCH and L-LOBBY (backend only, 10:24-25) do not name them. Also, 11:74 puts the process per match in X12, but X11b owns it. Repair: "X11b builds the server hooks, `standoff` and `mp1-serve.sh`; X12 builds the client hooks."

N6 (Medium). The RES1 detector cannot find an unknown field. Repair: "The export test builds the export of each tick below the end tick two times, with two different statistics values, and asserts byte-equal output." That check is red for any leak.

N7 (Medium). Surrender with `units: []` is refused today. `to_sim` refuses every empty envelope except Move to a Point (orders.rs:34-41). The replay validator requires `EmptyCommand` for every empty order (economylog.rs:1106-1114). Neither change is named in 6.7.8 or in 6.10 row O9. Repair: name both exceptions in 6.7.8.

N8 (Medium). `LeaveMatch` in Lobby, Countdown or Running is undefined, and a client can send it at any time. Repair: "Before Ended, S ignores `LeaveMatch` and logs one line."

N9 (Medium). Two wire rules have no row that can fail:
- "sent only on change of rate" (6.8:1172-1176);
- the 6-B `Fired` index (6.8:1167-1170); FX1 checks only the meaning.
Repair: add to FX1: "200 firing riflemen for 100 ticks: at most one `WeaponRecord` per unit for each change of `ready_tick`, and the `Fired` postcard size is at most 8 B." Red: the cooldown sent each tick, or a string weapon field.

N10 (Low). Repair: "A refusal before the start charges the bucket of the seat index."

N11 (Low). Four items have no row. My grep counted 0 hits in section 12 for `lobby_changed`, `not_in_lobby`, `colour_unknown` and `all_disconnected`.

N12 (Low). The design does not name where the five new messages are registered. They need to go in `protocol_plugin.rs:26-43`, after `ResyncRequest`, and the comment at `channels.rs:5-6` needs an update.

N13 (Low). The same time has two names: "boot-to-Lobby" (6.7.10:1032) and "boot-to-ready" (1:68).

**Spec status**
- D-B keeps D25 (Decided) from the start tick and refines S:308 (Baseline). It reassigns no claim.
- D-C stays inside S:296, which is a recommendation and says "may".
- I found no Decided line that is broken. CEO confirmation still applies.

**What I ran** (claim | result that would refute it | observed)
1. `git rev-parse HEAD` | the artefact head | another hash | 5158f96, exit 0.
2. `cargo build -p turts_server --locked -j 6` with the target dir `mc-mp1-regate-net` | the build works | exit not 0 | exit 0, 35 s.
3. `cargo build -p turts_pack` and `turts-pack` on a scratch copy of `muster` | — | — | exit 0 for both.
4. `turts_server` with `--stats`, no client, 3 s | 6.7.5:810, the IMM1 green and the LOB1 red on today's tree | ticks 0 | ticks 60, hellos 0. Exit 130 from my SIGINT, as expected.
5. `rtk proxy grep -cF` of 35 lifecycle names across 6.7, 6.8, 6.10, 7.6 and 12 | the names agree | a name in a different form | they agree. The 0-row gaps are in N11.
6. `grep -c '#[ignore'` in `turts_server/tests` | 6.7.4:757 says 31 | another count | 31.
7. `grep ResyncRequest` in the client | the client detects a lost end delta | a timer-based trigger | only ingest.rs:462 and :796.
8. Read outbound.rs:14 and :51-84 | the server sends each delta one time | an "already sent" guard | none.
9. Read replica.rs:283-287 and :403-418 | ORDW1 plant 1 | a merge | the table is replaced.
10. `grep EmptyCommand` | 6.7.8:965 | no refusal | orders.rs:40, economylog.rs:1110.
11. `git status --porcelain` | — | — | only other agents' `regate-*` files are untracked. I wrote nothing.

**Not checked (NOT TRACED)**
- I made no live run with two clients. I did not build the client.
- Whether the replica accepts a Snapshot that it did not request (this matters for the N2 repair).
- Whether a browser reload closes the WebSocket cleanly.
- Whether the rejoin filter accepts a `Welcome` that comes minutes after `Seated`.

**What I changed:** none.

Files:
- /home/julien/Projects/TuRTS-mp1/docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md
- /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-net/stats.json
- /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-net/server.log
