# MP1 plan revision 2 re-check: architecture and sequencing (technical-director)

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=2
The repair closes most of my findings. Two gaps stay open, H1 and H2. Each needs a short text repair before L0-WIRE and L-TGT start. FINDINGS=6 RUNS=12 NOT_CHECKED=3

**Closure list**
- **H1: PARTIAL** (P:683-700). The store, the getters, the "no clone" rule and the contract test exist. But "commits both or neither" is defined per message (P:692). That does not cover the resync path:
    - `apply_snapshot` (replica.rs:216-276) stages the snapshot, replays N retained deltas, and commits once (:268).
    - When a later batch fails, it returns `ResyncRequested` and keeps the old display (:260-266).
    - A per-message store apply would then put the store ahead of the replica.
    - `IgnoredStale` (:221) and `Discarded` (:285) are not tested.
    - The cost of the store apply on a Snapshot (a full replacement) is not stated.
- **H2: PARTIAL.**
    - Done: the scheduler models stacked edges (mp1_plan_check.py:127-136). `L-PWR` no longer touches combat.rs. `L-TGT|A3` is now ordered by a stacked edge.
    - My rerun on revision 2 finds 4 combat.rs pairs that still build in parallel with no input edge: A2c|A3, A2c|L-DMG, A2c|L-FX-WIRE, A3|L-FX-WIRE.
    - Only A2c|A3 share one function (`run_combat`, :154 and :110-117), and the slice plan orders that pair (SP:942).
    - L-DMG (:258-376) and L-FX-WIRE (:393-402) edit `weapon_parameters` and `fire`, not `run_combat`.
    - Open: the stacked edge across branches (new H-A below).
- **H3: PARTIAL** (P:670-681). The writer function, the scan and the single-move case exist. The limits of the scan are in new M-A.
- **M1: CLOSED** (P:435-445; task `L0-EXIT2` P:1168; stage-2 tasks take it as input, P:212-227).
- **M2: CLOSED** (P:88).
- **M3: CLOSED** (`progress::value_at` plant P:537; the getters P:693; the grep P:920).
- **M4: CLOSED** (P:649, P:653-656; D:902 matches).
- **L1: CLOSED** (P:759, P:850, P:803).

**New findings**

**H-A (High), P:85 and P:836: a stacked edge across the slice and MP1 branches is not defined.**
- L-TGT "branches from the A3 branch head", but that branch has none of its MP1 inputs (L-DMG, L-MATCH-SIM, lane 0).
- A3 then "rebases when X merges" (P:85), so L-TGT would carry old copies of the A3 commits into combat.rs:90-117. That is the H2 risk again.
- The slice plan has no amendment for A3^ on A4 (grep "stack" in SP: 0 hits; SP:165 still says A3 needs A4 merged). The 60-run path assumes A3 starts at offset 17.
- Repair text: "A cross stack (`A3^` in an MP1 task) branches from `mp1/integration` and merges `pre-mp1/a3-grids` at the end of the A3 build. A3 merges into `pre-mp1/integration` with a merge commit, not a rebase. Root adds the A3-on-A4 stack to the SP binding amendments."

**H-B (High), P:692 and P:695: the store must commit with the whole staged chain.**
- Repair text: "The store change set is staged with the candidate through `stage_snapshot` and each `stage_delta` of the replay, and `commit` applies both. The contract test adds three cases, each with the store unchanged or equal to the candidate tick:
    1. a resync where a retained delta fails;
    2. a stale Snapshot;
    3. a resync with 2 replayed deltas.
  The Snapshot cost is one store replacement."
- Also apply this to E11 (D:1379).

