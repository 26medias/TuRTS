# Design errata that wait for revision 6.3 (from the lane-0 reviews, 2026-11-12)

The orchestrator applies these in one pass when lane 0 exits. Each comes from an independent review of a lane-0 task.

## From the L-ROOM0 review (network-engineer)

- 6.7.8, replace "The server notices a lost client at the netcode timeout" with: "The server sees the end of a connection in
  two ways. A closed transport (a close frame, FIN or RST) adds `Disconnected` to the `ClientOf` entity within 5 ms (native
  release, L-ROOM0 review). A silent loss is seen at the netcode timeout, 10 s. L is the second value."
- 6.7.5, the reload bullet, add: "A page that unloads starts the closing handshake (WHATWG WebSockets, section 7), so the
  release normally comes before the first retry." (A real browser run is NOT RUN.)
- 6.7.8, add: "Under `Immediate` the room has no seat; the hook reads the slot from the `Session` of the ended entity."
- 6.7.5, add one line for the cells that the table calls "not possible" and for Boot: the refusal text of
  `JoinVerdict::Impossible` (the text that the repair of L-ROOM0 chose).
- 6.7.14 item 4 (CEO item, keep or lower the 10 s timeout): add the fact: the 10 s applies only to a silent loss; a closed
  transport is seen in under 5 ms. The recommendation "keep 10 s" costs less than the text assumed.
- Hook ruling for the wiring task (plan task L-MATCH-SRV1; record in the plan block): an observer on the lightyear
  `Disconnected` component with `With<ClientOf>` that reads the slot inside the observer; a `PreUpdate` system before
  `receive_hello` that calls `Room::connection_ended`; `Query<&Session, Without<Disconnected>>` for liveness; `Session` is
  not removed. The task asserts a notice under 1 s after SIGKILL.

## From the L0-RAW review (mod-builder)

- 6.10 "Version-table detection", add: "Exception: `dependencies` is required since API 0.1; its 0.2.2 feature is a declared
  dependency, so its row reads the parsed value (a non-empty map)."
- 6.10, add: "A path looks through a list." and "A field inside an enum variant is detected by a key marker on the typed
  value, so a key written with `None` counts."
- 6.10, add: "The version rows that read the raw document run before the content cross-checks."

## From the L0-RULES review (technical-director)

- 6.6 (near D:774), old: "gives the relation." New: "gives the set of relations that hold (they overlap: the own slot is
  Same, Ally and Any); `matches(filter, a, b, teams)` tests one filter. A slot that is not in the team table is allied
  with no other slot."
- 6.1 check (near D:474), old: "overlaps a footprint, a unit included". New: "overlaps the footprint of a unit or a site, a
  mobile unit included; an overlap with a resource node fails check 5".
- 6.4 (near D:631), add: "A `total` of 0 is complete: the charge is the full cost; `done` above `total` counts as `total`."
- 6.2: the combine rule for several short supply resources is not defined (MP1 has one). Open point, owner
  game-designer, before M6.
- Plan note: `UI-PLACE` builds the placement `View` from replica positions, and one shared-scenario test compares the
  server `View` and the client `View` (the conversion cannot make the two sides disagree; the view assembly can).

## From the Q-PLANTS review (performance-architect)

- Plan section 4: the slow gate runs the plants through the parallel runner set `mp1` (one worker for each command group),
  not the serial runner; a headed record never runs without the headed wrapper.

## Binding notes for later tasks (from the lane-0 re-checks, 2026-11-12)

- `L0-SIM` (or the next task that edits `crates/turts_checks/tests/formula_ratchet.rs`): make the count comparison exact
  (a count that goes down is red too, with the "record N" message, because the pass output is captured and the headroom
  would let a later copy in); add two fingerprints: a `/` whose operands are bound by a `let` with a `u128` type; and no
  function outside `turts_rules` whose name equals a public `turts_rules` function. Today a formula copy written in u64
  with `checked_mul`, or with a `let w: u128 = …` binding, is not seen.
- `L-REP`: `turts_rules::repair_charge_difference` returns `Option<u64>` (None = cannot pay), or the packer refuses
  `max x amount / per_health` above u64. Today a total above u64 saturates and the tick is undercharged with no signal.
