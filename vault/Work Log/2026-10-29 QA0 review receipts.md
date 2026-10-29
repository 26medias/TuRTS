---
type: log
date: 2026-10-29
tags: [turts, qa0, review]
ai-first: true
source: codex
---

# QA0 review receipts

These are the independent Sol reports. Each section records the reviewed source and later scoped checks. The final verdict for each accepted task is PASS. Long command output stays in the scratch paths that each report names.


## Task 4

VERDICT=pass-with-changes FINDINGS=1 HIGH=0 RUNS=10 NOT_CHECKED=2 ROLE=qa-lead

## Findings

1. Medium, required: `crates/turts_mod_runtime/src/version_table.rs:45` adds backticks around the admitted version; `:85` requires that wrong text. This conflicts with `task-4-brief.md:24`. Remove those backticks from the diagnostic and its test. Assert the complete diagnostic, including package, file, field, and unquoted admitted version, so T5 can use its specified text.

The rest meets T4 scope. `crates/turts_mod_runtime/src/manifest.rs:44` checks duplicates before unknown capabilities. `crates/turts_mod_runtime/src/parse.rs:20` limits hints to fields; `:27` limits distance and `:28` preserves first-name ties. Exact capability, typo, variant, and lockfile texts pass their tests. `crates/turts_mod_runtime/src/archive.rs:134` calls the table after M3 validation; `crates/turts_mod_runtime/src/version_table.rs:12` has the required empty real table. `crates/turts_mod_api/src/lib.rs:33` sets API 0.2.2. The six original callers and two new callers pass the id. All 46 schema closures remain byte-identical to the base. SC2 remains a guard, as `task-4-brief.md:27` requires.

## What I ran

All commands used the assigned review target, debug=0, incremental=0. Logs: `/tmp/turts-qa0-t4-review/`. `cargo test` commands used `--locked`.

| Command | Claim / refutation | Result |
| --- | --- | --- |
| `cargo test -p turts_mod_api` | `crates/turts_mod_api/src/lib.rs:46` release/API agreement; mismatch refutes | green exit=0 |
| `cargo test -p turts_mod_runtime --test t2c_package_rejects --test archive` | brief:33-34 diagnostics; failed assertion refutes | green exit=0 |
| `cargo test -p turts_mod_runtime --lib version_table` | brief:35 SC3; failed assertion refutes | green exit=0 |
| `cargo test -p turts_pack --test bad_packages --test cli` | brief:32,37 refusal/help; wrong text refutes | green exit=0 |
| `cargo test -p turts_client --test acquire --test t2d_client_hash` | `crates/turts_client/src/acquire.rs:63` uniqueness/root/hash; acceptance refutes | green exit=0 |
| `python3 /tmp/turts-qa0-t4-review/trace.py` | callers, constants, 46 closures; includes `cmp` | green exit=0 twice |
| SC3 command, scratch source with comparison removed | tests detect missing version bound | red exit=101, both fail |
| SC3 command, original source | cache reused scratch artifact | red exit=101 |
| SC3 command, original after package-cache clean | original code passes | green exit=0 |

## Not checked

NOT TRACED: full five gates, owned by builder; mutations of other rules.

## What I changed

none. No product source was changed. The report and scratch evidence are the requested review outputs.

## Open questions for the orchestrator

None.

## Scoped recheck — Run 4, 547014a

VERDICT=pass FINDINGS=0 HIGH=0 RUNS=1 NOT_CHECKED=0 ROLE=qa-lead

**ADDRESSED.** `crates/turts_mod_runtime/src/version_table.rs:45` now prints the admitted version without backticks. The exact assertion at `crates/turts_mod_runtime/src/version_table.rs:82` checks the complete prescribed diagnostic, including package, file, field, requirement, and unquoted version. The correction diff changes only this text and its assertion. No new breakage found within this fix.

`cargo test -p turts_mod_runtime --lib version_table --locked` | claim: both SC3 cases still pass; a failed assertion refutes | **green exit=0**, 2 passed. Assigned review target, debug=0, incremental=0. Log: `/tmp/turts-qa0-t4-review/recheck-547014a.log`.

What I changed: no product source. This append records the scoped recheck.


## Task 5

VERDICT=pass-with-changes FINDINGS=2 HIGH=0 RUNS=12 NOT_CHECKED=2

## Findings

