# MP1 gate 1: performance lens

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS FINDINGS=12 HIGH=4 RUNS=9 NOT_CHECKED=5

Design: `/home/julien/Projects/TuRTS-mp1/docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md`, revision 3, status DRAFT. I ran native release probes on a host with load 43 to 51 (other gate agents were building). The min values are the least-loaded values. My moving tick at 1,000 units was 6.87 ms against 3.167 ms in mc.md:58, so my absolute values are about 2x high. The ratios inside one run still hold. Tick rate assumed: 20 Hz. It is Open (D18).

## Findings

**High**

H1. The client rebuilds the whole debug JSON every frame, and nothing gates it.
- Where: `turts_client/src/lib.rs:147` (`Last`, `rebuild_debug_state`). The web client then copies it each frame (`turts_client_web/src/lib.rs:168,372`).
- Measured (native, min/p50): 400 units 4.4/5.1 ms and 148 KB; 1,000 units 11.3/13.0 ms; 5,000 units 96.8/128.8 ms and 1.78 MB per frame. VIS2 p50 is 133.3 ms. This is the strongest lead for the VIS2 cost. It is NOT TRACED in the browser.
- The JSON includes `health_trace`, which is never pruned (H4/M2), and SMT1 and AUD2 add fields.
- Text for 8.5: "The debug state is built only when the harness asks for it, at most 4 Hz. VIS3 measures the frame time with the debug state off and on."

H2. The UnitId-to-Entity index is not in lane 0, and design 6.3:153 names cold sites.
- `orders.rs:3056,3085` run once per command, not once per tick.
- Scan cost (bevy_ecs 0.20): one find by id costs 5.0 µs at 1,000 entities and 25.9 µs at 5,000. A HashMap lookup costs 0.026 µs, so the index is about 1,000x cheaper per lookup.
- The five hot sites:
    1. `economy/production.rs:282`: one O(n) scan per active producer per tick. MP1 sites and two queues per command centre multiply it. At 500 sites and 5,000 units: about 13 ms per tick.
    2. `transaction_m3.rs:635` and `:524`: two O(n) scans for each removal. DES1 extends this path.
    3. `gather.rs:1206` `find_unit`: about 10 calls per gatherer per tick.
    4. `combat.rs:94`: a full scan, 3 BTreeMaps and a grid rebuild each tick.
    5. `orders.rs:703`: a full scan and BTreeMaps each tick.
- Also `export.rs:299` `owner_of`: a linear scan for each record and slot. Measured 4.1 to 7.2 ms per tick for 2 slots at 5,000 units in combat. At 8 slots that is about 16 to 29 ms. `view.units` is sorted (`world.rs:61`), so a binary search fixes it.
- Upkeep: one insert per spawn and one remove per removal (about 30 ns each), about 32 B per unit. Use it only for lookup. Never iterate it, so determinism holds.
- Text for 10, lane 0: "Add the UnitId index and use it at the sites above, before any feature lane adds a lookup."

H3. Idle acquisition costs too much at scale. It runs every tick for every armed unit and evaluates friends as candidates.
- `choose_target` (`combat.rs:201`) runs before the cooldown check (`:229`).
- Extra cost per armed idle unit per tick: range 4: 2.3 to 3.9 µs; range 9: 6.8 to 12.7 µs; range 16: 12 to 34 µs.
- MP1 (300 armed, range 9): 2 to 4 ms per tick. That is acceptable.
- 5,000 units (2,500 armed), range 9: idle tick 45.2 ms against 13.5 ms with no weapons. That is near the 50 ms tick.
- Probe with 400 armed at range 9: no friend inside the disc gives 2.4 ms; dense friends give 5.4 ms. With friends removed, the query costs about 0.
- Text for 6.3: "Candidates come from a grid per relation (enemies only). A query runs only when the weapon is ready and has no valid target, and at most every K ticks, staggered by unit id. TGT1 asserts acquisition within K ticks." This is at least 10x cheaper.

H4. Slice part A3 carries load for MP1, but the slice runs it last and calls it "performance only" (pre-MP1 slice :36, :47).
- One static unit of radius 2.5 (the command centre) raised 5,000 movers from 42.7 to 195.5 ms (4.6x). At 1,000 movers: 6.9 to 11.0 ms.
- Text: "A3 is an entry condition for any MP1 content with a footprint above the mobile radius."

**Medium**

M1. Wire.
- The full `unit_orders` table is 77% of the moving delta at 1,000 units: 42,765 B per tick, about 855 KB/s per client. The design already plans to send changes only.
- `WeaponRecord.cooldown` changes every tick, so there are 500 records per tick at 1,000 units in combat. With `unit_orders`, that is about 4.9 KB per tick.
- `Fired` with a string weapon id costs 18.8 B per shot: 470 B per tick, 9.4 KB/s at 500 shots/s. With an index it costs 4.8 B per shot.
- Text for 6.8: "`Fired{unit, target}`, no string. Weapon, job and site records carry `ready_tick` or `started_tick` and a rate, and they are sent on change only." Fog (M5) then filters O(shots) per viewer.

