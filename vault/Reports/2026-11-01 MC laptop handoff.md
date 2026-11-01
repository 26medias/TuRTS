---
type: report
date: 2026-11-01
tags: [turts, mc, codex, handoff]
---

# MC laptop handoff

PAUSED at the CEO request for travel and transfer. Do not resume until the CEO asks. The implementation is incomplete:15/25 tasks complete,66/170 agent build runs used. No main merge or push. Main remains f050e429b38222c9917314bb23c1568dd7ad816c, design revision14/O13. STOP-PAL1 is released.

## Saved source

- Integration branch: codex/mc-integration. All completed tasks and current vault decisions are committed. This handoff is a documents-only commit on that branch.
- Open Task13 branch: codex/mc-t13 at4dfe0b32a384705acb0823ad6a7defca9948cb0b. Its worktree is clean. This is provisional source, not accepted or merged.
- Task5 branch retains accepted98147aeebedeee83ff6ba8cc4f3cfc5bd5f21a34. Mergee214ccc0ace791b7854039bf777b1499cad7fd53 passed all11 integration gates. Task5 lane was removed with provenance/evidence retained.
- All Git refs and stash chains are included in the transfer Git bundle. The original Task5 WIP stash2af32b0f49fc59600cd54bc716816aeeb68e5229 is historical preservation; do not reapply it over completed code.
- Main's pre-existing untracked vault/Reports/2026-10-28 target disk audit.md is copied separately, not committed.

## Exact Task13 checkpoint

Required baseline native palette and actual headed old-colour VIS1 reds are verified atca0c36b3. Native production/palette tests pass in earlier candidates. SEL2 at80b2711, SEL3 and basic VIS1 at57fc5ea each passed both headed runs. Retain their actual source identities; they did not execute on the latest candidate.

Full VIS1 has not passed. Source4dfe0b3 adds a dedicated OrderMarkerGizmoConfigGroup with depth_bias=-1 for attack/blocked crosses; range and waypoint default gizmos are unchanged. It also repairs three oracle stage-wrapper calls. The last bound wrapper target/mc-evidence/t13-vis1-full-depthbias-4dfe0b3-20261101 has only a row header and an unfinished receipt. Root verified no related live process and polled session99365: Unknown process id99365. Mark this attempt INTERRUPTED, never PASS or semantic red. Raw partial evidence is retained unchanged. Root interrupted the pending builder when the CEO requested pause. There is no active runtime lease/process to continue on the laptop.

The next production repair is C178, recorded but NOT implemented: sim/economy/views.rs omits existing PlainMoveOrderIdentities from UnitOrderView.current. Actual0c26fab captures show Moving/current:null, then Pending/current:null with one queued point. Project the real captured identity as fallback in BOTH view branches, preserving UnitOrders.current precedence, owner filtering, lifecycle cleanup, legacy MoveTo without identity, audit and frozen behavior. First add a real accepted Move Replace plus Append native regression and show its named compiled red, then repair/green; add stopped/removed and precedence controls. Do not invent a client target or weaken the two-leg waypoint pixel requirement.

C175 keeps exact outer footprint rings visible using shared depth-overlay materials and at most two neutral black/white backers chosen from actual slot colour. C176 uses inner0.88/outer1.0 and midband0.94 samples, four coloured quadrant interiors plus four changed non-slot gap regions. Pure controls reject solid, missing quadrant and gaps-only. C177 keeps exact attack/blocked cross shape/position/colour/timing using the dedicated gizmo group. All rulings are in vault/Decisions/2026-11-01 MC Codex rulings.md and the execution board. Design wins over plan conflicts.

Instrumentation repairs: screenshots persist PNG and base64; only saved shots enter checkpoint indexing; first-frame helpers take stage wrappers; after immediate attack capture the pointer moves to PNG-derived ground so its tooltip cannot hide the damaged health bar. Derived offline replay is diagnostic only. No setup/TypeError/compile/missing-artifact failure earns semantic red credit. Root manually read actual7702c8b tooltip as Training Target · Slot 2; final capture/source carry still needs binding.

## Required next sequence after explicit resume

