VERDICT=fail ITEMS=39 MISSING=15 PARTIAL=23 CONFLICT=0 RUNS=9 NOT_TRACED=6

# Gap audit: performance-architect (cost seat)

Scope: the cost side of selection and picking, health bars and status icons, fog of war, formations and
local avoidance, order queues, minimap, replays, and the player-facing latency and frame budgets.
Spec = `RTS-Engine-Project-Specifications.md` v0.7, cited S:line. Probe and model outputs:
`pa/sel.out`, `pa/model.out`, `pa/sweep.out` (all under this review directory).
Tick rate assumed: 20 Hz (the M1 prototype constant, `docs/measurements/m1.md:292`; D18 S:41 keeps it open).

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-IN01 | Screen-to-world picking | partial | S:610, S:688 (render validation only). M4 draft adds heightfield ray march (`m4-terrain-design.md:262`) | mechanism | M13 (render check) | M4 | terrain heightfield | yes: `crates/turts_client/src/economy_ui.rs:729-746` scans every replica unit and calls `ContentId::parse` (2 `String` allocs) per unit | Probe: 37.3 us p50 / 70.6 us p99 at 5,000 units; dense scan 3.3 / 6.2 us (11x). Native i9-14900K; wasm factor NOT TRACED |
| A-PC02 / B-2 | Box (drag) select | missing | "drag", "box", "marquee", "rectangle": 0 hits (`pa/sweep.out`) | mechanism | none | M4 (draft K1, `m4-terrain-design.md:27`) | A-IN01 | yes (same scan as A-IN01) | A drag re-runs the pass every frame. At 50,000 units the M3 pattern costs 436 us p50 per frame; dense 38 us |
| A-PC07 | Selection cap | partial | S:342 "affected entities" budget (Proposed); D39 S:62 forbids arbitrary caps | policy (cap) + mechanism (budget) | none | M4 | S:342 budgets | NOT TRACED | A cap is mod policy; the engine needs a technical bound on ids per command (S:342) |
| A-PC23 / B-13 | Selection visual | missing | "selection circle", "decal", "marker": 0 hits | mechanism | none | M4 | AN11 instancing | NOT TRACED | 200 decals: instanced, about 3 to 6 KiB/frame (model 4) |
| A-UI02 | Selection panel (200 icons) | partial | S:164 "command panels" only | mechanism + policy (layout) | none | M4 | A-PC02 | NOT TRACED | Cost of 200 Bevy UI nodes per rebuild: NOT TRACED. Needs a probe |
| A-PE04 | Large selection speed | missing | "hitch", "selection speed": 0 hits | mechanism | none | M4 | A-PC02 | yes (A-IN01) | Open budget: box-select pass time p99 at 1,000 / 5,000 / 50,000 replica units |
| A-OR24 | Group order fan-out | partial | S:342 "Support group orders" (Proposed); `MoveTo { unit_ids: Vec<UnitId> }` `crates/turts_net_protocol/src/messages.rs:55-56` | mechanism | none | M4 | S:630 path queue | no (wire is ready) | 200 ids x ~3 B = ~600 B per order. 200 per-unit path requests per click hit the S:630 per-tick cap; group route reuse (S:566) is the lever |
| A-OR11 | Order queue per unit | missing | "order queue", "queued order", "waypoint", "shift": 0 hits | mechanism | none | M5 (before M7 combat) | order model | NOT TRACED | Queue length needs a bound (S:342 "queued work"). Unbounded queue x 50,000 units is a memory defect |
| A-OR12 | Shift-queued orders | missing | "shift": 0 hits | mechanism + policy (key) | none | M5 | A-OR11 | NOT TRACED | Replicate the queue to the owner only; changed-only |
| A-OR13 | Waypoint display | missing | "waypoint": 0 hits | mechanism | none | M5 | A-OR11 | NOT TRACED | Per-frame line cost = selected units x queue length |
| A-GM01 | Destination spreading | partial | S:630 "shared destinations" test only | mechanism | none | M4 | A-OR24 | NOT TRACED | Without it 200 units stack on one point; avoidance cost rises |
| A-GM02 | Formations | partial | S:156 "formation primitives" (engine row); S:624 | both | none | M7 | A-GM01 | NOT TRACED | Group-level slot assignment is O(n log n) per order, not per tick |
| A-GM04 | Local avoidance | partial | D63 S:86 (Baseline); S:624 "bounded-neighbour" | mechanism | none | M4 or M7 | spatial index S:616 | NOT TRACED whether M1-M3 units overlap | M1 tick p99 228 us (`m1.md:138`) has no avoidance. Cost = units x k neighbours x passes; no k or budget stated |
| A-GM07 | Push idle friends | missing | "push": 0 player hits | mechanism | none | M7 | A-GM04 | NOT TRACED | Wakes sleeping units (S:616); must stay local |
| A-UI05 / B-24 | Minimap | partial | S:338 "minimap data may still be required" (data only) | mechanism + policy (look) | none | M5 | fog | NOT TRACED | 5,000 dots x 8 B = 39 KiB per redraw; at 10 Hz 0.40 MB/s (model 7). Cheap; needs a cadence |
| A-UI08 | Minimap alert flash | partial | S:338, S:634 "alerts" (replication only) | both | none | M5 | A-UI05 | NOT TRACED | |
| A-UI12 / B-28 | Health bars | missing | "health": 0 hits in the spec | mechanism (bars) + policy (modes) | none | M4 (combat exists since M3) | `HealthRecord` | yes: `HealthRecord { unit, current: u64 }` `messages.rs:256-259`, changed-only (`m3-industrial-design.md:472`) | Battle: all units change per tick, ~3 to 5 B each, 15 to 25 kB/tick per client at 5,000 units. No quantization budget. Render: 5,000 bars x 16 B = 78 KiB/frame, 4.8 MB/s at 60 fps |
| A-UI13 | Status icons | partial | S:604 "strategic icons" for distant units only | both | none | M7 | A-UI12 | NOT TRACED | Same instanced path as bars |
| A-UI14 / B-26 | Fog rendering | missing (design) | S:162, S:634 name the mechanism only | mechanism | none | M5 | A-FV02 | NOT TRACED | R8 texture 512x512: 256 KiB per update; 15.7 MB/s at 60 Hz, 2.6 MB/s at 10 Hz. 2048x2048: 251.7 MB/s at 60 Hz. Needs an update cadence budget |
| A-FV01 | Black shroud | partial | S:162 | mechanism | M5 (vault `2026-10-26 M1 scope.md:17`; spec S:680 says "permitted knowledge" only) | M5, written into S:680 | A-FV03 | no | Spec row does not name unit fog |
| A-FV02 | Fog of war | partial | S:162, S:634 (Baseline) | mechanism | M5 (vault) | M5 | A-FV03 | no | |
| A-FV03 | Sight radius | partial | S:162 (policy column) | policy | none | M5 | none | no | Stamp cost: 5,000 units x pi x 8^2 = 1.01 M cell writes/tick; 50,000 = 10.05 M (model 2) |
| A-FV04 | Server-side visibility | partial | S:338, S:634 (Baseline) | mechanism | M5 | M5, and on in every M7/M12 run | A-FV02 | yes: positions go to every slot unfiltered, `crates/turts_server/src/export.rs:111-118` | M1 figure 1 (10,396.5 B/tick, `m1.md:135`) is the no-fog upper bound. Filter = players x units tests: 40,000/tick medium, 400,000 stress |
| A-FV05 | Shared vision | partial | S:162 (policy) | policy | none | M5 | A-FV04 | no | Union of grids per team |
| A-FV09 | Terrain occlusion | partial | S:618 "counters do not implement terrain occlusion" | mechanism | none | M5 (after M4 heightfield) | M4 terrain | no | Line of sight per cell is far costlier than a stamp; no cost bound stated |
| A-FV10 | Last-seen structures | missing | "last seen", "ghost": 0 hits | mechanism | none | M5 | A-FV04 | no | Server memory = players x seen structures; replication on re-sight only |
| A-RO01 | Replay recording | partial | D41 S:64 "secondary"; S:364 retention open; S:710 | mechanism | none | M11 (shares the spectator feed) | S:364 checkpoints, needs decision on determinism | NOT TRACED | 20 min, 5,000 units: state stream 1.18 GB (positions only) vs command log 1.9 MB at 300 APM (~620x). Command log needs same-binary server determinism; S:239/S:243 do not require it. needs decision |
| A-RO02 | Replay controls, seek | partial | S:364 "seek ... remain open" | mechanism | none | M11 | A-RO01 | no | Seek cost = checkpoint interval x re-sim rate |
| A-RO05 | Replay version safety | partial | S:253 lockfile in saves, not replays | mechanism | none | M11 | D35 lockfile | no | |
| A-PE01 | Order responsiveness | partial | S:243 "immediate input acknowledgement" (Decided paragraph, baseline phrase); S:560 thresholds open | mechanism | none | M4 (measure), M7 (budget) | tick rate D18 | no figure in M1 (`m1.md` has no latency row) | Model at 20 Hz: RTT 50 ms gives 151 ms mean / 184 ms near-worst; RTT 100 ms gives 201 / 234 ms. Yardstick band 100 to 250 ms (`inventory-engine.md:514`) |
| A-PE02 | Stable frame rate | partial | S:650, S:660 "frame-rate targets remain open" | mechanism | M13 | M7 | AN11 | no | S:596 splits geometry/animation/shadows/effects/uploads; no HUD/overlay line |
| A-PE06 | Late-game slowdown | missing | "overrun", "degrad": 0 hits; S:612 cosmetic only | mechanism + policy | none | M7 | tick budget | NOT TRACED | No rule for a tick that exceeds its period (slow time, skip, shed) |
| A-PE07 | Latency hiding | partial | S:243; S:636 "no whole-world prediction" | mechanism | none | M4 | A-PE01 | no | Local marker and voice in the same frame cost nothing on the server |
| A-PF05 | Instant start | partial | S:630 "explicit pending states" | mechanism | none | M4 | path queue | NOT TRACED | Path queue wait adds to the A-PE01 chain; no bound |
| A-AN11 | Instancing and LOD | present | S:594-610 (Baseline) | mechanism | M13 | M13 | none | no | |
| performance-architect-1 | Input-to-feedback latency budget and figure | missing | "input latency", "click": 0 hits; T5 lag = interpolation only, 1.348 to 1.355 ticks p99 on localhost (`m1.md:156-161`, `crates/turts_server/tests/acceptance.rs:37-39`) | mechanism | none | M4 | relay with delay (`tests/net/relay.py`) | no | Add Open budgets: click-to-local-feedback (frames), click-to-ack (ms), click-to-first-visible-motion (ms p50/p99) at stated RTTs |
| performance-architect-2 | Client per-frame overlay budget (HUD, selection, bars, fog, minimap) | missing | S:596 list has no UI/overlay entry | mechanism | none | M7 | A-UI12, A-UI14 | no | Open budget: overlay CPU ms p99 per frame and texture upload bytes per frame at the medium workload |
| performance-architect-3 | M7/M12 scenarios hold no player-facing load | missing | S:650-658, S:682, S:687: no fog-on, no box select, no bars, no queues, no human command bursts | mechanism | M7, M12 | M7, M12 | A-FV04, A-OR24 | no | Add rows: fog filter on; 8 players each issue 200-unit orders at stated APM; health replication in the battle |
| performance-architect-4 | String ids and debug JSON in client hot loops | missing | `economy_ui.rs:733` parses `ContentId` per unit per pass; `debug.rs:206` rebuilds JSON of all units each frame (`m1.md:263-266`) | mechanism | none | M4 | none | yes, both paths | M1 browser frame figures include the debug hook; exclude it before any overlay budget is compared |

