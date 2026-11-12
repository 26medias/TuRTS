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

## Probe P-A: the browser GPU thread on the test host (2026-11-12, `target/mc-evidence/reviews/probe-a-gpu-thread.md`)

- On this host (Xvfb, RTX 3080) most of the 12 to 14 ms of the Chrome GPU-process main thread for each presented frame is the copy of the frame to the virtual display. A clear-only page costs 7.9 to 10.7 ms at 1280x720 and 2.28 ms at 640x360; the real client 12.3 ms. The second window adds no cost for each present. One real client window at 57.7 fps uses 89 % of that thread.
- Effect on the plan: three headed clients on this host need a frame limit (row SLOT1 stores 20); two headed clients run at about 30 to 40 fps each (E-GPU). This is a property of the test host, expected to be absent on a real desktop (NOT measured). The person run of PT1/PT2/PT3 on a real display is the check.
- Product items to assign to `PRES-MEASURE` (owner `bevy-rendering-engineer`, lens `performance-architect`): the client writes 602 to 617 KB to the GPU each frame for a 15-unit scene, one write of 565,760 B (buffer NOT TRACED); 12 to 15 % of the GPU-thread samples of the real client are in the nvidia kernel module against 1 to 2 % for synthetic pages (cause NOT TRACED). Next probe: count `createBuffer`, `createTexture`, `createView`, `createBindGroup`, `mapAsync` and the pass attachments for each frame.

## Open harness items for `qa-lead` (2026-11-12)

- The M2 convergence tick check carries no load in a static world (reviewer plant B stayed green; older than L0-DBG). Use a world that changes, or compare against the server tick log.
- The old SH1 flake "Execution context was destroyed" at `tests/browser/harness/driver-io.js:371` (red at base 51f1223 too).
- A plant run with `mp1_plants.py --files` selected records of other lanes and a Q-PREBUILD plant was found left in `scripts/mc-browser.sh` (restored by hand; cause NOT TRACED: the runner or an interrupted run). A runner must never leave a plant in the tree: reproduce or rule out.
- The plant runner made `target/debug` (1.4 GB) outside the lane cache wrapper: the record commands must run with the lane cache.
- **The 565,760 B write (from the pacing third pass, by size match; no run-time trace):** it is the Bevy UI vertex buffer, written whole each frame (`bevy_ui_render-0.20.0/src/lib.rs:2423-2424`): 5,440 vertices of 104 B = about 1,360 UI quads each frame, followed by 32,640 B of indices. Which UI nodes give 1,360 quads is NOT TRACED. Item for `PRES-MEASURE`, with `ux-designer` for the widget side (`UI-HUD` adds more widgets).

## Forward port owed: the replica stall repair (MC commit 50eb827, merged on `codex/mc-integration`, 2026-11-12)

- Defect: an ordered attacker kept `last_target` on the unit it killed until the cooldown ended; the owner's client rejects any update that names an absent unit, so the owner replica stalled about 1 s on each such kill. Repair: the removal function clears `last_target` for every weapon. Review: `target/mc-evidence/reviews/review-replica-stall-50eb827.md`.
- Port to `pre-mp1/integration` and `mp1/integration` (owner `simulation-core-engineer`; before `L-TGT` and before C1): one conflict each, in `scripts/acceptance.sh` (keep both; add the `mc_kill_replica` line after the el1 line). On the slice branch add the `last_target` clause and a plant test to `crates/turts_audit/src/combat.rs:66` (`check_removed`). On the MP1 branch confirm that the two new sim tests compile and pass with the L0-SIM tick order, and recheck any system added to the `Acquire` set that reads `last_target`.
- The attack range defect from E-GPU (the unit stops at 4.000025 with range 4.0) is still open, same owner.

## From the second CEO play test (2026-11-12; `input-ceo-second-play-test.md`, UX seat; binding for the next plan revision)

