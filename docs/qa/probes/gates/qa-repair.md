DONE items=12/12 acceptance=21 runs_must=22-26 runs_all=30-38

# QA0 design, repair pass (revision 1)

File changed: `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md`, the only repository file.
Memory: `.claude/agent-memory/qa-lead/qa0-design-2026-10-29.md`, updated.

## The 12 decisions

| # | Where in the design | Note |
| --- | --- | --- |
| 1 | 5.2 third test; A12 | The archive is a Library with zero maps. The test calls `load_package` and checks the exact text. Today: `lib.rs:16-23`. |
| 2 | 7 call sites; A16 | No client `resolve`. `check_lockfile` checks one entry per id and that `mod_id` is an entry. S:302, `acquire.rs:60-65`. |
| 3 | K1; 6 | O1 is the recommendation. O2 is the alternative, with its measured cost. A19: the ron derive rejects a duplicate (`repair-modrt.txt:6`). |
| 4 | 6.1; SC3 | "Matches a version below since_api" is defined over a finite released list (0.1.0, 0.2.0, 0.2.1, from `git log -G`). The check goes in `content()` after `validate_m3`. |
| 5 | 1.2; 7 | `--lib` is cut. Rust tests replace it: `resolve.rs`, a `turts-pack` process test, a server boot test. `turts-pack` calls `resolve(root, [])`, so it refuses what the boot refuses. |
| 6 | 8 | 0.2.1 to 0.2.2, a patch bump. The lockfile format and the hash rule do not change. One test literal changes (`t2c_package_rejects.rs:57`). QA0 adds no manifest field: two manifest values become valid, and the table gates both. |
| 7 | 3.1 `specs.tsv`; 3.4 | Columns `client` and `gate`. Enforce runs on every `client: yes` spec except a closed exempt set (M1 to M4 while not `gated`), which is fixed in the tool. |
| 8 | 3.4 | Set equality on rule ids. An empty or incomplete cases file fails. **Reading:** "gate not done" taken literally leaves M1 to M3 out (they are `done`), but S:711 needs them red. So the design reads it as "every exempt spec". Please confirm. |
| 9 | 4.1 to 4.4 | One role per file. Oracle defined, DR6. `t7.js` is replaced by `t7-driver.js`, `t7-oracle.js`, and `harness/run.js`. |
| 10 | 4.4 DR2, DR3 | Banned: `eval`, `Function`, `globalThis`, `process`, `fetch`, `import(`, string-built and aliased `require`. Also `constructor` and `module` (model run). |
| 11 | 10; 13 | Two tiers. Must: 6 tasks, 22 to 26 runs. All: 8 tasks, 30 to 38 runs. |
| 12 | 2; 14 | Receipt replaced (`repair-c1.txt`). New names: P1 to P8, SC1 to SC4, DR1 to DR8, LR1 to LR8, "coverage probe", "bridge probe". STE splits. |

## New fact found during the repair

The `turts_scene` bridge already gives `clip_from_world` and the viewport
(`crates/turts_client_web/src/lib.rs:471-479`). So the T7 oracle needs no client change. `CLAUDE.md`
describes this bridge as `{"cameras_3d": n, "units": [..]}`, so that text is out of date.

## Runs

| Command | Claim | Refuted if | Observed |
| --- | --- | --- | --- |
| Coverage probe, no pipe (`repair-c1.txt`) | A2 to A4; rule sets of 3.5 | an exit or set differs | M1 to M4 {R1,R3} exit 1; green exit 0; mutations exit 1 |
| Self-test model (`repair-selftest.txt`) | 3.4 conditions | empty, subset or incomplete file is green | all red |
| Driver model (`repair-driver.txt`) | DR2, DR3 close the evasions | an evasion is green, or `ok-driver.js` is red | 10 red, 1 green |
| Mod-runtime probe re-run, checked with `cmp` (`repair-modrt.txt`) | K1, SC2, SC3 | output changed | identical |
| `git log -G ENGINE_API_VERSION` | released list | another value | 0.1.0, 0.2.0, 0.2.1 |
| grep "0.2.1" literals | 8: one test literal | more | one |
| read `acquire.rs`, `client_web/lib.rs:405-480` | 7, 4.6 | client graph; no camera in bridge | no graph; camera present |
| STE sentence-length script | under 25 words | over | 3 real cases fixed; the rest are lists |

## NOT TRACED

Run times on CI. The runner Python version. Whether a colour search finds the unit under Xvfb with
Vulkan. The Node permission model on Node 20. O2 on `Exports`. The context of D48 libraries.