## Conflicts with the goal (Decided items)

- No Decided item blocks a CORE item in this area. CONFLICT=0.
- Tension, not conflict: D41 (S:64, Decided) makes replays "secondary". Replays are COMMON in five of six
  yardstick games (`inventory-engine.md:393`). The cheap replay (command log, ~1.9 MB per 20 min) needs a
  same-binary server determinism contract. S:239 and S:243 say full deterministic client sync is not
  required; they do not forbid server determinism. Needs decision: (a) state stream from the spectator
  feed (S:364), ~1.18 GB per 20 min at 5,000 units before compression; (b) command log plus checkpoints,
  which needs a determinism rule for the server; (c) no replay.
- D39 (S:62) forbids arbitrary caps. A selection cap (A-PC07) is mod policy, so no conflict. The engine still
  needs a technical bound on ids per command (S:342, Proposed).

## Lens answer

1. Section 13 budgets nothing a player touches. S:558 lists "browser compatibility, memory, startup payload,
   script overhead, and network traffic". S:596 lists geometry, animation, shadows, effects, uploads. S:662
   lists tick distribution, frame time, memory, download, bandwidth, cost. No line names input latency,
   selection time, overlay (HUD) frame cost, or fog cost. The only latency word in 13 is server "tail
   latency" (S:568) and "path request latency" (S:653).
