# MP1 input: technical-director (pre-MP1 slice, engine mechanisms, file splits)

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=INPUT

The brief asks for five design parts, so this reply is longer than the 600-word rule. I put the raw evidence in the scratchpad (paths at the end). Work tree: /home/julien/Projects/TuRTS-mp1 at 8d6902c. I did not read the studio-agent design or the full spec sections 4, 5.2, 6.1 to 6.3 again in this run. My decision references come from CLAUDE.md and the vault note.

# A. Pre-MP1 slice

## A1. `turts_audit`

**What the validator is today.** It is a full state-replay model, not a ledger check:
- `validate_tick_state` builds `expected = previous` and requires equality, or it calls `validate_movement_transition` (economylog.rs:8064-8082, :7845).
- It replays orders (:2238, :2694), dispatch (:3527-5277, with `validate_pending_movement_dispatch` at :4282), plain moves (:1182-1400, :3849-4200, :6866-7170) and gather (:5448-5760, :7628-7845).
- A ledger oracle already exists: `crates/turts_pack/tests/common/conservation.rs` (1,088 lines). It checks BAL-ore, BAL-shell, RES, SHOT, DMG, REFUND and DANGLING (`stock` :185, `balance_expected` :258, `check_operation` :370).

**Crate boundary.**
- `turts_audit` (lib) depends on `turts_sim` (it reads the audit types), `turts_net_protocol` (`CommandRejectReason` in the Admission record, economylog.rs:14,43), `turts_mod_api`, serde and serde_json.
- Its users: `turts_server` as a normal dependency (the sink writes `EconomyLogRecord`), and the dev-dependencies of the `turts_server` and `turts_pack` tests.
- New boundary.sh rows:
    - `forbid turts_audit $HOST "" bevy_render bevy_winit bevy_pbr turts_client turts_client_web turts_server turts_pack turts_mod_runtime turts_net_lightyear`.
    - Add `turts_audit` to the `turts_sim` row (:33).
    - Add `turts_audit` to the `turts_client`, `turts_client_web` and `turts_interaction` rows, host and wasm32 (:38-46).
    - Each new row needs a fixture case that goes red. NOT TRACED: how `tests/boundary-fixture` selects its rows.
- Alternative I did not take: a bevy-free log-format crate that `turts_sim` depends on. It means moving the view types out of `turts_sim`. Look at it again with the M11 replay format.

**Files, one for each domain:**
- `lib.rs`: record shape (exactly one Initial, first; :371-381) and dispatch to the domains.
- `format.rs`: `EconomyLogRecord` (:17-49) and `read_jsonl` (:345-367). The writer and the sink (:128-343) stay in `turts_server/src/economylog.rs`, about 350 lines.
- `clock.rs`: clock and operation continuity (:7575, :7597, :7608), operation-id order and event ordinals (the ordinal part of :1815).
- `slots.rs`: Claim records; an Admission record has `before == after`.
- `balances.rs`: slot balances match the sum of the Charged, Refunded and Converted events.
- `inventories.rs`: unit inventories, output inventories and cargo. Rule: "a refused command or operation changes no account". This replaces :2349.
- `reservations.rs`: claim open and close, no leak, token match, endpoint exists (RES, DANGLING).
- `production.rs`: refund = ratio x charged, paid once (the logic of `refund_amount` :6659).
- `industry.rs`: belts, inserters, extractors, machines and nodes.
- `combat.rs`: the ammo debit for each shot (SHOT). Damage is out, because health is not a ledger.

Seed the domains from conservation.rs. Then `turts_pack` imports `turts_audit`, so one oracle remains, not two.

