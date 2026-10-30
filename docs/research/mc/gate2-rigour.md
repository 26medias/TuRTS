VERDICT=fail FINDINGS=16 HIGH=3 RUNS=14 NOT_CHECKED=5

# Gate 2 rigour review: docs/superpowers/plans/2026-10-29-mc-player-control.md (f69badf)

Mode: review-plan. Design of record: D revision 6 (unchanged 0b1a250..f69badf: `git diff --stat 0b1a250 f69badf` touches only the plan).

## Findings

### High
H1. plan:519-523 (task 5, ORD1), plan:564,567 (task 15, ORD6 headless half). `crates/turts_pack/tests/mc_orders.rs` calls `orders::to_sim` and checks the version 3 Hello. Both live in the `turts_server` binary. `crates/turts_server/Cargo.toml` has no `[lib]`, `crates/turts_server/src` has no `lib.rs`, and `turts_server` depends on `turts_pack`. The test cannot compile, so the ORD1 red ("rush_order_attacks fails") and plant (b) ("drop the Job arm") cannot run. The ORD6 bucket half in `mc_feedback.rs` has the same defect (buckets are in `session.rs`, task 7). Fix: put the ORD1 tests in `crates/turts_server/src/orders.rs` `mod tests` (`cargo test -p turts_server --bin turts_server`), reuse `protocol_3_hello_is_refused` (plan:321), and point the ORD6 headless half at the task 7 bucket tests (plan:441).
H2. plan:388 (task 18, MOD1). `old_acme_copy_loads_with_no_orders` asserts the client notice "This game defines no orders" (D:1040, D:1216). No task file list implements that notice (grep: only plan:37, plan:388). Task 18 (wave 2, mod-builder, files = mods + mc_packages.rs) cannot reach green. Fix: assign the notice to a client task (9 or 15). Task 18 asserts only "loads, zero OrderDef".
H3. plan:49 versus plan:506-509 (task 12: plant step 3, "Commit" step 4) and plan:555-556 (task 14: plants step 4, "Commit" step 5). The contract restores a plant with `git checkout -- <file>`. On an uncommitted tree that discards the task's own work (for example `smart.rs` in task 14). Tasks 5, 8, 9, 13, 15, 16, 17, 18, 21 say "after the commit" but have no commit step before the plant. Tasks 2a, 10, 11, 19 put "Gate ... Commit" in that order, which contradicts plan:55 ("after the commit"). Fix: one fixed step order in every task: red, green, commit, gate, plants.

### Medium
M1. Red legs that the plan claims but never runs. plan:218,219,224: KM1 "no store write", KM2 "no overlay", and SEL4 "groups not stored" are "first". Task 16 step 1 (plan:610) runs only SLOT1 and PER1. CAM1 (plan:494): the only red shown is "camera did not move". The PointerLeft leg (D:1199) is never red. MM1 (plan:552): the red is "no minimap widget", not the design red "the minimap click does not move the camera" (D:1206). HD1 (plan:253): the table cites `lib.rs:278-280`, which task 9 changes (plan:451). The red shown is "unknown option --actions".
M2. Needs column gaps (plan:136-139). Task 10 (wave 5) runs "one action per behaviour on muster" (plan:480), but Attack arrives in task 5 (wave 6). HD1 then compares `not_applicable` with `not_applicable`. If `hd1.rs` pins results, task 5 G-ACC goes red on a file that task 5 does not own. Task 15 lacks need 4: the ORD2 plant restores the whole-list return of task 4 (plan:569), and ORD7 needs `SimSet::Orders`. The waves cover 4 but not 5.
M3. plan:342. Task 3 step 8 may be skipped ("go on with the rest"). No later task owns `RESERVED_BROWSER_KEYS` (a task 3 file).
M4. plan:457. The PROJ1 plant stays green if the core field of view equals the Bevy default. No step pins a non-default field of view. A missing copy fails every viewport, not only 1920 x 1080.
M5. plan:248,539. "The row check reads mc.md": no file and no command implement it, and no step shows it red.
M6. plan:647. The PT1 plant (guide page 5 removed) goes red only if `pt1-oracle.js` has a guide step. Task 21 (plan:626) does not require one.
M7. Doc updates (plan:683-693) are incomplete. Not listed: the CLAUDE.md `--move` text ("Send one MoveTo"; `options.rs:119` changes in 2b); the `--out` and `command_results` shape (`accepted`, `refused`, R5 keys); the `turts_move` semantics; the `turts_sim`, `turts_server`, `turts_mod_api`, and `turts_mod_runtime` crate rows (`orders.rs`, `grid.rs`, buckets, `content_mc_validation.rs`, `asset_headers.rs`); the turts-pack K18 warnings with exit 0 and the K12 caps.
M8. D:567-568: "EL1 records the real figure" (log size). Task 22 step 4 (plan:596) has no record step.

