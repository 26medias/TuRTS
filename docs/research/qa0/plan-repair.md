DONE items=6/6 tasks=6 runs_est=22-26

Repair of `docs/superpowers/plans/2026-10-29-qa0-instrument.md` (revision 1 after gate 2). What I changed: that plan only (plus my memory notes).

## Item by item

1. Rigour B1. Execution contract: in-tree trust runs and mutation runs come after the task commit; restore with `git checkout -- <file>` (`rm` for a created file); porcelain checked on the changed files only. T2: new step 4 "Commit" before trust runs; CI edit committed in step 8. T3: new step 5 "Commit" (role lines included) before trust runs; CI in step 8. T1 step 7: the TSV is uncommitted, so save and restore with `cp`.
   Scratch-repo check (`qa0/repair-g1/`): commit new `cases.tsv` and a role-line edit of `t7.js`; edit; `git checkout --`; `cmp` with saved copies. Result: both checkouts exit 0, both `cmp` exit 0, porcelain empty. Control (old plan: `git checkout --` on an uncommitted new file): exit 1.
2. Mod-runtime B1, rigour H1. Contract rule "signature first". T4 step 2 adds `RELEASED_ENGINE_APIS` (0.2.2 last) and `KNOWN_CAPABILITIES`, keeps engine 0.2.1, adds a stub `check_version_table`. A17 red is `t2c_package_rejects.rs:57`; `released_list_...` and `sc3_..._refuses` now compile and fail. T5 step 2 adds `pub mod resolve` with a stub that returns the root: t2c fails on "not supported at M1", the 8 LR tests and the order test fail on the stub. T6 row 22 (A13: remove `lr3_provided`).
3. Mod-runtime M1. T5 Files: `load_package` call stays in a `match` arm of `check_package`, no helper (`m2_public_path.rs:36-39`).
4. Mod-runtime M2. T4: version-table error is `ModError::new(<id>, "engine_api", ..).with_file(MANIFEST_PATH)`.
5. Mod-runtime L1. T1 receipt starts with `git rev-parse HEAD` and `ENGINE_API_VERSION`; the expected p1/p7 exits on 0.2.2 are stated.
6. Rigour medium/low. A19 marked "guard, not a test", out of the closed list and in a separate T6 row. `dirs/r1r3` case row 21, `MIN_CASES = 21`, T6 row 23. Doc updates 9 (D:154 `gated`) and 10 (21 rows). `m3-human.js` reason: A7 and A6 at `:5-6` (verified by read). Unexported map file: moved from NOT TRACED to a T5 note. Also: each LR test asserts a rule-specific word (rigour NOT TRACED item).

## NOT TRACED

CI runner Python version; run time of new Rust tests; whether other `turts_checks` tests scan the whole repository.
