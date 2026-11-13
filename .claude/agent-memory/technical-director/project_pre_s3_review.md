---
name: pre-s3-review-facts
description: S3 movement split review facts (a6f3f66): plan plant names the wrong test, plan regex, runner log paths, instrument traps
metadata:
  type: project
---

Verified 2026-11-12 on a6f3f66 (parent 85f34cb):
- The plan Step 4 plant (`let push = Vec2::ZERO;`, frame.rs:835) goes red only in `group_movement second_group_same_point` (panic group_movement.rs:1181). The test the plan names, `forty_units_rest_with_no_overlap_and_no_blocked`, stays green on both builds. Same failing set on both.
- Plan Step 2 regex `^(pub )?(fn|struct) ` gives 1 on plain.rs; `^(pub(\([a-z]+\))? )?(fn|struct) ` gives 3.
- Runner stdout lives in `target/mc-evidence/<label>/command.log`, not in the wrapper output. `grep` is rtk-rewritten in the shell: use `rtk proxy grep` for files you redirect.
- Whitespace-stripped item compare hides line wraps; only a trailing comma differs (spread.rs:100). A statement swap shows at item level only, not at line or char level.
- 34 `pub(super)` all needed (strip-and-compile in 3 stages: items E0603, fields and methods E0616/E0624, struct literal E0451 appears only after typeck passes).
- Public surface of `crate::movement`: 23 paths before and after; 20 private names fail at both (probe appended to lib.rs).
- Stale `movement.rs:NNN` cites remain in crates/turts_sim/tests/gmu_same_point.rs:6, crates/turts_server/tests/mc_orders.rs:578, tests/m3/README.md:110, tests/m3/refreeze_classify.py:78.
