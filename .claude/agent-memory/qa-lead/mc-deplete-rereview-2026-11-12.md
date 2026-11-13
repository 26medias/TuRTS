---
name: mc-deplete-rereview-2026-11-12
description: Re-review of codex/mc-fix-deplete 22eca1c (blocked gather trip waits and retries) - plants, free_approach unpinned, validator cargo-loss gap, no HUD signal
metadata:
  type: project
---

Verified 2026-11-12 at 22eca1c (worktree TuRTS-mc-deplete-review, cache name `mc-deplete-review`;
the runner `--target` takes the NAME, not a path; an absolute path fails its regex assert).
Scratch copy via git archive + copied target/debug (12 GB, 15 s) gives fast plant builds.

- Gates at 22eca1c: fmt 0, clippy 0, sim 327 passed, server 444 passed, frozen+rows+accounting
  78 passed, boundary 0, frozen sha bcbf72c2... equal. driver_rule self-test 47 PASS.
- In-process ORD3 full scene: node empty t32, node_depleted t86 (pruned: t74).
- Plants (scratch, cmp-restored): retry never ends -> red 3 tests (gather_depletion:419,
  mc_order_rows:1168, server economy.rs:707). Retry clears cargo -> red sim :341 and rows :1174
  but server ord3 validator test GREEN (validator has no conservation check on Other-phase ops).
  free_approach picks covered point, or always direct -> ALL committed tests GREEN; my scratch
  two-gatherer + one-blocker case goes red under both. Sim reverted to Blocked failure ->
  validator red at economylog.rs:15486 ("differs from its independently replayed wait").
- Scratch cases: ringed depot -> Waiting(Blocked) for ever, cargo kept, 219 retries/3000 ticks;
  a gatherer idle after node_depleted can park on the only free point and strand the other;
  straight-line movement makes free point unreachable -> 81 ticks MovingToReceiver standing still.
  Stop/Move during wait clear it, cargo kept. Node-side wait resumes. Replay bitwise equal.
- No client HUD text for any gather wait reason; no mod declares GathererWaiting alert.
- AccountingOracle (turts_pack tests/common/accounting.rs:2399,2456) does not model
  Waiting(Blocked); probe found 0 blocked gather trips in frozen, accounting, packages, E9 base.
- m2-audit-copy.js is a driver-rule fixture; equals 43282fe m2-audit.js; no check compares them.

- ORD3 headed at 22eca1c: both runs FAIL at driver step 1 "no exposed trooper column" (Click
  to play overlay). With the 4-line driver fix of c097ff9 (hud2 lane, F9 key first) applied dirty:
  both runs FAIL earlier in the oracle at "actual current A matches the authored target" (also
  seen on hud2 lane), but the gather checkpoints show: accepted t1355/t1604, node_depleted
  t1440/t1689 (85 ticks), notice visible, cargo 0; failure 24-32 ticks into the 6000 ms wait.
- GPU queue: flock is not FIFO; my run waited about 60 min behind 4 lanes.

Related: [[mc-deplete-review-2026-11-11]].