1. `crates/turts_mod_runtime/src/resolve.rs:206` — Medium. DFS violates design 7's id tie order. For `a → {b,c}`, `b → z`, it returns `[z,b,c,a]`; available leaves `c,z` require `[c,z,b,a]`. Select the smallest ready id and add a real tie case; the test at `tests/resolve.rs:122` has no tie.
2. `crates/turts_mod_runtime/src/resolve.rs:17` — Medium. LR3 and LR6 bind values, so T6's `^\s*lrN_` removal matches neither. Removing their actual lines leaves missing variables. LR1/LR2 also have multiple calls (`:11-15`). Put validation in one removable call per rule and keep traversal/index state separate. This meets the task brief and permits semantic reds without compile errors.

## What I ran

HEAD `9fc246c`; one semantic plant removes `crates/turts_pack/src/lib.rs:14` in `/tmp/turts-qa0-t5-review/source`. Separate target subdirectory; debug/test-debug=0, incremental=0, jobs=4, all Cargo commands `--locked`. Logs: `/tmp/turts-qa0-t5-review/`.

Read checks confirm LR7 first (`resolve.rs:10`), shared LR1/LR2 (`manifest.rs:69`), direct Game match-arm loading and world-free Library validation (`turts_pack/src/lib.rs:43-52`), both 0.2.2 rows in required order (`version_table.rs:20`), and exact error texts (`bad_packages.rs:221,267,276,289,303`).

| Command | Claim tested; refuter | Observed exits |
| --- | --- | --- |
| `cargo build -p turts_pack`; separately `cargo build -p turts_server` | Executable paths compile; build failure | source 0/0; plant 0/0 |
| `cargo test -p turts_mod_runtime --test t2c_package_rejects --test resolve` | `tests/resolve.rs:38`, LR messages and dependency lift; failure | green 0, 26 passed |
| `cargo test -p turts_pack --test bad_packages --test cli`; plant: `cargo test -p turts_pack --test bad_packages a15` | `bad_packages.rs:252`, public pack rejection; mutant green | green 0, 21 passed; red 101, named FAILED, package packed |
| `cargo test -p turts_server --test unit_boot`; plant: same command plus `boot_refuses` | `unit_boot.rs:182`, real boot rejection; mutant green | green 0, 11 passed/1 ignored; red 101, named FAILED, boot succeeded |
| `cargo test -p turts_mod_runtime --test resolve review_independent` | design 7 tie order; `[z,b,c,a]` | red 101, Finding 1 |
| Python regex probe; `cmp` resolver source/scratch | removable calls; matching LR3/LR6 lines or differing resolver | 0/0, Finding 2; identical resolver |

## Not checked

NOT TRACED: other rule mutations (T6); full builder gates (separate run).

## What I changed

none

## Open questions for the orchestrator

None. No Decided scope reopened.


---

## Run 10 scoped recheck: 6fc6720

VERDICT=pass FINDINGS=0 HIGH=0 RUNS=4 NOT_CHECKED=1

### Findings

- R1 ordering: **ADDRESSED**. `crates/turts_mod_runtime/src/resolve.rs:178` selects the smallest ready id with `BTreeSet::pop_first`. The new regression at `crates/turts_mod_runtime/tests/resolve.rs:139` checks `a → {b,c}`, `b → z` gives `[c,z,b,a]`.
- R2 removable calls: **ADDRESSED**. `crates/turts_mod_runtime/src/resolve.rs:16` starts exactly one unit-return call for each rule, in order LR7, LR1–LR6, LR8. State computation is separate at `:12`. LR3 and LR6 now return `Result<(), ModError>` (`:85`, `:193`). The LR3 deletion compiles and exposes the missing checks.
- No new defect found in `review-9fc246c..6fc6720.diff`.

### What I ran

Same review target, debug/test-debug=0, incremental=0, jobs=4. Scratch used the isolated `scratch` subtarget. Logs are in `/tmp/turts-qa0-t5-review/recheck-*.log`.

| Command | Claim tested; refuter | Observed |
| --- | --- | --- |
| `cargo test -p turts_mod_runtime --test resolve --locked` | `tests/resolve.rs:139`, fixed ordering and rule behavior; test failure | green exit 0, 13 passed |
| Delete literal `^\s*lr3_` line in `/tmp/turts-qa0-t5-review/recheck-source`; same Cargo command | `resolve.rs:19`, removable semantic validation; compile error or LR3 tests green | red exit 101; compiled; `lr3_rejects_missing_dependency` and `lr3_rejects_duplicate_provided_versions` both FAILED on `Ok` values; 11 others passed |
| Python call-shape assertions | `resolve.rs:16`, one call per rule in required order; another sequence/count | exit 0; `[7,1,2,3,4,5,6,8]`; one LR3 deletion |
| `git diff --check 9fc246c..6fc6720` | fix formatting; diagnostic | exit 0 |