- Facts: in the MC forge build 9b3a7bd a guide opens on every join as a full-size layer that the map shows through (`widgets/guide.rs:13-19`, `hud.ron:118-128`). A press over any widget does not reach the world (`core_adapter.rs:891-913`), so a click on a building selects nothing while the guide is open. The first click on "Close" is lost. Every building has orders in the package; "nothing to do" was a display and input fault. The `player` blind check passed because it judged frames made by a driver that closes the guide with F1 from the bindings: it never chose an input.
- Ruling (orchestrator): the nine conditions M1 to M9 of that report (section 6) are acceptance conditions for MP1. Owners: M1, M3, M6 `L0-CORE`/`UI-HUD` (input reach and cursor truth); M2 `UI-HINT` (design D:1995 "the guide does not open at the start" had no owner task: now `UI-HINT`); M4, M5 `C-UI`; M9 `UI-HUD` with engine request E2; M7, M8 `C1-PLAYER1`, `C1-PLAYER`, `I-PLAY1`.
- Ruling: the person-readiness protocol of section 5 (cold start from the URL only; a step tool the agent drives click by click from pixels; the statement at 60 s; failure signs F1 to F10; no "with a sheet" verdict; reference red on 9b3a7bd forge) replaces the driver-film method for every `player` judgement in the plan (C1 steps 1 and 2, plan :1114, :1196-1198). No request goes to a person before the verdict PERSON-READY. New lane-0 instrument task `Q-COLD` (owner `qa-lead`, review `ux-designer`): the step tool.
- CEO decision 2026-11-12: the MC person play test is deferred to the first playable MP1 build; MC is recorded as not signed by a person (`vault/Decisions/2026-11-12 MC person play test deferred.md` on the MC branch).

## From the L0-WIRE review (2026-11-12, `review-l0-wire.md`, backend seat)

- The candidate passes its gates; 1 High at the merge: the export rule "write `None` for a weapon target that is not in the view" hides the stall defect from the process test once the combat fixes are merged. Ruling: the export rule stays; plant 1d is re-pinned to cover both layers, and a check at the export boundary fails when the sim state holds a target that names a removed unit.
- Ruling (b) on the job state: protocol 5 removes `elapsed_ticks` and the per-tick `charged` from the job state in `QueueState`; the client derives progress from `JobProgress`. Measured before: `QueueState` in 200 of 200 deltas while a job runs, 106 B each, 57 % of the delta.
- Rulings for `L0-STORE`: every Snapshot carries `Some(match_state)`; the store drops a `JobProgress` that its producer's `QueueState` no longer holds; it drops a `SiteProgress` at `sites_completed` or at removal.
- Ruling (a): an order record may name a removed unit for about one tick (the client does not check unit targets in order records). Stated as an exception in `records.rs`.
- The extra change-list inputs (Bevy change detection on the movement and gather components) are a Baseline refinement of 6.8, on condition that each input has a red test.
- For `L-MATCH-SRV1`: the receiver of `LobbyAction` and `LeaveMatch` takes the player id from the connection, never from the message. `Fired.target` is not an `Option`: fog of war (M5) will need a protocol step.
- Before a stage-1 lane merges, these rows run headed on the protocol-5 tree: T7, the MC orders rows, HUD2, SEL1, MM1, PT1, VIS-full, the M2 oracle.

## The cold agent said "yes" on the build the CEO could not play (2026-11-12, session `scratchpad/c5-cold/session-1`, tool commit 4b5eff61)