- `L-PAY`: remove the refund formula copy at `crates/turts_sim/src/economy/production.rs:226` (the ratchet table counts it)
  and lower the count.
- `L-MATCH-SRV1`: the hook ruling of `docs/research/mp1/trace-connection-end.md`; assert a notice under 1 s after SIGKILL
  (the process test `connection_end.rs` has no lower bound and cannot tell the hook path from the timeout path).
- Stage 2 (`L-LOBBY2-SRV`): a test for the `CountdownToLobby` edge of `room.rs` (a plant that removed it stayed green).

## Note for every later visuals and effects task (from L0-SPLIT and its review, 2026-11-12)

- A new visuals concern adds one file under `crates/turts_client/src/visuals/` and one call line in `draw_world_visuals`
  after the last present concern call and before `update_health_fills` (the block `draw_queued_targets`,
  `update_health_fills`, then `reconcile_visual_entities`), and writes into `desired` through `&mut DesiredVisuals`.
  Several tasks add a call line in that same region: expect a textual conflict at the merge and keep the order.
- A new retained entity type also adds: a despawn loop in the early-exit block of the caller, a `Query` parameter on the
  system, a field in `VisualState` if it keeps state, and a reset in `sync_session`. `visuals.active = false;` stays
  before the let-else.
- Call pairs that must not swap: `sync_session` before the let-else; `resolve_hover` before `draw_hover_ring` and the
  tooltip; `intake_markers` before `track_blocked`; `draw_order_markers` before `draw_blocked_marks`;
  `collect_health_bars` before `update_health_fills`; every draw call before `reconcile_visual_entities`.
- The scene record `visuals/scene_oracle.txt` is a record of behaviour. A task that changes behaviour re-records it with
  the ignored test `bless_scene_fixture` and shows the diff of the record in its review. A task that adds a concern adds
  a scene for it.
- A new effect family adds a function in `core_adapter/effects.rs` and one match arm in `drive_core_adapter`.
  `Effect::Marker` and `Effect::Sound` have no producer today, so no test reaches those two arms.
- The fault registry case `E9-browser-debug-only-ui` now plants `visuals/rings.rs`. Its browser run is owed before the
  merge of the first later task that edits `rings.rs`.

## Binding notes from the L0-RAW re-check (mod-builder, 2026-11-12)

- `L0-API1` (the first task that puts `turts_mod_api::Written<T>` on a schema field): `Written<T>` does not round-trip
  (a missing key serialises as `None`, and parsing that text gives `written = true`), and its derived `PartialEq`
  compares the flag. Add `is_unwritten()` and use `#[serde(default, skip_serializing_if = "Written::is_unwritten")]` on
  each field; add a round-trip test; decide if equality ignores the flag.
- `L0-API1`: the raw rows of the version table run before the parsed rows, so a package with two old-API features can
  get its refusals in two steps (first a raw row, then a parsed row), and a parsed-row message names no file. Accepted for
  now; the task makes a parsed-row message name its file where the value knows it.
- Verified: no released package form changed from accept to refuse (old and new packer on unit, map and placement forms at
  seven requirement forms; every exit code equal).

## From PRES-PROBE (2026-11-12; `docs/research/mp1/probes/pres-probe/README.md` on `mp1/integration`)

Probe 1, a model from package bytes on Bevy 0.20: YES (native headless; wasm32 `cargo check`; the browser run is NOT TRACED).
- `PRES-MODELS` (plan): use the flat template (design 8.1 says no scenes; the plan row says `WorldAssetRoot`: the design
  wins); register an EMPTY shared memory `Dir` as an asset source before `DefaultPlugins` and insert the package bytes at
  acquire time (late bytes work: the `Dir` is shared); put the lock hash in the asset path; keep `Mesh3d` on the unit root
  (the browser scene bridge filters on it); keep `turret` and `muzzle` as own entities for design 8.2.
- The per-slot team material works with shared handles: insert the slot's `MeshMaterial3d` on each entity whose
  `GltfMaterialName` is `team`; two units of one slot share one handle; no material for each unit.
- Bevy 0.20 facts for the task text: `GltfPrimitive.material` is `Option<Handle<GltfMaterial>>`; the labelled sub-asset
  `Material{n}/std` exists only with `PbrPlugin`; `GltfPlugin` must come before `PbrPlugin`; a manual update loop needs
  `app.finish()` and `app.cleanup()`; a headless load test needs `TaskPool`, `Asset`, `Transform`, `Mesh`,
  `WorldSerialization` and `Gltf` plugins (and, for the standard material, a window plugin with no window, camera, image,
  a render plugin with no backend, and PBR).