### Not checked

NOT TRACED in this recheck: other seven rule mutations and full five gates; builder runs those separately.

### What I changed

none. Product worktree remains clean at `6fc6720e4883e985fc07e009afd38021dfbac99a`.

### Open questions for the orchestrator

None.


## Task 2

VERDICT=fail FINDINGS=4 HIGH=1 RUNS=8 NOT_CHECKED=2

## Findings

1. **High — `scripts/qa/completeness.py:251`.** Directory cases use `_enforce_cases`, a second implementation, instead of `enforce`. Disabling R1–R3 in the real CLI leaves all 21 self-tests and CI green. The incomplete MC fixture then exits 0. Make both paths call one implementation; row 21 must reject this plant.
2. **Medium — `scripts/qa/completeness.py:203`.** Exempt specs are discarded before R4 collects evidence. Thus an earlier exempt registry spec's `in`/`done:` dependency cannot satisfy a later checked spec. This conflicts with R4 (`docs/superpowers/specs/2026-10-29-qa0-instrument-design.md:188`); spec-mode already collects these rows (`:153`). Collect evidence separately from exemption checks.
3. **Medium — `docs/qa/specs.tsv:6`.** QA0 says `draft`; task-2-brief requires `gated` after its recorded gate pass. Set `gated`.
4. **Medium — `scripts/qa/completeness.py:245`.** Spec-mode skips R1–R3 only for basename `no-client.md` as QA0. The brief requires that policy for any spec-mode QA0 or MS case. Apply the milestone policy in spec-mode without a fixture-name exception.

## What I ran

Full commands, diagnostics and exits: `/tmp/turts-qa0-t2-review/{ci,plant}.txt`; reproducible plant: `plant.py`.

- `python3 scripts/qa/yardstick.py --check` | docs/superpowers/specs/2026-10-29-qa0-instrument-design.md:644 sync | nonzero refutes | exit 0, 0.058 s.
- `python3 scripts/qa/completeness.py --self-test && python3 scripts/qa/completeness.py --enforce` | docs/superpowers/specs/2026-10-29-qa0-instrument-design.md:645 gate | nonzero refutes | exit 0, 0.152 s; 21/21, checked 1.
- Scratch `--self-test`, before/after one plant | tests/qa/completeness/cases.tsv:22 detects skipped enforcement | green after plant refutes | exits 0/0.
- Scratch `--enforce --specs-dir tests/qa/completeness/dirs/r1r3/specs --registry tests/qa/completeness/dirs/r1r3/specs.tsv`, before/after | docs/superpowers/specs/2026-10-29-qa0-instrument-design.md:209 rejects incomplete client spec | green refutes | exits 1 (R1,R3)/0.
- Scratch exact completeness CI command, planted | tests/qa/completeness/cases.tsv:22 trust | green refutes | exit 0.
- Python grammar/paths probe | brief stdlib/3.8, R6/R7 | syntax error/missing path refutes | 3.8 grammar accepted; 74 decision rows; real done path exists.

The plant exposed a semantic failure, with no setup error. Exact rule-set equality and all three self-test guards exist; fixture edits match their declared mutations.

## Not checked

Python 3.8 runtime and builder historical trust runs: NOT TRACED; runtime unavailable and only one independent plant requested.

## What I changed

none

## Open questions for the orchestrator

None.

---

## Run 11 — scoped repair recheck, df9a1f3

VERDICT=fail FINDINGS=1 HIGH=0 RUNS=11 NOT_CHECKED=1

This recheck supersedes the original finding statuses.

## Findings

- Original 1 **ADDRESSED**: `scripts/qa/completeness.py:254` and `:307` call the same `enforce`. The repeated R1–R3 skip plant makes case 21 fail, exit 1.
- Original 2 **ADDRESSED**: `scripts/qa/completeness.py:219` retains exempt evidence. The persistent prior probe has only its expected R1, with no R4 and checked count 1.
- Original 3 **ADDRESSED**: `docs/qa/specs.tsv:6` now records QA0 as `gated`.
- Original 4 **ADDRESSED**: `scripts/qa/completeness.py:166` selects QA0/MS by milestone. An independently renamed fixture is green for both.
- **New Medium — `scripts/qa/completeness.py:219`.** Exempt rows also become R4 diagnostic targets at `:227`, so exemption is lost for R4. A registry containing only exempt M1 (`gate=done`) with `PC01 in` produces R4 for missing IN01, exit 1, while printing `checked specs: 0`. The same input on b91af0f exits 0. This contradicts the exemption in `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md:211`. Keep exempt rows as evidence; restrict R4 diagnostics to checked specs.