**What the slice keeps and what it drops.**
- Keep: record shape, clock, slots and admission, conservation for each resource in each record, the reservation lifecycle, refund arithmetic, and the no-change rule for refusals.
- Drop:
    - refusal prediction: :1402, :1762, :1823, :1943 and :2084;
    - order and automatic-order replay: :2238 and :2694;
    - queued and pending dispatch: :3527-5277;
    - `start_next_audited_order`: :5278;
    - gather order state: :5448-5760;
    - set-enabled: :5756;
    - the non-ledger parts of remove, produce and cancel: :5876, :6166 and :6526;
    - legacy move: :6866;
    - command phase: :7171;
    - attack helpers: :7475-7575;
    - gather goal and movement: :7628-8064.

**What `AuditState` becomes.** Do not remove fields from `AuditState` (views.rs:374) in A1. The frozen stream compares `op.before == op.after` over the full `AuditState` (m3_m2_stream.rs:119). A removed field can flip that frozen value.
- Add `LedgerAudit` in `turts_sim/src/economy/ledger_view.rs`. One function converts `AuditState` to `LedgerAudit`. The log writes only `LedgerAudit`.
- `LedgerAudit` holds: accounts, inventories, outputs, belts, inserters, extractors, weapon ammo, nodes, ledger_claims, queues and jobs, research jobs, catalog prices and ratios, units (id, type, owner), removed_units, clock, ordinals, pending events, claims and allocators.
- `AuditState` stays as the in-process test snapshot.
- Result: a new sim structure can enter the log format only through an explicit edit of `LedgerAudit`.

**Faults that lose their red, and what replaces each one.** I traced the source of each expected-failure string (scratchpad `failsrc.txt`).
- E8-mixed-cost-partial-commit (tests/m2/refutations.json:768): its red today is economylog.rs:2349. Re-pin it to the message of the new inventories/balances rule. The same plant also stays red in E6-mixed-cost-partial-commit, through accounting.rs.
- The other 40 M2 cases and all 17 M3 cases do not change. Their reds come from accounting.rs, conservation.rs, `turts_server/tests/economy.rs` and `m2-oracle.js`.
- Validator self-tests in `turts_server/tests/economy.rs`:
    - :486-564 (a position moved by 1 ulp): delete it. GM40, GMA, GMU and the movement tests own positions.
    - :441 (tick + 1): keep it, now under clock.rs.
    - :457, :468, :3293, :3319, :3345: keep them under the ledger domains.
- EL1 (design :1255) has three faults:
    - "movement input missing" becomes void.
    - "a write to `Position` or `UnitOrders` outside the movement function and `capture_operation`": a new `turts_checks` scan allows those writers only in the `movement/` and `orders/` files. It needs a planted fixture that goes red.
    - "the validator of today" becomes void.
    - Rewrite the EL1 row: the ledger validator accepts each tick, and the run records the log bytes.
- mc_orders.rs :469, :574, :694, :824, :1089, :1094 ("strictly replays") stay green but prove less. Each must assert its order outcome from the order log. NOT TRACED: whether they already do.

