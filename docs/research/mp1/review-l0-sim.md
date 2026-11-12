# Review of L0-SIM (performance-architect)

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=FAIL HIGH=2
(The behaviour is the same as before, and every gate passes. Two Highs: the set order that lane 0 freezes contradicts the design. One High has a 1-edit repair; the other needs a decision.)

**1. No behaviour change: confirmed.**
- Trace sha256 values: base d4c73ebe…, haul 805309b2…, acme 19dcf9a3…, cedar 0db10e01… All four are equal to the recorded values (`mc-evidence/mp1-l0sim-review-item1/command.log`).
- pack tests: 43+26+1+1+7 passed. turts_sim: 356 passed, 2 ignored. Both ignored tests are timing tests: `unit_index::tests::lookup_cost` and `record_spread_avoidance_and_combat_ticks_at_one_and_five_thousand_units` ("Task22 timing record"). turts_server: 214 passed, 30 ignored (process tests). turts_audit: 67 passed.
- `probe_neutral`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`. Both are equal to the recorded values. Stream sha256 is bcbf72c2…, equal.
- Set order. Base (`plugin.rs:93-108` @51f1223): Advance, Commands | Orders, Movement, Gather, Extract, Transport, Inserters, Production, Combat | Collect. Head (`plugin.rs:110-136`): Advance, Commands | **Supply, Income**, Orders, Movement, Gather, Extract, Transport, Inserters, **Construction**, Production, **Acquire**, Combat, **Outcome** | Collect.
- The relative order of the old systems is identical. The 5 new systems are empty.

**2. Index kept by hooks.**
- Bevy 0.20 has on_add, on_insert, on_discard, on_remove and on_despawn (`lifecycle.rs:189-250`). On a despawn, the order is despawn, then discard, then remove. on_discard also fires on a replace.
- The index is never iterated outside tests: the map field is private (`unit_index.rs:23-25`; grep found no `.iter`).
- I wrote 4 scratch tests:
    - (a) Two holders of one id, newer despawned first: index `[]`, scan `[(4,e)]`. **Stale.**
    - (b) `get_mut::<UnitId>` mutation: index `[(5)]`, scan `[(6)]`. **Stale.**
    - (c) Bare World, spawn+despawn in one CommandQueue: OK. The lazy path is safe.
    - (d) Same id, despawn and spawn in one batch, both orders: OK.

**3. Lookup cost and remaining scans.**
- `lookup_cost` in the lane (release, loaded host): scan 9,378 ns at 1,000 units and 42,661 ns at 5,000. Map lookup 23 and 25 ns. The builder recorded 8,062 / 29,937 / 12-13 ns, the same order.
- Scratch counter on the server with synthetic units (100 ticks): 20 full scans per tick at both N=400 and N=1,000 (O(n) each, not per unit). Index lookups: 0.
- m2 trace (1,600 ticks): 342 lookups. Per-unit, per-tick scans: `gather.rs:1377` find_unit, 3,116 calls (about 0.7 per gatherer per tick). By code, `extractor.rs:69→143`, `inserter.rs:173/186→208` and `machine.rs:247→271` are each called once for each such unit per tick. `production.rs:442` (try_output) does one scan and one allocation of N circles for each blocked producer per tick.
- Per event: `transaction_m3.rs:524` (each death). Per command: `command.rs:239`, `command_m3.rs:20`.
- Cost model (world about 2N entities, 20 Hz, 50 ms budget): `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l0-sim-review/cost_table.txt`

| N | 20 O(n) scans/tick | gather find_unit, N/10 gatherers, 0.7 calls | same, 10 calls (D:2226) | 1% deaths/tick |
|---|---|---|---|---|
| 400 | 0.15 ms | 0.21 ms | 3.0 ms | 0.03 ms |
| 1,000 | 0.38 ms | 1.3 ms | 18.8 ms | 0.19 ms |
| 5,000 | 1.7 ms | 30 ms | 427 ms | 4.3 ms |

- The gather find_unit must not wait for a later owner. Design 9 (D:2223, site 3) puts it in lane 0.

**4. Gate at process level** (base.tar, release).
- Head, `Immediate`: `--run-ticks 60` exited with 0 after 3.02 s. Stats: ticks 60.
- Scratch copy with `main.rs:76` removed: ticks 0 after 3 s, and nothing on stderr except the boot lines. **Silent.** `--run-ticks 5` did not exit (killed at 10 s, exit 124). In this build that is unreachable, because the boot always inserts `Immediate`. In a lobby, design 6.7.5 wants it.
- The server admission systems (`session.rs:120`, `command_script.rs:45`) are outside `SimSet`, so they still run before the start.

**5. Slot order** (the design status of each pair is "gated design").
- Supply first: **contradicts 6.2** (D:522-523): "A removal in tick t takes effect at the computation of tick t+1." `CommandKind::Remove` (`command.rs:265`) applies in `drain_commands`, which runs in SimSet::Commands in FixedPreUpdate, before Supply. A removal by command therefore counts at tick t. A station destroyed in Combat counts at t+1, as designed.
- Construction before Production: **contradicts 6.4**: "Jobs pay in ascending unit id of their owner (producer or site), then in queue order." Sites advance in Construction (`construction.rs:4-6`), so every site pays before every producer.
- Income before Orders: not settled. A PayOnQueue charge happens in Commands (`production.rs:131-138`), before Income, so `income.rs:3-4` is false for that path.
- Acquire before Combat: satisfies 6.3 ("one grid … built once for each tick", from the positions after movement).
- Outcome after Combat: satisfies 6.7.8 ("after all removals and after the death path, before Collect"). The builder's test covers it.
- Not settled: repair has no slot (heal before or after damage); the same-tick start of a building completed in Construction; the spawn_order of a unit spawned after Orders.
- Errata text:
    - E-a: "The supply computation runs in each tick after the tick advance and before the commands of the tick."
    - E-b: "Sites pay in Construction before producers pay in Production; inside each set, by ascending unit id, then queue order." Or: one payment pass over both.
    - E-c: "Income of tick t is paid after the commands of tick t."
    - E-d: "A site completed in tick t enables its capabilities in tick t; its supply counts from t+1."
    - E-e: "Repair runs in set ___." (needs a decision)

**6. Power stubs.**
- `weapon_enabled` is called at targeting and at fire (`combat.rs:199-202`, `:224/243`). `rate_factor` is called in production only. The construction call is left to L-CON1, as in plan :692. No repair call, which is correct: repair has no when_short (6.2).
- `steps_at_rate` (`progress.rs:84-91`) is **a second rate rule**. My model (`rate_model.py`) gives:
    - rate 1/2, duration 3: 5 or 6 ticks, by the start phase.
    - rate 1/3, duration 10: 28 to 30 ticks.
    - ticks_at_rate and the design Q model both give 6 and 30.
    - mixed 1/1 then 1/2: 14 ticks, where the design gives 15.
- The "tick 0 counts as 1" clamp is a test hack.

**7. My plants** (in a scratch copy).
- P1: `on_discard`→`on_remove`. Red (`the_index_equals_a_full_scan…`).
- P2: Collect without `run_if`. Red (`with_no_config…`).
- Restored with `cmp`.

**8. T7 headed.**
- First call, `mp1-l0sim-review-t7`: exit 1, "Cannot find module 'playwright-core'". This was a lane setup fault (no `npm ci`), not a red. wait_s 818.8, hold_s 106.6.
- I ran `npm ci` (node_modules is excluded from git).
- Second call, `mp1-l0sim-review-t7b`: **exit 0**, `PASS t7-oracle.js`, 43 frames, lag_p99 1.445 ticks. lock-receipt: wait_s 412.2, hold_s 99.4, yields 0, survivors none, source_unchanged true.

**9. Gates** (`mp1-l0sim-review-gates`): all exit 0.
- fmt, clippy -D warnings, turts_checks (21+7+2+14), wasm32 check, boundary. The stream sha256 is equal.
- Ratchet, scratch: a u128 charge copy in `power.rs` was red (`u128_div`); `x / 2` stayed green.

**10. Merge.**
- `merge-tree origin/mp1/integration 451f7b6`: clean (tree b730bf0, exit 0). Integration changes no crate.
- On the merge tree, `mp1_plants.py --files … --list` exits with 2 because `l0-split.json` is in the old format. Integration alone gives the same error, so this is not L0-SIM.
- With `--plants-dir` holding only l0-sim.json, `--all --list` accepts all 17 records (exit 0).
- No trial merge in the worktree. The tree is clean at 451f7b6.

**Findings**
- **H1** `plugin.rs:110-114`, `power.rs:20-21`: Supply runs after the command removals (contradicts 6.2). Repair: chain `Advance → Supply → Commands` in FixedPreUpdate and update TICK_ORDER, or add erratum E-a.
- **H2** `plugin.rs:126-127`, `construction.rs:4-6`: sites pay before every producer (contradicts 6.4 "Order inside a tick"). Needs decision: one payment pass, or erratum E-b.
- **M1** `unit_index.rs:53-66`: a duplicate UnitId is silent last-writer-wins, and loses the older holder. Repair: panic in `on_unit_id_insert` when the entry holds another entity, and update the `two_holders` test.
- **M2** `world.rs:14-17`: UnitId is mutable, so `get_mut` bypasses the hooks. Repair: add `immutable` (it compiles in sim, server, pack, audit and checks; I checked).
- **M3** `progress.rs:84`, `production.rs:304-311`: `steps_at_rate` is a second rate rule. Repair: count progress in Q units with the existing `rate_step` (D 6.4) and delete `steps_at_rate`.
- **M4** `gather.rs:1377`, `extractor.rs:143`, `inserter.rs:208`, `machine.rs:271`: per-unit-per-tick scans; gather is in the lane-0 list. Repair: swap the 4 bodies to `unit_entity` (about 12 lines).
- **M5** plan :687, "Footprint and supply resource ids to indices": not built, and the report does not say so.
- **L1** A server with no config is silent. Log one line (L-ROOM0).
- **L2** `production.rs:442`: one scan and one Vec for each blocked producer per tick (P9 lane).
- **L3** `income.rs:3-4`: the comment is false for PayOnQueue.
- **Simpler and cheaper:** a dense `Vec<Option<Entity>>` by id in place of the SipHash map, about 10 times cheaper than 23 ns.

**Not from this task:** at 5,000 synthetic units, one tick takes 12.0 s at the head and 13.2 s at the base. perf: 41% in `UniformGrid::within`, 24% in sorts inside `within`, plus `spread_destinations` (MC movement). This is a D58 risk.

**Still owed:** acceptance.sh (31 ignored process tests, plan step 3), NOT RUN. The scratch copies are in `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l0-sim-review/`.