## What I ran

Exact commands and outputs: `/tmp/turts-qa0-t2-review/run11/results.txt`; runner: `check.py`.

- `python3 scripts/qa/yardstick.py --check` | sync | nonzero refutes | exit 0, 0.0245 s.
- `python3 scripts/qa/completeness.py --self-test && python3 scripts/qa/completeness.py --enforce` | CI | nonzero refutes | exit 0, 0.0867 s; 21/21, checked 1.
- Direct incomplete-MC CLI | R1/R3 enforcement | green refutes | baseline exit 1; planted exit 0.
- Scratch skip plant, `--self-test` | row 21 detects skipped rules | green refutes | exit 1, case 21 failed.
- Renamed QA0/MS policy assertions; persistent prior assertion | repaired policies | wrong rules refute | all exit 0.
- Original cases byte comparison | preserve 21 sets | differences refute | equal, exit 0.
- Exempt-only CLI, old/fixed | exemption retained | fixed nonzero refutes | exits 0/1; semantic regression, no setup error.

## Not checked

Python 3.8 runtime: NOT TRACED; unavailable. Recheck scope is the repair diff only.

## What I changed

none

## Open questions for the orchestrator

None.

---

## Run 13 — scoped repair round 2 recheck, e3fd97d

VERDICT=pass FINDINGS=0 HIGH=0 RUNS=7 NOT_CHECKED=1

## Findings

Run 11 Medium **ADDRESSED**. `scripts/qa/completeness.py:219` retains every eligible spec as dependency evidence. Exempt specs stop at `:221`; only checked specs enter the consumer list at `:227`. R4 receives separate evidence and consumer lists at `:229`. The persistent regression guard at `:295` rejects the previous defect.

No new breakage found within `df9a1f3..e3fd97d`. Previous findings remain closed.

## What I ran

Full commands and outputs: `/tmp/turts-qa0-t2-review/run13/results.txt`; reproducible runner: `check.py`.

- `python3 scripts/qa/yardstick.py --check` | sync | nonzero refutes | exit 0, 0.0273 s.
- `python3 scripts/qa/completeness.py --self-test && python3 scripts/qa/completeness.py --enforce` | gate | nonzero refutes | exit 0, 0.0988 s; 21/21, checked 1.
- `python3 scripts/qa/completeness.py --enforce --specs-dir tests/qa/completeness/dirs/exempt-only/specs --registry tests/qa/completeness/dirs/exempt-only/specs.tsv` | exemption | nonzero refutes | exit 0, checked 0.
- Persistent prior-evidence assertion (`scripts/qa/completeness.py:288`) | earlier evidence retained | R4 or wrong count refutes | exact rules R1, no R4, checked 1; exit 0.
- Cases byte comparison against df9a1f3 | preserve 21 sets | difference refutes | identical, exit 0.
- Scratch plant passes evidence as consumers, then runs self-test and direct exempt-only CLI | regression guard detects defect | green refutes | both exit 1; self-test names exempt-consumer failure, CLI emits R4.

Initial scratch runner hit an anchor-count assertion before planting. This setup error was corrected and all checks rerun; it is not semantic-red evidence (`setup-error.txt`).

## Not checked

Python 3.8 runtime: NOT TRACED. Scope was the round 2 repair only.

## What I changed

none

## Open questions for the orchestrator

None.


## Task 3

VERDICT=fail FINDINGS=4 HIGH=3 RUNS=10 NOT_CHECKED=2

## Findings

1. **High — A2 escape.** `scripts/qa/driver_rule.py:152` checks only direct `eval(`. `const execute = eval; execute("1+1")` exits 0, against design 4.4 A2. Ban the `eval` identifier; add persistent coverage. Function aliases correctly fail.
2. **High — A1 escape.** `scripts/qa/driver_rule.py:150` masks strings before the bridge scan. `window["turts_debug"]()` exits 0. This fixed bridge name is outside the accepted variable-key residual. Scan literal bridge keys; add coverage.
3. **High — A4 escape.** `scripts/qa/driver_rule.py:169` checks dangerous process members only with dot syntax. `process["binding"]("fs")` exits 0. Check forbidden literal member names too; add coverage.
4. **Medium — dependency scan.** `scripts/qa/driver_rule.py:301` includes `node_modules`, excluded by design 4.2. A legal tree with one vendor JS file exits 1/DR1. Prune dependency directories; add coverage.