- A Bevy load of the bad-pivot model succeeds, so the pack check (MDL1) is the only gate for the model rules.
- The headless client branch has no `AssetPlugin`: `PRES-MODELS` decides what the headless client does with a model.

Probe 2, the HUD change counter on today's tree (forge HUD, 108 widget entities, a static replica):
- Today: 89 `Text`, 66 `Node` and 108 `Visibility` components are marked changed in EVERY frame (red in 58 of 58 frames).
- The four write sites that the plan gate named are not enough. `UI-HUD` must change: `widgets/values.rs:39, :65, :124`;
  `widgets/tooltip.rs:97, :100`; `widgets/tree.rs:73, :76` (`update_fit_layout`), `:96` and `:840` (callers of
  `apply_node_placement`, which must end with `set_if_neq`), `:104` and `:848` (`UiTransform`). With all of them the count
  is 0 after frame 2.
- Time for one update, release build: 2.8 to 3.0 ms today, 1.7 to 1.8 ms with the changes (about 40 % less; the host was
  loaded, so read the ratio; the browser cost is NOT TRACED).
- The red test for `UI-HUD` is `hud_change_counter.rs` of the probe folder; it runs headless with no GPU. The counter
  must be a system (a world read after `app.update()` always reads false).
- NOT TRACED: other systems that write `Text`, `Node` or `Visibility` each frame outside the test app
  (`client_screens.rs:256`, `settings.rs:558`, `bindings_editor.rs:1042/1065`, `notices.rs:141/166`,
  `volume_settings.rs:227`, `visuals/world_tooltip.rs:21`). `UI-HUD` or `PRES-MEASURE` checks them.

## The headed wrapper is in use (Q-LOCK: review FAIL with 2 High, repair, re-check PASS with 0 High; 2026-11-12)

- File: `target/mc-evidence/gate-runner/mc_headed_v6.sh` in the main worktree (outside the repository), sha256
  `1d3a0000e37cd7d001827cb32843807c45c5824a71640cccc323c2d090ebb675`. Daily read: `gpu_receipts_today.sh` next to it,
  sha256 `f1660fee4dc54f09ed909c3ec7c47fc8f55e05c02ef4be9bfac80ccc53e2574a`. The plan section 1 takes both hashes.
- Limits, interim until the builds leave the lock (task Q-PREBUILD): table 1,500 s, pre-merge 1,800 s. After Q-PREBUILD
  merges, the proposal to verify on the first prepared headed runs: table 600 s for a row and 1,700 s for PT1 only;
  pre-merge 900 s for a session without PT1 and 1,800 s with PT1.
- The text that every headed task prompt carries: "Call `mc_headed_v6.sh --mode pre-merge|table` with `--mode` first and
  a new `--label` for each call. Make one call for each row in table mode. Build every server, package and wasm before
  the call. Exit 124 or 137 is the wrapper's hold limit, not a test red. Exit 76 means the caller died: run again with a
  new label. Exit 4 means the queue folder is not writable: stop and report. Exit 2 or 3 is a wrapper fault. Exit 75
  never reaches you, so do not retry for it. Never kill the wrapper's process group; send TERM to the wrapper pid."
- Rule for the old wrapper (v4): a v4 call takes the same lock with no limit, no ticket and no receipt, so it can block
  every v6 call and nothing records it. Every lane uses v6 from its next headed run. A run that needs more than the
  limit (the M2 browser fault set held 3,545 to 4,671 s under v4) is split into one call for each case.
- Low items for the next edit of the wrapper: the self-check watchdog leaves a `sleep 200` that holds stdout (a piped
  self-check takes 166 s); a child with a cleared environment in a new session is not found by the marker scan (scan by
  the session id too); the daily read selects by start date (select by end).

## Prepared headed runs (Q-PREBUILD: review 1 High, repair, re-check PASS with 0 High; 2026-11-12)