### Low
L1. plan:261 "D:1442-1632" is D:1445-1636. plan:643 "D:1634-1676" is D:1637-1679.
L2. plan:340: ":374" and ":424-427" point to `content_m3_validation.rs:375,427`, not `tests/content_m3.rs`.
L3. plan:686: the D65, D67, and D73 lines are already Decided (spec:94,96,102). Say "design part only".
L4. plan:373,507 use `rtk proxy grep` in Codex steps. Codex may not have rtk.
L5. plan:160 lists 2b for `index.html`, but the 2b file list (plan:312) does not include it.

## Positive results (each tested)
- Ownership: 42 rows in D section 14, 42 plan rows, each with exactly one owner. Each task "Owns rows" line agrees with the table. The 81 `in` coverage rows map to 27 tests, and plan:261 matches each row. (own.py)
- Waves agree with Needs. The critical path is 51 (3,18,2b,9,11,13,15,16,21,23,24). The subtotal is 106. (deps.py)
- Numbers match the design: 4,096; 64/640; 16,384/163,840; 2,048 per tick; 16,384 pending; queue_depth 1..255, default 16; s, 0.05, 0.025, 1 s, 3 claims; R_near 4.58; protocol 4; API 0.2.3; the K12 and K18 values; the client numbers; muster has 515 units.
- I sampled about 75 citations. All hold except L1 and L2 (cites.txt, cites2.txt).
- The frozen stream has no version or hash field, so the package version bumps do not touch it.
- The driver rule walks subdirectories (driver_rule.py:288), so task 20 step 1 can go red. The DR7 text matches driver_rule.py:309,351.
- The Bevy 0.20 feature `audio` includes `vorbis`.

## What I ran
| command | claim tested | refute result | observed |
|---|---|---|---|
| yardstick.py --check | G-QA plan:58 | exit!=0 | "yardstick matches inventory" exit=0 |
| completeness.py --self-test; --enforce | plan:58 | other text | 21/21; "checked specs: 2"; exit=0 both |
| driver_rule.py --self-test; tests/browser | plan:58 | exit!=0 | 46 cases PASS; exit=0 both |
| bash scripts/boundary.sh | plan:375 line format | other format | 11 checked lines, boundary ok, exit=0 |
| own.py | plan:212-261 owners and coverage | missing or double | none; 42/42; 81/81 |
| deps.py | plan:116-142 | wave order break, path not 51 | none; 51; 106 |
| sed batches (cites.txt, cites2.txt, more) | plan path:line | moved line | 2 stale (L1, L2) |
| ls / cat turts_server Cargo.toml, src | plan:521 test placement | lib target exists | bin only (H1) |
| grep "defines no orders" | plan:388 owner | a client task implements it | none (H2) |
| grep design numbers | plan:23-26 | mismatch | none |
| git diff --stat 0b1a250 f69badf | design unchanged | design changed | plan only |
| JSON scan of refutations | plan:65-80, risk 2 | other files/counts | 41/17; table matches |
| grep bevy-0.20.0 Cargo.toml | plan:28 audio | no vorbis | audio = bevy_audio + vorbis |
| sed driver_rule.py | plan:355,630 red text | no recursion, other text | recursive; DR7 text matches |

## Not checked
- The lightyear message size (plan:320): NOT TRACED.
- Whether HashMap neighbour order changes the tick log bytes (DET1 red, plan:593): NOT TRACED.
- What completeness.py enforces for a `done` MC row (plan:689): NOT TRACED.
- Whether the "M2 browser proof" red cause is fixable in task 21: NOT TRACED (plan:88).
- How tasks 13, 15, and 16 fill debug keys without an edit to `debug.rs` (R11): NOT TRACED.

## What I changed
none (I wrote this report to the scratchpad only)