The 40 unique cases match exact sets. CLI and self-test share `check_path` (`driver_rule.py:376,409`). Recursive closure works. Five ledger rows and role assignments match the brief. All 12 live JS changes are first-line comments only. No harness or split was added.

## What I ran

Commands below run at c00135d. Logs: `/tmp/turts-qa0-t3-review/`.

| Command | Claim; refutation | Observed |
| --- | --- | --- |
| `python3 scripts/qa/driver_rule.py --self-test && python3 scripts/qa/driver_rule.py tests/browser` | CI:149; nonzero | exit 0; 40/40; 0.160s |
| `node --test tests/browser/{m2-projection,m2-audit,t7-launch-options}.test.js` | unchanged behavior; failure | exit 0; 16/16 |
| `node --check` each browser JS | valid syntax; nonzero | 12 exits 0 |
| `cmp` stripped headers against base | header-only; difference | 12 exits 0 |
| CLI three forbidden probes | A1/A2/A4; exit 0 refutes | exits 0 |
| CLI Function alias | A2; exit 0 | exit 1 |
| CLI dependency tree | exclusion; nonzero | exit 1 |
| CLI cyclic transitive require | closure; exit 0 | exit 1/DR2/A1 |
| Scratch A7-disabled CLI, self-test, exact CI | shared real enforcement; green self-test refutes | exits 0,1,1; four mismatches; baseline CI exit 0 |
| `ast.parse(..., feature_version=(3,8))` | compatible grammar; exception | exit 0 |

## Not checked

NOT TRACED: browser execution; Python 3.8 runtime (grammar checked only).

## What I changed

none

## Open questions for the orchestrator

None.


## Scoped recheck: d8b6a52 (Run17)

VERDICT=fail FINDINGS=1 HIGH=1 RUNS=10 NOT_CHECKED=2

### Findings

1. **High — repair introduces an A2 regression.** `scripts/qa/driver_rule.py:155` masks strings, including executable template interpolation, before scans at lines 157–159. A driver with ``exports.drive = async io => `${eval("1+1")}`;`` or `` `${Function("return 1")()}` `` now exits 0. Both exit 1/DR2/A2 with c00135d. Node executes both expressions and prints 2 and 1. Preserve template expression code in scans, or scan A2 without this mask. Add persistent regression cases. This uses direct dynamic-code syntax, outside the accepted variable-key residual.

All four earlier findings are closed: identifier ban `driver_rule.py:157`, literal bridge key `:153`, dangerous literal process member `:176`, dependency exclusion `:318`. The original 40 rows and sets are byte-identical. Four appended cases pass. The 40-case minimum at `:404` remains correct; accepted `var-key.js` stays green.

### What I ran

Logs: `/tmp/turts-qa0-t3-review/recheck17/`.

| Command | Claim; refutation | Observed |
| --- | --- | --- |
| Exact CI command from `.github/workflows/ci.yml:149` | current gate; nonzero | exit 0; 44/44; 0.174s |
| `cmp old-cases.tsv first40-cases.tsv` | original rows; difference | exit 0 |
| CLI original three forbidden probes | fixed A1/A2/A4; exit 0 | three exits 1, exact sets |
| CLI dependency fixture and `var-key.js` | legal inputs; nonzero | both exit 0 |
| Prescribed `node --test` | behavior; failures | exit 0; 16/16 |
| `node --check` each live JS | syntax; failure | 12 exits 0 |
| Scratch prior checker with 44 current cases, exact CI | regression coverage; green | exit 1; four mismatches |
| Current/prior CLI on two template probes | A2 retained; current green | exits 0/1 each |
| Scratch self-test with 40/39 rows | minimum 40; reversed result | exits 0/1 |
| `node -e require(...).drive({}).then(console.log)` | probes execute; exception | both exit 0; outputs 2/1 |

### Not checked

NOT TRACED: browser execution; Python 3.8 runtime. Python 3.8 grammar parses.

### What I changed

none; product worktree clean.

### Open questions for the orchestrator

None.


## Final-head scoped recheck: d6e770c08b6035e6933eeb74f6d9cae62bf9978f (Run18)

VERDICT=pass FINDINGS=0 HIGH=0 RUNS=16 NOT_CHECKED=2

### Findings

None in `d8b6a52..d6e770c`. **ADDRESSED:** the remaining A2 interpolation issue reported against d8b6a52. `scripts/qa/driver_rule.py:156` and `:157` now scan unmasked code. Both direct executable-template probes return DR2/A2. Literal template bridge keys are checked at `:153`; literal dynamic-code keys at `:160`. No new breakage found in this final diff. All four original findings remain closed.