**M-A, P:671: the syn scan sees names only.**
- Direct writes use `get_mut::<UnitOrders>()` (22 sites in orders.rs), `remove::<UnitOrders>()` (orders.rs:3261), and `insert`.
- The scan does not see a write through an inferred `get_mut()`, a helper that takes `&mut Order`, or an `insert(var)`. It also does not see a removal or despawn that must send `orders_cleared`.
- Repair: list those patterns in the scan. Add a `test-probes` system in `Collect` that compares `Changed<UnitOrders>` and `RemovedComponents<UnitOrders>` with the change list and panics on a difference.
- The slice builders do not get the rule (SP has no `set_unit_orders`). P:681 makes the MP1 owner repair it at each forward merge, so a direct write goes red there. That works if Root counts about 1 run for each of A2b1, A2b2 and A2c.

**M-B, P:715-716: the pure-move proof cannot see a change of control flow.**
- `draw_world_visuals` has an early `return` (visuals.rs:343) and accumulators that the concerns share (`desired` :437, `ring_metadata` :438, `health_fractions` :621).
- A `return` that moves into a concern function stops only that function. The joined-body `cmp` still shows equal text.
- Repair: a rule that each early exit stays in the caller, plus a behaviour oracle before and after (the spawned mesh set for fixed scenes; NOT TRACED that a headless frame test exists).
- Ruling: make the splits a separate task `L0-SPLIT`. Reasons:
    1. A pure move gets its own review, as S2, S3 and S45 did.
    2. L0-CORE falls from 5 runs (range 4-7) to 3.
    3. L0-SPLIT can start now (S45 is merged), not at offset 8.
    4. It saves 1 run (my run: 59).

**L-A, D:2294 (E6):** the design names two kinds of edge. The plan has three (P:83-87). Add the stacked edge to D.

**Sequencing proposal (scheduler rerun on copies): 57 runs, C1 at 37, PT3 at 56 (-3).** Three edits:
1. `L0-SPLIT` (2 runs, now), with L0-CORE at 3 runs.
2. `L-PWR` takes inputs L0-EXIT and L-DMG, and merges after L-CON1. This needs one more L0-SIM stub, `power::rate_factor`, for the producer and constructor rate (the `weapon_enabled` pattern).
3. `L-OBS` stacks on A3 (`A3^`) in place of the plain input A3. This needs H-A.

Each edit alone gives 59. Edits 2 and 3 give 58. With all three, the path goes back to A3 → L-TGT with 0 slack.

**H24 proposal (one erratum)**
- `WorldStyle` gets `#[serde(default)] pub unit_labels: Option<LabelStyle>`.
- `LabelStyle { show: LabelShow, #[serde(default)] own_only: bool, colour: String, #[serde(default)] font: Option<String>, size: u16 }`, with `#[serde(deny_unknown_fields)]`.
- `LabelShow { Always, Selected }`. Drop `Never`, because `None` already means "no labels" (one meaning for each value).
- The fields follow `WorldTooltipStyle` (interface.rs:359-) and `Text` (:51-55).
- Checks: `colour` is `#RRGGBB`; `size` is 8 to 32; the font exists. Defaults: `own_only` false, `font` None (the engine font).
- RON: `unit_labels: Some((show: Selected, own_only: false, colour: "#ffffff", font: None, size: 14)),`

**Design 6.1 errata:** E11 needs H-B. E12 (D:902) and E14 (D:1386-1387) are consistent with the plan and with section 9. E6 needs L-A.

**What I ran** (Python with `-I -B`; `git status --porcelain` was clean after each run)
- The plan check script on revision 2: exit 0, path 60.
- `td_sched2.py` (build windows against input and stacked edges): 75 pairs, of which 4 share combat.rs.
- Five edited copies of the plan through the scheduler: 59, 59, 59, 58, 57.
- Read replica.rs :216-378, the `UnitOrders` struct and its write idioms, the `draw_world_visuals` exits and locals, the core_adapter Effect arms, `WorldStyle` and the `forge`/`acme` hud.ron.
- grep of the slice plan for the stacked edge and `set_unit_orders` (0 hits each).
- Scripts and copies: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-td/`.

**Not checked:** a headless visuals oracle; whether A3 bit-equality holds over A2c; the store's Snapshot cost in numbers.

**What I changed:** none in the repository. I did not update my memory note.