M2. The client replica cost grows over a match.
- Each delta clones the full replica three times (`replica.rs:432,451,1090`).
- `replace_keyed` is O(changes x records) (`:1029-1048`).
- `health_trace` is never pruned (`:1182`). At 400 units, `apply_delta` rose from 0.28 ms to 0.98 ms with 120k entries, and to 2.29 ms with 400k entries.
- Text: "Prune health_trace to a window. MP1 adds no field to `Candidate`."

M3. The charge rule needs a decision: per-tick charge or charge in steps.
- Design 6.4:157 says each tick charges the difference. TD input :199-200 says charges come in progress steps and the step size is mod data.
- My model ran 20,000 cases: the sum equals the cost and no step is negative.
- u64 overflows in `cost x e` above 7.7e14 for a 10-minute job at half rate. Use a u128 intermediate.
- Per-tick charging writes one ledger event per job per tick: 400 for a 20-second tank.

M4. String parsing in per-tick and per-frame paths.
- `combat.rs:265` parses a ContentId for each weapon each tick.
- `visuals.rs:648` parses one for each unit each frame (hover).
- Text: "Damage type, armour class, acquire and footprint are resolved to indices at install time. The CON2 ghost reads a footprint list that is rebuilt per delta, not per frame."

M5. An idle tick already costs 3.0 ms at 1,000 units and 13.5 ms at 5,000 (full view and diff in `plugin.rs:174-245`).
- Text: "A new record kind enters `CommittedTick` from its writer's change list."

**Low**

L1. A pool of 256 effects is enough. A 60-unit fight gives about 50 shots/s and about 56 live instances. 400 riflemen give about 230. FX1 needs a red for "the pool grows, or one entity per shot".

L2. A floor for VIS3 in the play test (Proposed, not a spec budget): p95 at or below 50 ms and p50 at or below 33 ms on the play-test machine. Above that, PT3 does not start.

L3. Memory to record. Textures: 512² RGBA8 is 1.33 MiB with mips; 1024² is 5.33 MiB. About 16 MiB in total, doubled if the main-world copy stays (RE:31). With the 3 to 8 MB of GLB bytes, the wasm heap can grow by about 35 MiB, and it never shrinks. Before any art exists, measure placeholder models at the triangle cap (about 0.9 M triangles at 400 units).

## Three measurements for `docs/measurements/mp1.md`

1. Server tick p50/p99 over 200 or more ticks, for an MP1 mixed scene at 2x200 units, 1,000 units with 4 slots, and 5,000 units with 8 slots. Record the idle tick separately.
2. Postcard bytes per tick per client, split by record kind, and the export time in ms for all clients.
3. Native `apply_delta` p50/p99, plus browser frame p50/p95 and the wasm memory high-water after 10 minutes at 400 units, with the debug state on and off.

## What I ran

All logs: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/pa/all-runs.log`. Each probe below exited 0.

| Command | Claim tested | Refute if | Observed |
| --- | --- | --- | --- |
| `sim` probe | 6.3 acquisition cost; A3 | range 9 costs the same as range 4 | 45.2 vs 23.2 ms at 5,000 |
| sparse/dense | per-relation grid | dense = sparse | 5.4 vs 2.4 ms |
| `wire` probe | 6.8 bytes | `Fired` dominates | the weapon and order records dominate |
| `pa_trace` | replica growth | flat | 0.28 to 2.29 ms |
| `pa_debug_json` | VIS3 can be read | below 1 ms | 4.4 ms at 400 units |
| `scan` | 6.3 index | below 2x | about 1,000x |
| `charge.py` | 6.4:157 | sum not equal to cost | 0 bad |
| greps | owner_of, schedule, trace | gated or pruned | neither |

A compile error happened once and was not counted as a red: alsa was missing. I used a link-only shim in the scratchpad. One probe file was in the worktree for one run and is removed. `git status --porcelain` is empty.

## Not checked

- The browser frame and memory (no headed run).
- A GPU on an integrated chip.
- How lightyear frames the messages beyond postcard (`lightyear_serde-0.31.0/Cargo.toml:78`).
- The victory, power, placement and repair costs are arithmetic estimates only. The code does not exist yet.
- Medium-confidence numbers come from a loaded host.

## What I changed

None.

## Open questions for the orchestrator

- M3 needs a decision: per-tick charge or progress steps.
- Do the debug-state fix (H1) and the health_trace pruning (M2) belong in MP1 lane 0, or in the slice?