### What I ran

All commands ran on final head d6e770c. Logs: `/tmp/turts-qa0-t3-review/recheck18/`.

| Command | Claim; refutation | Observed |
| --- | --- | --- |
| `python3 scripts/qa/driver_rule.py --self-test && python3 scripts/qa/driver_rule.py tests/browser` | final gate; nonzero | exit 0; 44/44; 0.151s |
| CLI `recheck17/template-eval.js`, `template-function.js` | A2 retained; exit 0 | both exit 1/DR2/A2 |
| CLI `eval-alias.js`, `literal-bridge.js`, `literal-process-binding.js` | earlier fixes; green | exits 1; exact expected sets |
| CLI `var-key.js`, `literal-key.js`, `index-ok.js`, `zlib-driver3.js`, `png-pure.js` | accepted controls; red | five exits 0 |
| CLI `dirs/node-modules-excluded` | dependency pruning; red | exit 0 |
| CLI template bridge/eval/Function key probes | literal names caught; green | three exits 1; A1/A2 plus A4 |
| `cmp original40.tsv current-first40.tsv` | original rows unchanged; difference | exit 0 |

### Not checked

NOT TRACED: browser execution; Python 3.8 runtime. This final diff changes only the static checker. Run17 Node and syntax results apply to unchanged browser files.

### What I changed

none; final product worktree clean.

### Open questions for the orchestrator

None.

## Final whole-branch review and fix rechecks

This report retains the initial findings and subsequent scoped checks. Run26 is the final verdict.

VERDICT=pass FINDINGS=0 HIGH=0 RUNS=56 NOT_CHECKED=2

Latest controlling verdict: Run26 on canonical `3e1b0ab3897dda6de910bf73c784153aa97d72d9`; earlier findings below are historical and superseded by the final appendix.

## Findings

1. **High, blocking — `scripts/qa/driver_rule.py:151`.** `without_strings` masks executable template interpolation. A driver with ``const state = `${window.turts_debug()}`;`` passes (exit 0); Node executes the bridge. Direct A1 use violates design 4.4, outside the accepted variable-key residual. Preserve interpolation code in A1 scans and add a persistent regression case. `/tmp/turts-qa0-final-review/template-probe.json`.
2. **Low, optional — `scripts/qa/completeness.py:87`.** A coverage row `| invalid-id | mod | - |` is ignored. R1 says unknown row ids fail. The otherwise-green fixture has no diagnostic. Distinguish header/separator rows from malformed data ids. This cannot conceal missing CORE rows.
3. **Medium, optional — `crates/turts_pack/src/main.rs:27`.** Consumer report's LR3 remedy/help recommendation is valid but nonblocking: pinned text meets the approved contract. Root explicitly deferred explanation to Claude's documentation pass. Keep that disposition visible.

Source: detached `7f979506dcce1be78aae9c51308adea514e4973e`; production code equals verified `6234bf50d285f04d70b0f5d7e1ef3b9cde09aca6`; later diff is vault-only.

## What I ran

Each row names the claim; the opposite result refutes it. All probes/edits were in `/tmp/turts-qa0-final-review/`.

| Command | Claim | Observed exit/result |
| --- | --- | --- |
| `python3 scripts/qa/yardstick.py --check` | design 11/A1 sync | 0 |
| `python3 scripts/qa/completeness.py --self-test` | A2–A4 exact sets | 0; 21/21 |
| `python3 scripts/qa/completeness.py --enforce` | A5 shared path | 0; checked1 |
| `python3 scripts/qa/driver_rule.py --self-test` | A7 sets | 0; 44/44 |
| `python3 scripts/qa/driver_rule.py tests/browser` | A6 roles | 0 |
| `sha256sum -c SHA256SUMS` | plan T1 probe integrity | 0; 83 |
| Python byte comparisons | plan global constraints | 0; protected files unchanged, 46 closures, 12 header-only browser edits |
| `cargo test -p turts_mod_runtime --test resolve --test archive --test final_review --lib --locked -- --nocapture` | design 6–7 | 0; 37 tests; 25 diamond input permutations; nested duplicates refused |
| Driver CLI, template bridge | A1 direct bridge fails | 0, defect |
| Driver CLI, bare bridge | A1 control fails | 1/DR2/A1 |
| `node execute-template.cjs` | accepted expression executes | 0/BRIDGE_EXECUTED |
| Python malformed-row probe | R1 unknown row fails | 0; no diagnostics, defect |
| Python audit of mutation logs | T6 red/restore claims | 0; all23 logs contain failures, restored logs no FAILED |