**Steps, green after each one:**
1. Pure move. Create the crate with `format.rs` plus `replay.rs`, which holds the old validator unchanged (economylog.rs:345-8083, and the tests :8084-19082 into the crate's tests). Switch the three `#[path]` includes (economy.rs:9, el1.rs:6, mc_orders.rs:3) to `turts_audit::`. Add the boundary rows.
2. Add `LedgerAudit` and the domain files. Run both validators in every caller. Add one planted fault for each domain. Re-pin E8.
3. Switch the callers to the ledger validator. Delete `replay.rs`. Apply the fault list above. The sink writes `LedgerAudit`.
4. Replace `common/conservation.rs` with `turts_audit`. Run `scripts/m3-refutations.sh`: C1 to C17 must stay red.

## A2. One movement model

**Code that changes:**
- The single Point Move takes the deferred path (orders.rs:2772-2820), not the plain branches (:2822-2900).
- `CommandKind::MoveTo` maps to an envelope with a pool of one (`queue_legacy_move_to_envelope` already exists at :551).
- Delete:
    - `PlainMoveProgress` (movement.rs:58);
    - `step_plain_progress` (:671-905) and `make_plain_progress` (:925);
    - the plain spread context (:1409-1430, :1500-1511, :1885-1960);
    - the plain reservations and identities (orders.rs:96-178, :1189-1358, :3101-3286, :3614-3660);
    - `AuditState.plain_movement_progress`.
- `PlainGatherGoal` (movement.rs:63,79-82) moves into `MoveProgress`.

**What changes for one unit:**
- **Destination.** A pool of one takes P when `free_slot_grid(P)` passes (movement.rs:282-292). Otherwise it takes the nearest free hex slot, at dispatch time. Today a plain move displaces only when it finds P occupied (movement.rs:1885-1890). A lone unit gets the same result. The result differs when P is reserved by another moving or resting unit.
- **Path.** Both models use the same straight `step` when no neighbour is near (movement.rs:500-515; pooled :2161-2169). For a lone unit I expect bit-identical positions.
- **Arrival tick and final position.** Both models snap to the target (plain :1936-1951; pooled :2092-2106). I expect them to be identical.
- **After arrival.** A pooled unit keeps `MoveProgress{Resting}` and a pool entry. Its slot becomes a reserved cell for later envelopes (design :613-616).
- **Audit.** The pooled path writes order operations (orders.rs:2874).
- **Load.** A single move now counts in the 2,048-units-per-tick spreading cap, so under load it can wait a tick.

**Expected stream change.** The stream has no positions (m3_m2_stream.rs:31-36).
- The acme scout (MoveTo at :263) gains operations with `event_kinds []` at the accept tick and at the arrival tick.
- base, haul and cedar (Gather at :331) should not change.
- Any other difference is a defect until someone explains it.

**The one re-freeze:**
1. Before A2, `qa-lead` adds a test-only trace that is not frozen: for each tick and unit, the position bits, target and activity; for each operation, the unit and the phase. Run it on the pre-A2 build and log its sha256.
2. The builder lands A2 and does not touch the fixture.
3. `qa-lead` runs a classifier (Python standard library). It checks:
    - P1: every position is bitwise equal for the 4 packages over 400 ticks.
    - P2: every economy event kind falls on the same tick.
    - P3: every changed operation has `event_kinds []`, belongs to the acme scout, and falls on the accept tick or the arrival tick.
    - An independent hand calculation: arrival tick = ceil(|P - p0| / (speed x dt)).
4. Classifier self-test: one flipped position bit must give red, and one added Stored event must give red.
5. The technical director reviews the classified diff. The CEO approves the old and new sha256 in the vault. Then the fixture and refutations.py:18 change in one commit.

A1 step 3 must come before A2, so that A2 edits no validator.

**Tests that pin plain moves:**
- turts_sim/tests/orders.rs:330, 383, 496, 555, 607, 988. The test at :988 (no order operation) inverts.
- group_movement.rs:109, 282 (inverts), 325, 1895, 2005, 2156, 2757, 2876.
- orders_plain.rs:118 (inverts).
- command.rs:341 (clamp; keep it).
- The unit tests in movement.rs:3474-3700.
- The MoveTo users in economy_ordering.rs and economy_accounting.rs.

**Guard (design 6.3 step 4, :624).** Keep `one_unit_alone_takes_the_point` (group_movement.rs:1309; exact (32,32) at :1322). Add cases:
- through MoveTo;
- through an envelope with group 0;
- with a static unit of radius 4 far away;
- to a point off the map: the unit ends at the clamped point, bitwise.

T7 (within 0.011) stays.

## A3. Neighbour grid

**Proof, part 1 (traced now).** `UniformGrid::within` filters by exact distance (grid.rs:85-87, 103-105) and sorts by (id, index) (grid.rs:109). Its result is {i : |p_i - q| <= reach} in id order, for any cell size. The cell size changes only the cost.
- NOT TRACED: an f32 floor at a cell edge (grid.rs:63,120-124) could drop a point at exactly `reach`. Add a property test that compares `within` with a brute-force scan.

**Proof, part 2 (the callers).** Each movement caller passes `r + max_radius` plus step terms. Each one then filters by the pair distance, where `other.radius <= max_radius`. The 21 sites are listed in scratchpad `within_calls.txt`.
- One exception: `refresh_slot_clearance` (movement.rs:1110-1117) uses `max_radius` as a semantic cap.
- Its values feed only `has_potential_free_slot` (:1166-1204), through `.last()` and `>= radius - 1e-5`.
- The cap is a monotone map. So a cap of the largest mover radius gives the same result.

**Fix:**
- Movement: two grids over one index space.
    - Movers: cell = 2 x the largest mover radius.
    - Statics (`speed` None, movement.rs:1414): cell = 2 x the largest static radius.
    - `within_pair(q, r, extra)` queries each grid with r + that class's largest radius + extra, then merges and sorts by (id, index).
- Combat and engagement (combat.rs:111-118, orders.rs:722-728): cell = the smallest weapon range in the catalog. The queries already use the weapon range (combat.rs:330, orders.rs:1115).

**Proof runs:**
- A test-only copy of the pre-A3 kernel compares outputs bit for bit on seeded frames: 1,000 to 5,000 movers and 0 to 20 statics of radius 0.5 to 8.
- The stream must stay unchanged.
- GM40, GMA and GMU must pass.
- Re-run the RUN126 probe and record the time, not judge it (1 static, radius 4).
- Plant that must go red: query the statics with reach `r` only.

## A4. Split `step_frame_preserving_resting`

**Decision: yes.** Do it after A2 and before A3, with the same reference harness.
- Phases: `build_index`, `escapes` (:1515-1660), `blocked_recovery` (:1660-1880), `arrival`, `steering` (:2125-2190), `projection` (:2190-2230) and `spare_claim` (:2290-2400), with a `FrameState` struct.
- This is not a pure move. The proof is the reference copy plus an unchanged stream.
- Cost estimate: 1 build run and 1 review run.
- Reason: A2 and A3 edit about 20 places in this function anyway. M4 and M4b need the split.
- Alternative I did not take: defer to M4. Then the harness gets built two times.

# B. MP1 engine mechanisms

**A shared pure crate.** Add `turts_rules`: bevy-free, with its own f32 point type. It holds placement validity, `relation(a, b)` and the damage formula. Reason: the lockfile has two glam versions (Cargo.lock:3818-3833), and the client must not hold a second copy of the rules.

1. **Construction with a placement ghost** (`turts_sim/economy/construction.rs` plus `turts_rules::placement`)
    - Authority: the server checks the site and owns progress. The client ghost uses the same function, for colour only. A mismatch gives a per-unit refusal (D65).
    - API: a `constructor` capability, a `Construct` behaviour (Content + Point target), a site entity with progress, and events for started, completed and cancelled.
    - Decision now: the site owns progress and its job. Then N sites can progress at the same time, a worker can build (MB), and a site can be attacked.
    - Trap: progress inside the command-centre queue.
    - The rule "at most one work capability" (content_m3_validation.rs:418) blocks a producer that is also a constructor. It needs an exemption row.
2. **Power** (`economy/power.rs`)
    - Authority: the server computes supply and demand for each slot, each tick.
    - API: a resource kind `Capacity`; unit fields `provides` and `consumes`; a per-capability `when_short` (production rate ratio, weapon disabled).
    - Decision now: `scope: Slot` is explicit, so M6 can add `Network`.
    - Trap: "power" checks hard-coded in the production or combat code.
3. **Damage and armour** (`economy/damage.rs`)
    - API: `damage_type`, armour `{class, flat}`, and a damage-type x class ratio table.
    - Formula: max(min, floor(dmg x ratio) - flat). It covers both the C&C and StarCraft models.
    - Integer rationals only (invariant 15). Trap: f32 multipliers.
4. **Engineer repair** (`orders/behaviours/repair.rs`)
    - API: a `repairer` capability with rate, filter, cost per HP charged over time, and `consume_on_use`; a `Repair` behaviour.
    - The repair charges are ledger events, so turts_audit sees them.
    - Trap: a direct write to health outside a transaction.
5. **Rally** (`behaviours/rally.rs`)
    - API: `SetRally` (Point, Unit or Node).
    - On spawn, the engine sends the follow order through `apply_order` with source Engine. A command-log replay derives it again (G9).
    - Trap: a fake player envelope.
6. **Charge-over-time payment**
    - API: `PaymentPolicy::ChargeOverTime` (schema.rs:398), charged in progress steps. Progress pauses when credits are short. Refund = ratio x charged.
    - The step size is mod data. Do not fix it in the engine.
7. **Cancel and refund**
    - Production cancel exists (CancelRefunds, schema.rs:410).
    - Add `CancelConstruction`.
    - `remove_unit` (transaction_m3.rs:354, the one death path, called from combat.rs:405) cancels jobs with an `on_destroy` ratio and releases claims.
8. **Teams** (`slots.rs`, new `relations.rs`)
    - `SlotRelation` (schema.rs:245-249) gains `Ally` and `Enemy`. `Other` keeps its meaning.
    - Decision now: export takes a viewer group (team or slot), so M5 can share vision.
    - Trap: using `Other` to mean enemy.
9. **Match lifecycle and an outcome for each slot** (`match_state.rs`)
    - The victory rule is declarative (`LastTeamStanding`, with a tag) behind a `VictoryRule` trait, so MS can add a script implementation later.
    - Outcomes are checked once per tick, after all removals. A simultaneous loss is a Draw by default.
    - A defeated slot keeps its slot (D25).
    - Trap: a `command_center` check in engine code (invariant 1).
10. **Victory and defeat screen** (`client_screens.rs`, engine widgets, D67)
    - It reads only the reliable `MatchState`.
    - Trap: working out the result from the unit list. That breaks under fog.
11. **Lobby** (new `turts_server/src/room.rs`)
    - The lobby state stays out of `turts_sim`. The sim gets a `MatchConfig`.
    - A seat holds `Human(id)` or `Bot(id)`, so MP2 bots use the same path.
    - Colour never enters the sim.
    - `SLOT_COUNT` (slots.rs:7) becomes part of the config.
12. **Automatic targeting** (`combat.rs`)
    - API: weapon `acquire {range, leash}` and a `priority` list of tags.
    - Decision now: target queries take a visibility predicate. It is always true in MP1, and M5 and the bots use the same query.
    - Add a UnitId-to-Entity index. Today there are 78 `iter_entities()` calls, and `validate_target` scans the world (orders.rs:3056, 3085).
13. **Attack, hit and death effects**
    - Export sends per-viewer `CombatEventRecord`s. They are cosmetic: the state comes from `removed` and `healths`.
    - Effect assets go through the existing `exports.assets` mechanism.
    - Trap: a side channel outside export.
14. **The two remaining M3 rules**
    - Lift "health requires removal" (content_m3_validation.rs:374-376): a unit with health and no removal is destroyed with its contents.
    - Lift "a unit with a queue cannot be removed" (:420-424), as in item 7.

**API version.** Make one step, 0.2.3 to 0.2.4 (turts_mod_api/src/lib.rs:52). All the MP1 changes are additive. Add one version-table row for each feature (version_table.rs:21-52).

**Wire.** Make one bump, `NET_PROTOCOL_VERSION` 4 to 5 (turts_net_protocol/src/lib.rs:13). It holds:
- `unit_orders` changes only, plus removed ids, taken from the CommittedTick batch the same way healths are (export.rs:180,190-195; replica.rs:523-525);
- construction, power, combat-event, MatchState and lobby records;
- `rally` on the order record;
- teams and colours in Welcome;
- the new refusal reasons (messages.rs:146).

All of it goes through the per-slot export (export.rs:25,106). Today that export does not filter units (export.rs:28).

**Schema and wire freeze.** The first MP1 commit adds every schema field, every table row and every record, with stub refusals. Then the lanes do not all edit schema.rs, version_table.rs and messages.rs.

**Bots.** `admit_player_commands` (session.rs:382) and its token bucket (:60-86) stay the only way into `CommandQueue`.

# C. File shape for parallel lanes

First move the inline test modules out (pure moves). Then split the production code.

- **C1 economylog.rs**: A1 step 1. Pure.
- **C2 orders.rs** (tests start at :3673). Pure moves:
    - `mod.rs`: types (:417-507);
    - `pending.rs`: :49-416, :1134-1189, :1358-1416, :1652-2185;
    - `plain.rs`: the plain code (A2 deletes it);
    - `run.rs`: :697-1134;
    - `apply.rs`: :2610-3101, :3331-3673;
    - `behaviours/attack.rs`, `follow.rs`, `queue.rs`: :2185-2610.
    - Then, not pure: a behaviour handler table. A new MP1 behaviour = one new file plus one line in the table.
- **C3 movement.rs** (tests start at :2512). Pure moves:
    - `spread.rs`: :116-480;
    - `search.rs`: :517-666;
    - `plain.rs`: :671-990 (deleted in A2);
    - `pool_index.rs`: :995-1345;
    - `frame.rs`: :1382-2408;
    - `ecs.rs`: :2410-2512.
- **C4 turts_interaction lib.rs.** Placeholder files already exist (compose.rs 68 B, groups.rs 71 B, selection.rs 67 B, smart.rs 71 B). Pure moves:
    - `selection.rs`: :539, :963-1106;
    - `groups.rs`: :921-963, :1343;
    - `smart.rs`: :1174-1412;
    - `compose.rs`: :1106-1174, :1500-1550;
    - `bindings.rs`: :776-914;
    - `world_view.rs`: :1551-1616.
    - Then a refactor: split `Core::step` (:137-539) by input kind.
- **C5 visuals.rs.** Pure moves:
    - `materials.rs`: :38-185, :1372-1482;
    - `markers.rs`: :253-430;
    - `health.rs`: :222-250, :373-396, :1113-1193;
    - `tooltip.rs`: :1282-1372;
    - a new `effects.rs`.
- **C6 client_shell.rs.** Pure moves:
    - `bindings_editor.rs`: :46-700, :1626-2115;
    - `settings.rs`: :698-740, :1085-1627;
    - `notices.rs`: :738-1085.
- **C7 (not in your brief, also hot).** `widgets/mod.rs` had 12 MC commits; `core_adapter.rs` had 10. Move their tests out (:1681 and :1430). Split `widgets/mod.rs` into `tree.rs`, `values.rs` and `activation.rs`.

**Pure moves a reviewer can check by diff:** C1 step 1, all the test moves, and C2 to C6 except the handler table and the `Core::step` refactor. Check: `git diff -M --color-moved=dimmed-zebra --color-moved-ws=allow-indentation-change` shows no uncoloured +/- lines other than `mod`, `use` and visibility lines.

**Order:** C1 → C2 and C3 → A2 → re-freeze → A4 → A3. C4 to C7 run in parallel with these, because they touch other crates.

# D. Crate layout after MP1

| Crate | Responsibility | Forbidden dependencies |
| --- | --- | --- |
| `turts_mod_api` | Adds the MP1 schema; version 0.2.4 **(changed)** | Any `bevy*` |
| `turts_rules` **(NEW)** | Pure rules: placement, relations, damage formula | Any `bevy*`, `turts_sim`, `turts_client`, wgpu, winit |
| `turts_net_protocol` | Protocol v5 records **(changed)** | Any `bevy*` |
| `turts_sim` | Adds construction, power, damage, relations, match state, a unit index; `orders/` and `movement/` folders; one movement model **(changed)** | bevy_render, bevy_winit, bevy_pbr, `turts_mod_runtime`, `turts_audit` **(new)** |
| `turts_audit` **(NEW)** | Log format and the ledger validator, one file for each domain | bevy_render, bevy_winit, bevy_pbr, client crates, `turts_server`, `turts_pack`, `turts_mod_runtime`, `turts_net_lightyear` |
| `turts_mod_runtime` | Adds the version-table rows and the lifted rules | Any `bevy*` |
| `turts_pack` | Its tests use `turts_audit` | Not a dependency of `turts_client` |
| `turts_net_lightyear` | Registers the v5 messages | `turts_sim`, `turts_mod_runtime` |
| `turts_server` | Adds `room.rs`; `economylog.rs` keeps only the sink **(changed)** | bevy_render, bevy_winit, bevy_pbr |
| `turts_interaction` (no row in CLAUDE.md today) | Input to orders; the split files and a placement mode | Any `bevy*`, `turts_sim`, wgpu, winit, `turts_audit` **(new)** |
| `turts_client` | Adds effects, lobby and result screens | `turts_sim`, `turts_audit` **(new)** |
| `turts_client_web` | No change | `turts_sim`, `turts_audit` **(new)** |
| `turts_checks` | Adds the `Position`/`UnitOrders` writer scan | Not a dependency of any crate |

# E. What I would refuse (at most five)

1. **A validator branch for any MP1 behaviour.** That branch was the queue point (16 MC commits), and it is a second model.
2. **A second movement path for rally, repair, construction or attack approach.** It goes against the CEO decision of 2026-11-11, and M4 and M4b would have to change both paths.
3. **Default-mod rule names in engine code** (victory, power, engineer). This breaks invariant 1 and spec 4.3, and blocks MB.
4. **Any frozen-value change other than the one approved re-freeze.** Also refuse a re-freeze made by "regenerate with the current code" without the classifier.
5. **Closing an Open item without a measurement or a decision:** tick rate, numeric budgets (charge step, power quantum), transport, or a scripting variant (D34). Also refuse client authority over placement or outcome (D18).

# What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| grep each refutation expected-failure string in crates/tests | Which faults get their red from economylog | More than one hit in economylog | Only E8-mixed-cost (economylog.rs:2349) |
| grep `validate_records`/`read_jsonl` callers | Users of the validator | Hits outside 3 test files and session.rs | economy.rs, el1.rs, mc_orders.rs, session.rs:1053-1059 |
| read grid.rs:61-111 | Results do not depend on cell size | `within` returns unfiltered cells | It filters by distance and sorts |
| dump 21 `within` call sites | Each caller filters by pair distance | A caller uses the raw list | One semantic cap, at :1110 |
| grep clearance uses | The cap only feeds a prefilter | Another reader | Only :1166-1204 |
| read m3_m2_stream.rs:31-36,119,263,331 | The stream has no positions | Position fields | Count, kinds and before==after only |
| `sha256sum` of the fixture | Hash bcbf72c... | A different hash | It matches |
| `git log bb611bd..HEAD` per file | Hot-file counts | Counts differ | 16/12/12/11/11; also widgets 12, core_adapter 10 |
| grep glam in Cargo.lock | One glam version | — | 0.33.12 and 0.34.1 |

# Not traced

- The A4 cost.
- Whether the mc_orders tests assert their outcomes.
- The f32 cell-edge miss in the grid.
- How the boundary fixture selects its rows.
- That a pooled single move dispatches in the same tick.
- How the pool of a lone unit is collected after arrival.

I did not run cargo.

# What I changed

No repository file. My memory directory only: `/home/julien/Projects/TuRTS/.claude/agent-memory/technical-director/project_mp1_input.md` and its line in `MEMORY.md`.

Evidence files:
- `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/failsrc.txt`
- `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/within_calls.txt`
- `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/elog_items.txt`
- `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/hot_items.txt`

# Open questions for the orchestrator

- The producer must decide whether `turts_rules` and the schema/wire freeze belong to the slice or to MP1 lane 0.
- The CEO approval of the re-freeze needs the classifier output first.