- Fact: a fresh general agent with no project memory, given only the one question and the command, played MC 9b3a7bd forge with the Q-COLD tool. It clicked "Click to play", then "Close", read the guide, researched Hardened Shells in 54 s, and answered "Yes, a person can play it". The machine checker failed the session on sound only (F5a: one alert sound 4 times in 15 s). The CEO, on the same build, clicked the buildings first and got no response.
- Ruling (orchestrator): the answer of an agent is never the verdict. An agent reads every text and obeys it; a person clicks what they see. The verdict of the readiness protocol comes only from checks that do not depend on the agent's patience:
  1. A scripted "first-click probe" by the tool itself, before any agent input: after the page is ready and one click to give focus, it clicks the screen centre of each own unit and building (the tool may read the scene bridge for this; the agent never sees it) and requires a visible response and a non-empty selection for each (condition M1). A layer that takes those clicks is a FAIL (F2, F3), whatever an agent does later.
  2. Machine checks from the UI record at the first frames: an unrequested text block of more than 40 words (F9), a widget of more than half the viewport with a see-through fill (F3), an alert line or alert sound before the first input (F5b, F6), a control whose first click is lost (F8, by a scripted click on each visible button).
  3. The agent session stays, for the statement at 60 s and for the findings a person would also meet (here: Disable shows nothing; a stale refusal text; a tooltip over a button), but its "yes" or "no" is not read.
- The reference red (check C5) is now: the first-click probe FAILS on 9b3a7bd forge. The builder's fixed-script run already showed the fact (3 building clicks, 0 pixels changed).

## From the Q-COLD review (2026-11-12, `review-q-cold.md`, UX seat)