- Every headed MP1 run uses the prepared form: `bash scripts/mc-pixels.sh --prepare <DIR> <one row file>` outside the lock,
  then under the wrapper `bash scripts/mc-pixels.sh --prepared <DIR> <row files>`; the same options exist for
  `mc-browser.sh`, `t7.sh`, `m3-browser.sh` and `m2-browser.sh`. A prepared run builds nothing and exits 66 when the
  source, the package, a prepare-time script, a tool version, a build flag or an artefact does not match its manifest.
  HEAD is strict: re-prepare after each commit (24 to 101 s, outside the lock). Notes: `docs/qa/prepared-runs.md`.
- Measured: time to "page served" 40 to 51 s with the builds inside the run, 0.5 to 1.1 s prepared.
- Hold limits to verify on the first prepared headed runs: table mode 600 s for a row and 1,700 s for PT1 only; pre-merge
  mode 900 s for a session without PT1 and 1,800 s with PT1.
- A pixel plant: one scratch checkout for each plant with the plant left as a dirty edit; prepare there only when the
  plant touches a hashed root; a plant in a driver or oracle file needs no prepare.
- Low items for the next edit: `tool_version` must run with `cwd` at the repository root (a run started elsewhere can
  give a false 66); `m2-browser.sh` does not export `CARGO_PROFILE_DEV_DEBUG` and `CARGO_PROFILE_TEST_DEBUG` (a false 66
  outside the lane runner); the ALSA `PKG_CONFIG_*` variables of the native PER1 build are not compared.
- Not done yet: the M2 browser fault runner still builds inside one outer lock (3,545 to 4,671 s). It needs a prepare
  after each plant, the lock taken for each browser phase, a lock-only mode of the wrapper, and a no-git fallback of
  `mc_prepare.py`; the baseline folder can be used again after the restore. Size: about 4 runs. Owner: qa-lead.

## From the L0-SIM review (2026-11-12, `review-l0-sim.md`)

- The review failed with 2 High. Orchestrator ruling: the gated design stands for both. H1: Supply runs before the commands of the tick (6.2). H2: one payment order over sites and producers (6.4). The technical director rules the open order items (E-c income and commands, E-d site completion, E-e repair slot, `spawn_order` after Orders) in `ruling-l0-sim-order.md`; its errata go into revision 6.3.
- **Risk for D58, not from this task (lead, measured by the reviewer; not re-run by the orchestrator):** with 5,000 synthetic units one tick takes 12.0 s at the L0-SIM head and 13.2 s at its base. perf: 41% in `UniformGrid::within`, 24% in sorts inside `within`, plus `spread_destinations` (MC movement). Owner to assign: `performance-architect` with `simulation-core-engineer`, before the first hosting benchmark.
- **Order ruling accepted (orchestrator, 2026-11-12):** `ruling-l0-sim-order.md` is binding, with the Repair set after Acquire and before Combat (heal before damage; the client derives a hit from the health drop, D:614-615). Revision 6.3 takes its errata: E-c (6.4 at D:644), row PAY1 (D:2451, "in the tick of the next delivery"), E-e (6.5 after D:703). Frozen order: FixedPreUpdate Advance, Supply, Construction (placement of held orders only), Commands; FixedUpdate Income, Orders, Movement, Gather, Extract, Transport, Inserters, Production (the one pay pass), Acquire, Repair, Combat, Outcome; FixedPostUpdate Collect.

## From the L0-CORE review (2026-11-12, `review-l0-core.md`)

- The review failed with 1 High: the plan (:467) freezes `WidgetAction::FocusAlert` with no field, and design D:2059-2061 needs a camera target and, for some alerts, a unit to select. Orchestrator ruling: option (a), the client action is `FocusAlert { x, y, unit: Option<u32> }`; the mod-facing form of 6.10 stays as written. The plan text at :467 and :779 takes this in the next plan revision.
- Ruled as lane-0 work, not later change requests: an empty `placement::intercept` call at the top of the event loop (GHO1 (d), D:2470), and an empty state, input and output type for each stage-1 owner in that owner's file.

## From E-GPU (2026-11-12, branch `mp1/e-gpu` 466ed01; leads from the builder, review pending)