2. M1 says nothing about input latency. `m1.md` has no latency row (`m1.md:133-142`). T5 measures how far
   the presented position lags the newest server tick: 1.348 to 1.355 ticks p99 on localhost, no relay
   (`m1.md:156-161`). That is one term of the input chain, not the chain.
3. Missing systems that change the M7 and M12 scenarios:
   - Fog (A-FV04): today every slot gets every position (`export.rs:111-118`). M7 bytes and CPU measured
     without fog are not the shipped cost. Fog adds per-player stamping (1.01 M cell writes/tick at 5,000
     units, r = 8) and a per-client filter (40,000 tests/tick).
   - Health replication (A-UI12): changed-only health in a battle adds 15 to 25 kB/tick per client at
     5,000 units, next to ~49 kB/tick of positions (9.86 B/unit, `m1.md:136`).
   - Local avoidance and group orders (A-GM04, A-OR24): the M1 tick p99 (228 us at 1,000 units) has neither.
     A 200-unit order is 200 path requests unless routes are shared.
   - Order queues (A-OR11): per-unit memory and wake-ups with no bound.
4. Open budgets the spec must add (names, not values):
   - click-to-local-feedback, frames;
   - click-to-ack, ms p50/p99, at stated RTT;
   - click-to-first-visible-motion, ms p50/p99, at stated RTT and tick rate;
   - box-select pass time, us p99, at 1,000 / 5,000 / 50,000 replica units;
   - client overlay CPU per frame (bars, icons, decals, waypoints, minimap, fog), ms p99, medium workload;
   - fog: server visibility CPU per tick (ms p99), fog texture upload bytes per frame, fog grid memory per player;
   - health/status replication bytes per unit per tick in the 8-viewer battle;
   - order queue length per unit and ids per command (S:342 "queued work", "affected entities");
   - replay bytes per match-minute and seek time;
   - tick-overrun behaviour (A-PE06).