Read full receipt and acceptance: workspace796/0/18, process17/17, Node16, VulkanT7 passes137frames; all23 mutations red/restored-green. A8 deferred; A19 O1 guard. These receipts do not close finding1.

## Not checked

NOT TRACED: required behavior when `available` aliases the root id (two differing-version Library entries return `[game]`; LR3/LR8 do not define this identity case). Hosted final `check` status remains root-owned.

## What I changed

none. Requested report only; scratch build uses a separate target. Worktree clean.

## Open questions for the orchestrator

Repair finding1 before acceptance. Optional findings can stay deferred.


## Run 24 — Final fix batch recheck

VERDICT=fail FINDINGS=3 HIGH=1 RUNS=24 NOT_CHECKED=2

Candidate `a48aad17ba7527d82a434edd66b131e723b8a2b4`, clean detached ed62. Original fix commits: T2 `12dbb83eda815cacbd99fe3466e3e46f01ae79b7` → candidate `50d6e48`; T3 `7b859f8cd32b90746ad3563b506846e4213ba9ba` → candidate `a48aad1`.

### Findings

1. **High, blocking — A1 remains incomplete.** `scripts/qa/driver_rule.py:120` counts a `}` inside a template-expression comment as a real closing brace. ``exports.drive = async io => `${/* } */ window.turts_debug()}`;`` still exits 0; Node prints `BRIDGE_EXECUTED_AFTER_COMMENT`. `without_comments` at `:42` does not remove interpolation comments. Fix template-expression comment handling and persist this regression. This stays inside the original direct-template A1 defect.
2. **Low, optional — `scripts/qa/driver_rule.py:79`.** A comment containing `turts_debug` inside an otherwise legal interpolation now triggers A1. Both line/block comment controls return `42` in Node without a bridge, but CLI exits 1. The comment handling fix can cover this too.
3. **Medium, optional — prior consumer help recommendation.** Still deferred to Claude's documentation pass. No source-help change requested.

**ADDRESSED:** original simple-template example now DR2/A1, with a persistent row 45. **ADDRESSED:** malformed R1 row at `scripts/qa/completeness.py:94`; direct malformed and original well-formed unknown ids each produce exactly R1. Original 44 driver rows and all 21 completeness case rows remain byte-identical.

### What I ran

| Command/probe | Exit/result |
| --- | --- |
| `python3 scripts/qa/yardstick.py --check` | 0 |
| `python3 scripts/qa/completeness.py --self-test` | 0; 21/21 |
| `python3 scripts/qa/completeness.py --enforce` | 0; checked1 |
| `python3 scripts/qa/driver_rule.py --self-test` | 0; 45/45 |
| `python3 scripts/qa/driver_rule.py tests/browser` | 0 |
| Four direct/nested/object/quoted-brace template probes | each 1/DR2/A1 |
| Escaped/literal/nested legal templates and accepted var-key | each 0 |
| Original Node execution probe | 0/BRIDGE_EXECUTED |
| Malformed and well-formed unknown coverage probes | exactly R1 each |
| Two template-comment controls, CLI/Node | CLI1; Node0/no bridge |
| Comment-brace bridge, CLI/Node | CLI0; Node0/bridge executed; defect |
| Case byte checks and old-source23 receipt audit | 0; unchanged prior rows; all23 old-source PASS |

Logs: `/tmp/turts-qa0-final-review/fix-recheck24/`. Current candidate mutation rerun and hosted final CI remain NOT TRACED. Root-id alias required behavior remains NOT TRACED contract ambiguity. No repairs or implementation changes; requested report appendix only.


## Run 26 — Final source and independent test verdict

VERDICT=pass FINDINGS=0 HIGH=0 RUNS=56 NOT_CHECKED=2

Canonical `3e1b0ab3897dda6de910bf73c784153aa97d72d9`; reviewed candidate `5b5d6aef897636ae074178372fe3cbdc34a94b38`. Original fixes: T2 `12dbb83`, T3 `7b859f8` and `b2180c8`. **ADDRESSED:** A1 simple and comment-brace template escapes, with persistent cases 45/46 (`scripts/qa/driver_rule.py:125`). **ADDRESSED:** malformed R1 row (`scripts/qa/completeness.py:94`). Conservative complete-namespace scanning follows the approved probe. The earlier comment/literal allowance note is superseded by this policy. No remaining blocker. Optional consumer help remains explicitly deferred.