1. Read the brief sections3/4, execution board, planTask13, design4.1/9.1 and today's C161-C178 rulings. Do not read/import Claude configuration or role files. Read the applicable Codex skills only.
2. Register the next builder/reviewer dispatch or resumed run on the ledger BEFORE launch.66/170 used; next run67. Earlier RUN62/RUN66 reports are preserved; no live child state/runtime handle transfers.
3. Create integration and Task13 worktrees from restored branches. Use only one compiler/cache/browser owner globally. Task13 target is target/mc-t13 with all five Cargo settings below. Rebuild dependencies on the laptop; the32GiB cache is intentionally omitted.
4. Complete C178 red/green; run fresh bound fullVIS1 and investigate exact failures without reducing acceptance. The last4dfe wrapper was interrupted and must not be carried as a completed observation.
5. Final Task13 G-RUST/G-QA/G-WEB/affectedG-REF, four required plants (world box, one-frame-late selection, no screen type filter, near-identical palette pair), and independent actual foreign-solid pixel plant. Each red must compile/fail meaningfully and restore exactly to green. Review actual final source and the native system initialization, pipeline/material sharing and raw pixels;0High required.
6. VIS2 second measurement-only red/green/commit: existing docs/measurements/mc.md has title+empty VIS2 section. Grep red must print0/exit1, not missing-file2. Real headed Chrome WebGPU5000units: Muster515+synthetic4485. Record p50,p99,draw calls,memory, machine/Chrome/adapter/raw proof and four numeric rows. Laptop measurements must honestly name its hardware. NVIDIA desktop profile is not a portable requirement; use actual supported WebGPU adapter and prove it. No acceptance threshold is invented; GCP remains pending. If hardware cannot run the required headed proof, mark NOT TRACED and report rather than fake it.
7. Pixel-affected CAM1/HUD1/HUD4 checks, final independent acceptance, merge Task13 into integration and re-gate integration. Close lane/cache only with preservation proof. Complete Wave6 push/read hostedCI. Main is never merged or pushed.
8. Continue waves7(10,15,17),8(14,16,22),9(21),10(23),11(24). Task23 is a mandatory CEO stop: send serve-onlyURL and wait for the CEO PT1/PT2, log not blind.

## Runtime and evidence

Main evidence root: target/mc-evidence. Transfer includes this complete evidence tree except rebuildable __pycache__/node_modules, and standalone build helpers. Histories contain absolute desktop paths; preserve them as historical evidence. Restore maps only evidence symlinks to the new root. Do not relabel prior commands as laptop executions.

Cargo settings for every compile/build subprocess:

```sh
export CARGO_TARGET_DIR=/absolute/path/TuRTS-mc-t13/target/mc-t13
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS=4
```

Registered refutation modules may pin jobs2; record actual values. Never use clippy --all-features. Never git add -A. Commits end with Co-Authored-By: Codex <noreply@openai.com>. Vault-only commits use [skip ci]. Keep frozen M2 SHA bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b.

Useful helpers under restored main target/mc-evidence:

- integration_gates.py LABEL ABS_LANE target/mc-X (native/QA/frozen).
- gate-runner/run_gate.py --lane ABS --target target/mc-X --head FULL_CLEAN --label UNIQUE --gate acc|t7|m3. Do not run m3 during the approved panel-removal window.
- gate-runner/run_mc_rows.py --lane ABS --target target/mc-t13 --head FULL_CLEAN --label UNIQUE --row tests/browser/rows/VIS1-full.json. Set actual display and supported GPU flags; ensure MC_BROWSER_HEADED=1 and MC_PIXELS_HEADED=1. The helper's desktop DISPLAY/profile may need a documented instrument-only laptop adaptation.
- cache_handoff_v2.py for future idle-cache transfer; it invalidates local workspace artifacts across5profiles and retains third-party dependencies.

Laptop needs Git, Rust+wasm32-unknown-unknown target, Cargo, Trunk, wasm-bindgen/wasm-opt as required by the pinned project, Node/npm, Python3, headed Chrome/WebGPU, native Bevy/audio development libraries, ffmpeg/libvorbis for future assets, and tar+zstd. Use package locks. Do not copy secrets, global Codex/Claude settings or tool authentication. Install node dependencies in the lane with npm ci --ignore-scripts --no-fund from tests/browser when needed.

## CI and remaining stops

Last hosted Wave5 sourcead0eef46 run41300331651: agents SUCCESS; check FAIL on removed economy_ui.rs registry anchor; browser FAIL timeout waiting for Command Center selection and rebuilt owner economy panel after5000ms, last phaseJoinedslot0. Both are approved Task12-to21 red window, owner21, under O13. No Wave6 CI push/read has occurred. Draft PR2 is open/unmerged: https://github.com/26medias/TuRTS/pull/2. Remote integration was last pushed atd8255b257498470d152cab00832652d36fa5d3e8 before later Task5/Task13/vault work; fetch/verify remote before any force-with-lease. The bundle is authoritative for this handoff.

Stop only for brief section4 narrowed conditions: frozen change; suspect Decided/K call; player-visible design change; global170 or task4x budget; Task23 URL/human play; plant retirement; main merge. Root decides listed same-shape/instrument conflicts and records C rulings. Full implementation scope remains all25 lanes, all acceptance rows,0legacy bridge rows, measurements, humanPT1/PT2 and all3hostedCIjobs green. Never mark done from partial gates.
