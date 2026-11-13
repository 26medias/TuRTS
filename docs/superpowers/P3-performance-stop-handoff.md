# P3 handoff: performance stop

2026-11-13. Branch: codex/p2-p3-presentation. Engine checkpoint: 6d5a92a0.
Main was not merged or pushed.

The CEO requires a stop and report below 30 FPS at 400 detailed units.
The first P3 High combat run measured 26.67 FPS at 400 units. Feature work stopped.
The cause has not been profiled. Do not claim a cause yet.

## Playable build

/home/julien/Projects/TuRTS/target/mc-evidence/play-skirmish.sh still opens the accepted
P2 animation build from the presentation-play worktree. P3 has not replaced it.
P2 is complete. Its acceptance, review, measurements and player probes are in mc-evidence/p2-*.

## P3 code present

API 0.2.7 adds composed effect layers, costs, projectile/work/site refs, damaged lists,
and offscreen-only alerts. Pack validation bounds layers, particles, lifetime, costs, local
images and reference cycles. The first shared particle renderer runs in Chrome.

The separate combat timeline holds visible health until impact, up to 300 ms. Overlapping
hits release separate health values. Snapshots clear pending hits. Pending deaths lose
interaction identity immediately. Shared white materials give 100 ms damage flashes.
Damage, work and site conditions feed effects. Alert filters run before cooldowns.
Clicks focus the camera; J focuses the latest warning. Space still centers selection.
Flashes and shake settings persist. Shake rendering is not built.
Skirmish has initial effects and warning declarations.

## Checks actually run

Evidence paths below are relative to target/mc-evidence/.

- p3-effect-contract-tests.log: 3 pack tests pass. Legacy kinds, limits, cost, cycles and API refusal.
- p3-alert-core-tests.log: 3 tests pass. Remembered warning, click selection and invalid input.
- p3-fx-initial-tests.log: 5 tests pass. Flight clamp, held health, overlapping hits, snapshot reset,
  and the bounded full Low pool.
- p3-first-effects-smoke/result.json: Chrome smoke passes. Shots, pending impacts, flashes,
  and removal were seen. Peak 18 particles. No shader validation error. One HTTP 404.
  This is not the complete FX3 pixel acceptance test.
- p3-first-effects-smoke-run/receipt.json: command/wrapper 0; source unchanged.
- p3-first-contract-suite.log: 43 runtime unit tests pass; 3 old bookkeeping tests fail.
  The run stopped there. This is not a full passing contract suite.
- p3-first-measurement/result.json: High 200 units 50.12 FPS; High 400 units 26.67 FPS.
  Every soldier stayed visible and shooting. Burning row rejected: only 4 of 10 buildings survived.
  Low and 800-unit rows did not run. docs/measurements/models.md records the partial results.

Every test Chrome used a fresh profile and --alsa-output-device=null.
No P3 independent review or final cold-player acceptance run has passed.
Do not present this P3 build as accepted.

## Remaining work

1. Address the CEO's performance stop before resuming feature work. Profile the 400-unit cost.
   CPU allocation and per-frame debug serialization are hypotheses, not findings.
2. Retain the health bar for pending lethal impacts. The model waits; its bar currently disappears.
3. Replace the full source-to-target fallback line with a moving bolt and real trail.
4. Render decals, optional images/models/lights, shake, and effect sounds. These are incomplete.
5. Reserve particles for whole effects. Finish distance/cost priority, per-cost drop counters,
   and the critical sprite fallback. The current emergency hit is a cross.
6. Start re-entering offscreen loops in steady state. Tie work chips to a work cadence.
7. Draw the warning edge pulse. Run offscreen/onscreen/cooldown/J/click browser checks.
8. Resolve dependency effect refs and check dependency image bounds. Count layer delay in costs.
   Add image-size and damaged-list version tests.
9. Remove obsolete exact task-owner/count bookkeeping tests in unbuilt.rs. Keep semantic refusals.
10. Repair the burning fixture so ten damaged buildings survive throughout sampling.
11. Finish FX1-FX13, High/Low measurements, the one required independent P3 review and fixes,
    cold player probes, and a fight win. Then replace the launcher.

## Contract choices

Reuse OwnUnitDamaged with unit filters instead of redundant attack triggers.
J is used because Space is taken. Decals may last 120 seconds: the generic 10-second layer
cap conflicts with the required 60-second scorch. Other non-looping layers keep the 10-second cap.
No optional sound art was added. Sound stays off by default.

## Build and disk

Draft artifacts: target/mc-evidence/skirmish-effects-first-prepared.
Its manifest binds 6d5a92a0. Later driver/docs commits do not change the engine source.
Refresh prepared metadata before a strict final run.

Active native cache: /home/julien/Projects/TuRTS-mp1-integration/target/mc-mp1-integration.
Active WASM cache: /home/julien/Projects/TuRTS-models-stage1/target/mc-zoom-web.
Keep these if work resumes. Keep target/mc-evidence. Disk had about 250 GB free before this build.
Use /tmp/codex-mp1-cargo for native builds with the existing ALSA sysroot.
Wait for the whole Trunk command, including wasm-opt and distribution.
Never merge or push main without fresh CEO authorization.
