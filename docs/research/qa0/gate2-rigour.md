VERDICT=fail BLOCKING=1 HIGH=1 RUNS=10

# Gate 2, rigour lens: `docs/superpowers/plans/2026-10-29-qa0-instrument.md` (P below)

Design: `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md` revision 3 (D below), at commit `bfef807`. What I changed in the repository: none.

## Blocking

B1. P:193 (T2 step 4) and P:249 (T3 step 5c). These trust runs restore files with `git checkout --` before the task commits (P:204, P:257). That destroys the work of the task, and the expected `git status --porcelain tests/qa` result cannot occur. Run G1 shows both failures in a scratch repository:
- T2 (b): `cases.tsv` is a new, uncommitted file. `git checkout -- cases.tsv` fails ("pathspec ... did not match", exit 1). The file stays header-only, and `porcelain` shows `?? cases.tsv`. Step (c) has the same problem.
- T3 (c): `t7.js` is tracked, but its role line came from step 1 and is not committed. `git checkout --` reverts the file to HEAD, so the role line is lost. Step 7 then goes red on DR1.
- Fix: commit the task files before the trust runs. As an alternative, save a copy with `cp` and restore it from that copy. Also limit the `porcelain` check to the files that the run changed.

## High

H1. P:296 and P:299 (T4). `released_list_ends_with_engine_version` is listed as an expected red in step 3. `RELEASED_ENGINE_APIS` is added only in step 4 (P:300), so in step 3 `cargo test -p turts_mod_api` does not compile. P:36 says that a compile error is not a red. The real red for A17 is `t2c_package_rejects.rs:57` (it asserts "0.2.1" today). Fix: label the list test as a guard. As an alternative, add the constant before step 3, with a test that asserts `ENGINE_API_VERSION == "0.2.2"`.

## Medium and low

M1. P:287, P:422. SC2 is counted as closed ("A19 (SC2)"). P:299 and Risk 4 (P:434) say correctly that no O1 build can make it red. Fix: in T4, write "A19: guard, not a test", and keep it out of the closed list.

M2. P:160, P:180-182. In `--enforce`, no self-test case shows R1 to R3 on a `client: yes` spec. Only T6 row 20 does, one time (P:404). If a bug makes `--enforce` skip R1 to R3, CI stays green until the next milestone spec arrives, against S:721. Fix: add a `dirs/r1r3` enforce-dir case, a `client: yes` spec without coverage, with the set {R1, R3}. Set `MIN_CASES` to 21.

L1. P:157. The QA0 row `gate: gated` departs from the D:154 table, which says `draft`. It has no effect on behaviour: the tool reads `gate` only for the exempt files. Add the change to "Doc updates" (P:439-448).

L2. P:223. The reason given for `m3-human.js` under DR6 ("A1 through `m2.js`") is wrong. The file has its own Part A items: A7 `node:fs` (`tests/browser/m3-human.js:5`) and A6 `playwright-core` (`:6`). The result does not change.

L3. Resolved: an item that P:463 marks NOT TRACED. G2: `turts-pack` packs `zeta` with an extra map file that the manifest does not export: exit 0, and the file is in the tar. So the T5 Library packages are safe to keep `maps/arena.ron`.

## Lens checks: pass

- (1) Mapping. A1 is in T1. A2 to A5 are in T2. A6 and A7 are in T3. A9 and A16 to A19 are in T4. A10 to A15 are in T5. A20 and A21 are split in two halves, and P says so. A8 is deferred. SC1 to SC4 and DR1 to DR7 / A1 to A7 each have a case (P:233-237).
- (2) Self-test. G3, on my model with R1 to R9 and a minimum of 20: an empty file is red (conditions 1, 2, 3); a file with no M4 row is red (condition 1 m4, condition 3 19<20); the full file is green. This agrees with P:193 (b) and (c).
- (3) Plan choices.
    - Probes at `docs/qa/probes/`: D names no path, so there is no contradiction. The probe copy is one addition beyond the must tier; P:60 and P:87 explain it.
    - `zeta`: `^0.2`, one map, `capabilities: []` (`crates/turts_pack/tests/fixtures/zeta/manifest.ron`). This agrees with P:331.
    - R4 to R8 for no-client cases in spec mode: D does not say how spec mode finds the client value, so there is no contradiction.
    - The duplicate check before the unknown check is sound. With an empty list, both tests can go red.