- The instrument gives `NOT-READY` on the build the CEO could not play, with no agent (first-click probe 14 of 14 fail; 189 words of unrequested text; a see-through layer over the whole page; alert lines before the first input; a lost first click on "Close").
- New MP1 product conditions (owners in brackets): the start overlay passes the first click to the control under it, or nothing clickable shows under it (`L-LOBBY-C1`, `UI-HUD`); at 1024x768 every close control is inside the page (the MC guide's "Close" has a fixed offset of 1176 px and is outside it) (`C-UI`); at most 40 words in one unrequested text block and 60 in total after the first click, and a goal line of at most 15 words (`UI-HINT`, `C-UI`); each order button gives a visible result or a reason within 1 s (`UI-HUD`, engine request E2); a click on an enemy unit or a resource node shows its name, a ground click clears the selection, a right click on the ground gives a reason or an order mark (`UI-HUD`).
- MP1 `muster` today: the first-click probe passes 60 of 60, but a guide of 66 words opens at each join and its text is cut at the bottom of its box (`widgets/guide.rs:13-19`). Owner `UI-HINT` (condition M2).
- From the review of the seventh CI repair (`target/mc-evidence/reviews/review-ci7-c0adb30e.md`), owner `qa-lead`: add a second bound to the M2 order-latency oracle: the observed tick rate between the two checkpoints is at least half of `tick_hz` (hosted measure 19.4 at 20); the wall guard must fail closed when `wall_time_ms` is missing (`m2-oracle.js:410`); the failure text at `:438` still says "5 s".
- Merge ruling for `codex/mc-fix-reorder` with the 12-tick recipe: the spawn-window subcase keeps BOTH the 600 ms recipe and `--reorder-grace-ticks 0`; its comment ("answer tick 11 ... output tick 10") is rewritten by hand to the merged numbers (answer 7, output 18).

## Toolchain pin (2026-11-12)

- `rust-toolchain.toml` named the channel `stable`. On 2026-11-12 the hosted runner moved to rustc 1.100.0 and the MC hosted run 43288692044 went red in `check` (new clippy lint `needless_bool` at the pending-order queue code) and in the four `m2-native` shards (`economy_variants`). The MC branch now pins `channel = "1.99.1"`. The slice and MP1 branches still say `stable`: each takes the pin at its next merge by the orchestrator. Open task (owner `technical-director` for the date, a domain seat for the work): move to 1.100 on purpose: fix the lint, find the cause of the `economy_variants` failure on 1.100, run the full gates.

## From the Q-COLD re-check (2026-11-12, `rereview-q-cold.md`); Q-COLD merged

- Product condition for `UI-HUD` (row HUD5), from the probe on MP1 `muster`: within 1 frame of a click on an order button, the button shows a pressed state; within 1 s either the world, a panel or a notice changes, or, for an order that needs a target, the button stays marked active and the cursor shows the order's cursor until the target click or Esc. Today Stop, Hold and Attack change 0 pixels around the button; with the sound off by default a person gets no sign that the click registered.
- Low, engine label format: an icon-only button with a key shows a leading space before the key (` [M]`).
- Follow-up for the tool (owner `qa-lead`): the order probe accepts the armed state of a target order (button marked active and the order cursor); a build where the buttons probe finds no button AND the order probe finds no order button for any own unit type is a FAIL ("no on-screen control at all"); the checker's C4 text names P3, P4, P5 and F9b; the rebuild command in `docs/qa/cold-play.md` says "a new label each time".

## From the second L0-WIRE re-check (2026-11-12, `rereview2-l0-wire.md`); 0 High

- Ruling: in protocol 5 the fields `done`, `total` and `per_tick` of `RateRecord` count raw units of 1/Q tick. Today Q is 1, so they equal ticks. The client derives ticks with `base_ticks_done(units, duration_ticks, total)`. Example for `L-PWR`: Q = 2, a job of 100 ticks has `total` 200; at rate 1/2 `per_tick` is 1. No protocol step is needed.
- For `L-PWR` and `L-PAY`: the freeze rule exists in two places (the sim skips a frozen producer before the step; the wire reads `view.m3.disabled`). A new freeze cause must go through ONE predicate that the sim skip and the view both use; the test `crates/turts_client/tests/job_progress_clients.rs` is the guard. Follow-up owed by the L0-WIRE builder: make that predicate now.
- Ruling: the delta reorder repair of the MC branch (client reorder window, MC commits 8365e8be to 01ade17b, merged at 9d2ab86b) must be on `mp1/integration` before the first stage-1 lane that runs process rows under load (`L-MATCH-SRV1`) and before C1. Evidence on this branch: `mc_kill_replica` failed 2 of 12 runs with "delta tick 497 base 496" after "last 495" when the test build ran the server multi-threaded. It comes with the forward merge of `codex/mc-integration` into `mp1/integration`, which also brings the MC test repairs (AUD1, HUD2 are red on the MP1 branch without them) and the compiler pin.
- Accepted exception: test code of `turts_client` may have dev-dependencies on `turts_sim` and `turts_pack` (the crate table forbids the normal edge; `scripts/boundary.sh` reads normal and build edges only). No dev-dependency is added to `turts_server` (it changes the features of the server binary under test).
- Instrument gap: the Snapshot oracle `measured_job_rates` (`crates/turts_server/tests/economy.rs:1847`) cannot go red on a wrong rate today, because no raw-Snapshot scenario has a running job. Owed: one such scenario.
- The slice probe values change at slice task A2b1 (by design): `frame_chains=e6e4417f9cbe5fa5`, `muster_world=49fc63ffd1d8c57d` at f119dc39. The MP1 branch keeps the old values (d0d1b1c572163424, aa9db740578ed5c1) until the slice merges forward; each MP1 task compares with the values of its own base commit.
- L0-STORE review (2026-11-12, PASS, `docs/research/mp1/review-l0-store.md`). Orchestrator rulings: (1) the match clock: the replica takes the start tick at `on_welcome` and `match_clock_ticks()` has no argument (review option (a)); the clock stops with no store record, because the server sends no delta after the end tick (plan :833). The task name `L-MATCH-CLIENT` in the builder errata does not exist in the plan; the consumers are `UI-RESULT` and `UI-HUD`. (2) The new store refusals (a site record for a non-site, a removed or a duplicate unit; a construction queue for a non-own unit, a bad point or unknown content; a supply record for another slot, a non-`Supply` resource or a duplicate; duplicate connection slots) are binding for the writers of `L-CON1`, `L-PWR` and `L-MATCH-SIM`: the plan revision adds them to those task blocks. (3) One getter for `SupplyShort`. (4) Merge order: `mp1/forward-mc` leads; L0-STORE puts its changed lines inside `apply_chained`.