5. Ten-times-cheaper levers: dense SoA scan with a type index (probe: 11x on the pick pass); fog stamp only
   for units that change cell; quantized health (one byte of fraction) sent on bucket change; replay as
   command log plus spectator checkpoints (~620x less storage), if the determinism decision allows it.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `python3 -I pa/sweep.py` (22 term groups over the spec) | Lens: S:554-668 has no player-facing budget | a hit for drag, health, order queue, latency of input in 13 | drag 0, health 0, order queue 0, decal 0, control group 0; latency only S:243, 560, 568, 586, 653 |
| second spec sweep (health, click, waypoint, overrun, degrad, ghost, occlu, determinis) | missing verdicts for A-UI12, A-OR13, A-PE06, A-FV10 | any hit | all 0 except occlu S:618, determinis S:239/243/364/574 |
| `rtk proxy grep` latency/input in `docs/measurements/m1.md` | M1 has an input-latency figure | a click-to-motion row | none; only T5 interpolation lag (`m1.md:156-161`) |
| read `export.rs:100-118`, grep visib/fog in server and sim | server filters positions by visibility | a filter on `positions` | no filter; economy only (`owns_balance`, `export.rs:125`) |
| read `economy_ui.rs:721-746` | client pick pass is O(N) with string parse | a spatial index or type index | O(N) over `HashMap`, `ContentId::parse` per unit |
| `rustc -O pa/sel.rs && ./sel` | M3 pick pattern is acceptable for box select at scale | M3 pattern within 2x of dense | 11x slower: 37.3 vs 3.3 us p50 (5,000); 436 vs 38 us (50,000); ~1,250 allocs/pass at 5,000 |
| `python3 -I pa/model.py` | latency, fog, bars, replay, minimap costs | latency under 100 ms at 20 Hz, RTT 100 | 201 ms mean, 234 ms near-worst |
| grep `HealthRecord` in `messages.rs` | health replication is compact | a quantized type | `current: u64` varint, per-unit record |
| grep fog in `vault/Decisions` | fog has a milestone | none | M5 (`2026-10-26 M1 scope.md:17`); spec S:680 does not say it |

## NOT TRACED

1. Wasm-to-native slowdown factor for the pick probe (the probe ran native on an i9-14900K, 28 threads).
2. Bevy UI cost of a 200-icon selection panel.
3. Whether M1-M3 units overlap (no avoidance) in `turts_sim`.
4. Order storage per unit in `turts_sim` (I saw `EconomyOrders`, `bootstrap.rs:38`, not its shape).
5. The 25 % "units that change cell per tick" fraction in the fog model is an assumption.
6. Replay command size (40 B) and APM figures are assumptions, not measured.