- (5) Waves. T1 and T4, and T2 and T5, have disjoint Files lists. `ci.yml` is in T2 and T3 only, in waves 2 and 3.
- (6) Budget. 6 x 3.75 = 22.5, and 7 x 3.75 = 26.25. This agrees with P:59 and D section 13.
- T3 expected sets: the 20 evasion sets at P:233 equal `repair3-driver.txt` (G4).

## What I ran

| # | Command | Claim tested | Refuting result | Observed |
|---|---|---|---|---|
| G1 | scratch git repository: uncommitted edit, then `git checkout --` | P:193, P:249 restore steps work | file restored, porcelain empty | role line lost; new file: exit 1, `?? cases.tsv` |
| G2 | `turts-pack` on `zeta` copy with an unexported map file | P:463 NOT TRACED | exit 2 | exit 0, file in tar |
| G3 | `selftest3.py` on empty, no-M4, full (`gate2/selftest.txt`) | P:193 (b), (c) | empty or no-M4 green | both red; full green |
| G4 | read `repair3-driver.txt` against P:233 | T3 expected sets | a set differs | equal |
| G5 | grep `m3-human.js` Part A words | P:223 reason | only via `m2.js` | direct A6 and A7 at `:5-6` |
| G6 | `cat` zeta manifest | P:331 | other fields | as stated |
| G7 | `sed` on `cli.rs:99`, `bad_packages.rs:95-96`, `t2c:57`, `lib.rs:3-11`; `ls` tests | T4 and T5 Files lines | a moved line | as cited |
| G8 | grep D for `NO_CLIENT` and the exempt set | P:158, P:160 | D keys them by milestone | both by file path (D:193, D:212-215) |
| G9 | read `no_content.rs:100-110` | probes under `docs/` break the no-content scan | the scan reads `docs/` | it reads `mods/` and a temporary `src` |
| G10 | arithmetic | P:59 | sums differ | 22.5, 26.25 |

## NOT TRACED

- Python version on the CI runner.
- Run times of the new Rust tests.
- Whether other `turts_checks` tests scan the whole repository: I read only `no_content.rs`.
- Whether the LR1 and LR2 tests in `resolve.rs` use words that LR3 does not print. If a test only checks the field `dependencies`, then removing the rule makes LR3 fire on the same field, and the test stays green.

---

# Re-check, plan revision 1 (2026-10-29)

VERDICT=pass BLOCKING=0 HIGH=0 RUNS=3

What I changed in the repository: none.

| Finding | Status | Evidence |
| --- | --- | --- |
| B1 | Closed. The execution contract (P:37) commits the task files before any trust run. T2 commits at step 4, before the step 5 trust runs (P:197-198). T3 commits at step 5, before step 6 (P:254-255). T1 restores with `cp` (P:136). The `porcelain` check covers only the changed files. | Run R1 (`gate2/repo2`): commit, edit, `git checkout --`: both exit 0, both `cmp` exit 0, porcelain empty. |
| H1 | Closed. P:36 adds the "signature first" rule. T4 step 2 adds `RELEASED_ENGINE_APIS` (last entry 0.2.2), keeps the engine at 0.2.1, and adds a stub `check_version_table`. P:308 names `t2c:57` as the A17 red. `released_list_...` and `sc3_..._refuses` compile and fail. | P:302-308 |
| T5 stub (new) | It compiles: `PackageId(pub String)` (`crates/turts_mod_api/src/ids.rs:10`). The LR tests and the order test fail on the stub. T6 row 22 removes `lr3_provided` for A13. | R2 |
| SC2 | Closed. P:293 says "guard, not a test", and A19 is not in the closed list. P:437 gives A19 the red "none". | |
| `dirs/r1r3` | Closed. It is row 21 (P:184-186), and `MIN_CASES` is 21 (P:159). Trust run (e) and T6 row 23 make it go red (P:198, P:419). | |
| QA0 `gated` | Closed. Doc update 9 (P:464); doc update 10 records 21 rows (P:465). | |
| DR6 reason | Closed. P:228 names A7 and A6 at `m3-human.js:5-6`. | |

New defects: none found in the changed lines.

Runs:
- R1: the scratch repository above.
- R2: `ids.rs` grep for the stub signature.
- R3: grep of the plan for the commit steps.