Exact five Python CLI commands exit 0: yardstick sync 355/181; completeness 21/21 and checked 1; driver 46/46 and live tree exit 0. Direct, nested, object, quoted-brace and comment-brace examples all fail with exit 1/DR2/A1. Both Node witnesses execute bridges. Var-key stays at exit 0; concatenated prefix is A4 only. Prior 45 driver rows and 21 completeness rows are preserved.

After source pass, switched to qa-lead test mode on clean canonical a6bb. All 23 required mutation rows passed: 24 semantic red commands (19 Rust exit 101 with named FAILED tests, 5 Python exit 1), 24 restored greens exit 0. Independent audit verifies committed blob hashes, empty final porcelain, exact HEAD and removal of the file created by row 20. All four flakiness commands exit 0; full bad_packages passes 16/16. No source repairs.

Committed byte comparisons exit 0 for baseline 6234 runtime/browser/CI/harness components, reviewed 5b source excluding vault, and protected contracts. Baseline 796 Rust passes/0 failures/18 ignored, 17 process passes, 16 Node passes and Vulkan T7 with 137 frames remain traced component evidence. No full final pipeline rerun is claimed.

Final artifacts: `/tmp/turts-qa0-t6-final/{mutation-receipt.md,mutation-audit.json,flakiness.json,acceptance.md}`. Concise report: `final-validation-report.md`. Source probes: `/tmp/turts-qa0-final-review/run26/source-checks.json`. A8 stays deferred; A19 stays a guard. Final hosted CI run 40923429639 is root-owned. Root-id alias policy remains NOT TRACED.

## Final canonical validation verdict

VERDICT=pass FINDINGS=0 HIGH=0 RUNS=56 NOT_CHECKED=2

## Findings

No blocking source or instrument findings. Canonical `3e1b0ab3897dda6de910bf73c784153aa97d72d9` matches reviewed candidate `5b5d6ae` outside vault (diff exit0). Both A1 examples now fail with DR2/A1 (`scripts/qa/driver_rule.py:125`; persistent cases 45/46). R1 malformed and well-formed unknown ids both give exactly R1 (`scripts/qa/completeness.py:94`). Original 45 driver cases and 21 completeness cases are preserved. The approved static probe permits conservative namespace-text refusal; no new string allowance is inferred.

## What I ran

Switched from independent review to qa-lead test mode for prescribed plants. All runs use a6bb, warm `target/qa0-t6`, debug/test debug 0, incremental 0, jobs 4.

| Command/check | Claim; refuter | Observed |
| --- | --- | --- |
| `python3 /tmp/turts-qa0-t6-final/run_mutations.py 1-23` | Every prescribed plant fails semantically; a compile error or missing named failure refutes | 23 rows; 24 red commands: 19 Rust exit 101, 5 Python exit 1; 24 restored greens exit 0 |
| `python3 /tmp/turts-qa0-t6-final/audit.py` | Exact tests, committed blobs, clean restoration | 0; both capability/lockfile tests named; all 23 clean/blob proofs; row 20 created file removed |
| Four commands in `flakiness.json` | Three Python CI commands plus full bad_packages repeat | all exit 0; yardstick 355/181; completeness 21/21, checked 1; driver 46/46, live exit 0; pack 16/16 |
| Three committed component byte comparisons | Baseline runtime/browser/CI and reviewed fixes apply to final SHA | all0 |

RUNS=56 counts 48 mutation commands, 4 repeats, 1 audit, 3 byte comparisons. Source-phase direct/nested/object/comment-brace CLI probes fail with exit 1/A1; both Node witnesses execute bridges. Accepted var-key stays at exit 0; concatenated prefix is A4 only. Full source-phase evidence: `/tmp/turts-qa0-final-review/run 26/source-checks.json`.

Final artifacts: `/tmp/turts-qa0-t6-final/mutation-receipt.md`, `mutation-audit.json`, `flakiness.json`, and `acceptance.md` (all 21 items). Baseline Rust 796/0/18, process 17/17, Node 16 and Vulkan T7 with 137 frames apply through exact component-byte proof; no final full-pipeline rerun is claimed.

## Not checked

Final hosted CI is root-owned: [run 40923429639](https://github.com/26medias/TuRTS/actions/runs/40923429639). Root-id alias required behavior remains NOT TRACED. A8 is approved deferred; A19 is the approved O1 guard. Consumer LR3 explanation remains optional/deferred.

## What I changed

No lasting product changes. Only prescribed temporary plants, restored; clean canonical HEAD. Requested reports/artifacts only. Original receipts remain untouched.

## Open questions for the orchestrator

None. Record hosted status, save receipts, and stop at authorized run 26.