- Verdict PASS: two headed clients on one GPU for 600 s, frame p50 33.3 ms and p95 50.1 ms in each window, 0 disconnects, lowest 5 s rate 14.0 fps.
- No MC package gives two armed slots (only `mods/forge/units/turret.ron` and `mods/muster/units/trooper.ron` have a weapon). Plan step 1 of E-GPU assumed one.
- **Product defect to assign (lead, root cause NOT TRACED):** an attack order from out of range stops the unit at 4.000025 from the target with range 4.0, and it never fires (`crates/turts_sim/src/orders/behaviours/attack.rs:30-34`, `economy/combat.rs:321`). Owner: `simulation-core-engineer`, after the L0-SIM repair merges (same files). It must be closed before `L-TGT`.
- The hosted MM1 probe of plan step 2 cannot run: the workflow has no MM1 step. Frame times sit at about 30 fps; cause NOT TRACED (given to the frame pacing repair).

## From the L0-DBG review (2026-11-12, `review-l0-dbg.md`)

- The review failed with 1 High: four `page.waitForFunction(async ...)` predicates never wait (Playwright does not await the predicate), so the join gate and the SH1 and SLOT1 waits passed at once; new reds in SEL1, M2 and SH1. Repair in work.
- Ruling (orchestrator, as the reviewer proposes): the default debug request keeps the 4 Hz cap; `min_interval_ms` 0 is a legal test-only form for the SH1 per-frame phase capture. Erratum for row DBG1: "With a request, the rate is at most 4 Hz; a harness can pass `min_interval_ms` 0 for a per-frame capture (SH1 only, test-only)."
- Old reds at the base 51f1223, not from this task: ORD3 (same message), SH1 run 1 ("Execution context was destroyed"), SLOT1.
- Wrapper note: a v4 headed caller bypasses the v6 ticket queue. Every lane must use v6.

## The M2/M3 trace hashes on the MP1 branch (orchestrator ruling, 2026-11-12)

- Fact (from the L0-SIM second repair; `repair2-l0-sim-report.md`): line 1 of each trace file is a header with `emitter_sha256`, the sha256 of `crates/turts_pack/tests/m2_trace.rs`. `L0-API1` (967a786) added five match arms to that file, so on `mp1/integration` since that merge the four whole-file trace hashes differ from the recorded slice values, while every line after line 1 is byte-equal. The frozen stream `tests/m3/m2_operation_stream.json` (bcbf72c2…) and the `probe_neutral` values are unchanged.
- Ruling: this is not a re-freeze. On the MP1 branch the "no behaviour change" rule compares the trace body (all lines after line 1) with the body of the recorded traces; the header is reported, not compared. The body hashes get recorded at the next merge gate. On the slice branch the whole-file hashes stay the rule.
- Owed (owner `qa-lead`, before the slice merges forward again and before Q2): the classifier pins the emitter sha256 (`tests/m3/refreeze_classify.py:585-616`); reconcile that pin with the MP1 emitter file, with a stated reason, in the forward merge.
- Also from that pass: a rate that Q does not divide is a panic that cites design 6.4 (the packer refuses such a Q). An integrity stop with a diagnostic would need a new `StopReason` and a wire change: owner `L-PWR`, as an open item. `scripts/acceptance.sh` failed 3 of 4 runs at three different steps under host load 10 to 14 (cause beyond load NOT TRACED).
- **Trace body hashes on the MP1 branch (recorded 2026-11-12, verified by the reviewer at c0ab357 against bfea57c; `confirm-l0-sim.md`):** base d78dc09392e5b96917513772488991b617e8d6bb2b70a26056075c28c4822f0c, haul 887ce95f698017026f79685347446e5240f32770e704af9d097e06eef86b074f, acme ba1c849aaee46dd490b9d7d444c4c431bc324b4970e6f5c6edb31a565bd19856, cedar 09f637325c5119727a70286e1b47b0a223cf60ec1a206aebc35853a5ffe540ae (`tail -n +2`; line counts 2,898, 4,820, 802, 4,712). The header must equal the sha256 of `crates/turts_pack/tests/m2_trace.rs`; when it changes, the reviewer reads the diff of that file.
- Conditions from the L0-SIM confirmation: `L-PWR`: the packer computes Q and refuses a Q above 65,536, and a test asserts that Q divides every Rate. `L-MATCH`: the input log records commands in `CommandQueue` order (live, then script). `acceptance.sh` fails under host load above about 25 (t5 margin 0.009; f11 timeout): run it when the load is low.
