# QA0 Instrument and Schema Groundwork Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Plan status:** DRAFT, revision 1 after gate 2 (rigour `fail` B1, H1, M1, M2, L1-L3; mod-runtime `pass-with-changes` B1, M1, M2, L1). Re-gate this revision. Author: `qa-lead`, 2026-10-29. Commit of record for every `path:line`: `bfef807`.

**Goal:** Put the completeness gate and the pixels-only driver rule into CI. Fix the two `turts-pack` defects. Open the schema for additive growth (K1 = O1). Replace the refusal "dependencies are not supported at M1" with a real library check (row QA0, S:721).

**Architecture:** Two Python standard-library tools in `scripts/qa/` run in the `agents` CI job. They read TSV inputs under `docs/qa/` and self-test on fixtures under `tests/qa/`. The Rust work stays in `turts_mod_api`, `turts_mod_runtime`, `turts_pack`, `turts_client`, and one server test. A new pure function `turts_mod_runtime::resolve::resolve` is called once in production, from `turts_pack::load_package`.

**Tech Stack:** Rust 2024, Bevy 0.20, RON 0.12.2, semver, Python 3 (standard library only), Node 20 on CI (v24.10.0 local).

**Design:** `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md` (revision 3, passed gate 1), "D" below. Product spec: `RTS-Engine-Project-Specifications.md` v0.8, "S" below. **CEO decisions (2026-10-29):** K1 to K6 as recommended (K1 = O1, K2 = option A in the must tier, K3 strict library, K4 no table for QA0, K5 no library serving, K6 CEO owns the CORE list). Build the "must" tier of D section 10 only.

**Deferred to MC:** the T7 split into `t7-driver.js` and `t7-oracle.js` with the harness `io` and `view` (K2 option C, A8); O2; the M3 version gate moved into the version table; a two-run process test of the T7 driver in CI.

## Global Constraints

- `ENGINE_API_VERSION` goes from `"0.2.1"` to `"0.2.2"` (D section 8). `NET_PROTOCOL_VERSION` does not change. No manifest field and no modder field is added. The lockfile format does not change.
- `deny_unknown_fields` stays on all 46 schema types (K1 = O1).
- K5 effect: after QA0, the server and the Game path of `turts-pack` refuse every root that has dependencies, with LR3 ("not provided"). This replaces "not supported at M1". The CEO accepted this effect with K5.
- The client never calls `resolve` (S:302, D section 7). It checks the lockfile only.
- The Python tools use the standard library only. Do not use a feature newer than Python 3.8 (the CI runner version is NOT TRACED).
- A self-test case passes only on set equality of rule ids (D section 3.4). A subset or a superset is a failure.
- When a mutation fixture gives an unexpected extra rule, fix the fixture. Never edit the expected set to match the tool output.
- No task edits `CLAUDE.md`, `.claude/agents/`, the design, or the product spec. Doc changes go to "Doc updates for the orchestrator".
- Code style: 4 spaces per indentation level, in Rust, Python, JavaScript, and TSV examples.

## Execution contract

- One branch per task, `codex/qa0-t<N>`, from the integration branch `codex/qa0-integration` (create it from `main` before wave 1).
- Before a Rust task merges: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` (`.github/workflows/ci.yml:29`), `cargo test --workspace --locked` (`ci.yml:33`), `bash scripts/boundary.sh` (`ci.yml:35`), and `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked` (`ci.yml:31`).
- Before a Python task merges: the CI command of its step, run locally, exit 0.
- Build binaries one at a time. Stage explicit paths only. Never `git add -A`.
- Never pipe a red or a green run through `tail`, `head`, or `grep` before you read its exit code. Write `; echo exit=$?` after the command (the revision 0 receipt lost the exit codes this way, D:114).
- "Red" means a test that compiles and fails, or a tool that exits 1 with a rule line. A compile error is not a red (D:618-619). When a test names a new item, add the item first as a stub that compiles (the "signature first" pattern), so the test can fail.
- In-tree trust runs and mutation runs come after the task commit. Commit the task files first (explicit paths). Then edit, run, and restore with `git checkout -- <file>` (for a file that the run created: `rm <file>`). Then `git status --porcelain -- <the changed files>` must be empty. Never use `git checkout --` on a new or uncommitted task file: it fails on a new file and it deletes uncommitted work on a tracked file (gate 2 run G1).
- Record each task's red output, green output, changed paths and commit in `vault/Work Log/`.

## Waves

Tasks in one wave share no file.

| Wave | Tasks | Shared-file reason for the order |
| --- | --- | --- |
| 1 | T1, T4 | none: T1 writes `docs/qa/`, `tests/qa/`, `scripts/qa/yardstick.py`; T4 writes `crates/` |
| 2 | T2, T5 | T2 needs the T1 fixtures; T5 needs T4 (0.2.2, `manifest.rs`, `main.rs`, `cli.rs`) |
| 3 | T3 | T3 and T2 both edit `.github/workflows/ci.yml` (`agents` job, `ci.yml:138-143`) |
| 4 | T6 | needs all |

```text
T1 → T2 → T3 ─┐
T4 → T5 ──────┴→ T6
```

## Budget

Six tasks. The design rate is 3.75 agent runs for each plan entry, gates included (D section 13).

- 6 tasks: 22.5 runs. With one repair entry: 7 entries, 26.25 runs. **Estimate: 22 to 26 runs.** This is in the CEO range.
- Merges that make this fit. The design has six must tasks (P1 to P6), and the probe copy adds one. Seven entries cost 26.25 runs before a repair. So:
    - T1 holds the probe copy and the yardstick extraction (from P1).
    - T4 and T5 cut P3, P4 and P5 by file. T4 holds the parts that need no library rule (capability, version table mechanism, typo hint, 0.2.2, client lockfile check). T5 holds the library rule, the resolver, the lift, the call sites, and the two real table rows.
    - The mutation runs of P6 are in T6, so an agent that did not write the code breaks it.

---

### Task 1: Probes into the repository, and the yardstick list

**Goal:** Move every probe and receipt that D cites from the session scratchpad into the repository. Extract the yardstick list. After this task, no task and no file of the plan points at the scratchpad.

**Seat and model:** `qa-lead` (build mode), Sonnet 5.5. Review: the orchestrator reads the `cmp` lines and the receipt. No gate agent.

**Depends on:** nothing. Wave 1.

**Source directories** (the only place in this plan that names them):

- `QP` = `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/qa0/`
- `CP` = `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/qa-probe/`

**Files (create):**

| Source | Repository path | Why this path |
| --- | --- | --- |
| `CP/fx_green.md` | `tests/qa/completeness/green.md` | D section 3.1 names it. T2 ports the syntax. |
| `CP/fx_mut_a_drop_PC02.md` to `CP/fx_mut_e_PC02_test_H9.md` | `tests/qa/completeness/mut-a.md` to `mut-e.md` | D section 3.5 names `mut-a` to `mut-e`. |
| `QP/repair-driver/*.js`, 28 files (all except `zlib-driver.js`, which revision 3 superseded with `zlib-driver3.js`) | `tests/qa/driver/<same name>` | D section 4.4 names `tests/qa/driver/`. |
| `CP/completeness_gate.py`, `CP/run_mutations.txt`, `QP/repair-c1.txt`, `QP/runs-c1.txt` | `docs/qa/probes/coverage/` | D names no path. One directory for evidence is the cheaper choice. |
| `CP/hook_ban.py`, `CP/fx_blind.js`, `QP/runs-c2.txt`, `QP/runs-c2-detail.txt` | `docs/qa/probes/bridge/` | as above |
| `QP/repair-driver/dr_model3.py`, `QP/repair3-driver.txt`, `QP/repair2-driver.txt`, `QP/repair-driver.txt` | `docs/qa/probes/driver/` | as above |
| `QP/repair-selftest.txt`, `QP/repair-selftest/*.tsv`, `QP/rig-final-selftest.txt`, `QP/rig-recheck/selftest3.py`, `QP/rig-recheck/*.tsv` | `docs/qa/probes/selftest/` | as above |
| `QP/p0` to `QP/p7` (package directories), `QP/rig-pack/dup`, `QP/runs-pack.txt`, `QP/repair2-pack.txt`, `QP/rig-pack2.txt` | `docs/qa/probes/pack/` | as above |
| `QP/schema-probe/{Cargo.toml,Cargo.lock,src/main.rs}`, `QP/runs-schema.txt` | `docs/qa/probes/schema/` | as above. No `target/`. |
| `QP/modrt-probe/{Cargo.toml,Cargo.lock,src/main.rs}`, `QP/runs-modrt-o2.txt`, `QP/repair-modrt.txt` | `docs/qa/probes/modrt/` | as above. No `target/` (116 MB). |
| `QP/gate-modrt.md`, `QP/gate-rigour.md`, `QP/qa-repair.md`, `QP/qa-repair2.md` | `docs/qa/probes/gates/` | D cites `QP/gate-modrt.md` H1, N4 and `QP/gate-rigour.md` run 12. |
| new | `docs/qa/probes/SHA256SUMS` | the hash of each file under `docs/qa/probes/` |
| new | `docs/qa/probes/receipts/2026-10-29-t1-rerun.txt` | the re-run receipt (step 4) |
| new | `scripts/qa/yardstick.py` | D section 3.1 |
| new | `docs/qa/yardstick-a.tsv` | D section 3.1 |

One edit is necessary: `docs/qa/probes/modrt/Cargo.toml` names `turts_mod_api` by the absolute path `/home/julien/Projects/TuRTS/crates/turts_mod_api`. Change it to the relative path `../../../../crates/turts_mod_api`. Both probe manifests have an empty `[workspace]` table, and the root workspace has `members = ["crates/*"]` (`Cargo.toml:3`), so cargo does not build them. To build one, set `CARGO_TARGET_DIR` to a path outside the tree.

**Acceptance items closed:** A1. Precondition of A2 to A7 (the fixtures exist in the tree).

**Steps:**

- [ ] **Step 1: The oracle first.** Count the yardstick rows with `awk`, independent of any tool:

    ```bash
    awk -F'|' '/^# Part A/{a=1} /^# Part B/{a=0} a && /^\| [A-Z][A-Z][0-9][0-9] \|/ {n++; t=$7; gsub(/^ +| +$/,"",t); split(t,w," "); if(w[1]=="CORE"){c++; if(t!="CORE") cn++}} END{print "rows="n, "core="c, "core_with_note="cn}' docs/research/review-2026-10/inventory-engine.md; echo exit=$?
    ```

    Expected: `rows=355 core=181 core_with_note=4`, `exit=0`. The four CORE rows with a note are IN08, CA15, FV04, MP02. CO13 and EC16 have tier `OPTIONAL (CORE for FAC)`; they are not CORE.

- [ ] **Step 2: Copy.** Make the directories. Copy each file of the table with `cp`. Then run `cmp` on each pair, except the edited `modrt/Cargo.toml`. Expected: every `cmp` exits 0. Write each `cmp` line and its exit code into the receipt.

- [ ] **Step 3: Hashes.** Run `cd docs/qa/probes && find . -type f ! -name SHA256SUMS | LC_ALL=C sort | xargs sha256sum > SHA256SUMS`. Then `sha256sum -c SHA256SUMS; echo exit=$?`. Expected: `exit=0`. Red check: change one byte in a scratch copy of the directory and run `sha256sum -c` there. Expected: one `FAILED` line, exit 1.

- [ ] **Step 4: Re-run the probes from the repository paths.** Write the receipt `docs/qa/probes/receipts/2026-10-29-t1-rerun.txt`. Its first lines hold `git rev-parse HEAD` and the value of `ENGINE_API_VERSION` (`crates/turts_mod_api/src/lib.rs:33`). Each further line holds the command, the exit code, and the rule set. No pipe hides an exit code.
    - Coverage probe, on `docs/research/review-2026-10/inventory-engine.md` with `<n>=<file>` arguments (milestone as an integer, `docs/qa/probes/coverage/completeness_gate.py:101-108`): the four specs `docs/superpowers/specs/2026-10-26-m1-foundation-design.md` (1), `2026-10-26-m2-economy-design.md` (2), `2026-10-27-m3-industrial-design.md` (3), `2026-10-28-m4-terrain-design.md` (4); then `tests/qa/completeness/green.md` and `mut-a.md` to `mut-e.md`, each as milestone 4. Expected (this plan's re-run on `bfef807` gave it): each spec `exit=1` with rules {R1, R3}; `green.md` `exit=0`; `mut-a` {R1, R4}; `mut-b` {R2}; `mut-c` {R3}; `mut-d` {R4}; `mut-e` {R2}. This equals `docs/qa/probes/coverage/repair-c1.txt`.
    - Bridge probe: `python3 docs/qa/probes/bridge/hook_ban.py <file>` on each of the 12 files `tests/browser/*.js`. Expected: red on 7 of 12, as in `docs/qa/probes/bridge/runs-c2.txt`. `t7.js` and `m3-human.js` exit 1.
    - Driver model: in `tests/qa/driver/`, run `python3 ../../../docs/qa/probes/driver/dr_model3.py` on the 20 evasion samples, `ok-driver.js`, `zlib-driver3.js`, `var-key.js`, `../../../tests/browser/t7.js`, `../../../tests/browser/m3-human.js`; then with `--pure` on `../../../tests/browser/m2-audit.js`, `../../../tests/browser/t7-launch-options.js`, `png-pure.js`, `literal-key.js`, `index-ok.js`. The model prints verdicts and always exits 0 (`dr_model3.py:50-57`). So read the `RED` and `GREEN` words, not the exit code. Expected: 22 RED (20 samples, `t7.js`, `m3-human.js`), 8 GREEN, as in `repair3-driver.txt`.
    - `turts-pack` on the package copies: `turts-pack` writes `<dir>.tar` next to `<dir>`. So copy `docs/qa/probes/pack/p0` to `p7` to the scratchpad first, then run `cargo run -q -p turts_pack -- <scratch>/pN; echo exit=$?` on each. Expected on engine 0.2.1: p0 0, p1 0, p2 0, p3 2, p4 2, p5 2, p6 2, p7 2. T1 and T4 are in one wave. If T4 merged before this step (engine 0.2.2), p1 exits 2 on `capabilities` and p7 exits 2 on `exports.maps`; the receipt records which engine it ran on. The hashes of p0, p1 and p2 equal `runs-pack.txt`, because the archive is reproducible (`crates/turts_pack/src/main.rs:19-20`). A different hash means the copy is not exact.
    - The schema and mod-runtime probes are not rebuilt. They are evidence for K1, and the CEO chose O1. The receipt says so.

- [ ] **Step 5: Write `scripts/qa/yardstick.py`.** `--extract` reads Part A of `docs/research/review-2026-10/inventory-engine.md`, from the heading `# Part A` to the heading `# Part B`. It writes `docs/qa/yardstick-a.tsv` with the header `id	area	name	tier	tier_note	kind	deps` and one row for each item row. `area` is the section number of the `## A<n>.` heading (for example `A1`). `tier` is the first word of the tier cell. `tier_note` is the rest of the cell, or empty. `deps` is the comma list of item ids in the "Depends on" cell, or `-`. `--check` extracts to a temporary file and compares the bytes with the committed TSV. It exits 0 when equal and 1 with the first differing line when not.

- [ ] **Step 6: Red before green.** Run `python3 scripts/qa/yardstick.py --check; echo exit=$?` before `--extract`. Expected: `exit=1`, the TSV is missing. Run `--extract`. Then count with `awk` on the TSV:

    ```bash
    awk -F'\t' 'NR>1{n++; if($4=="CORE"){c++; if($5!="") cn++}} END{print "rows="n, "core="c, "core_with_note="cn}' docs/qa/yardstick-a.tsv; echo exit=$?
    ```

    Expected: `rows=355 core=181 core_with_note=4`, the same as step 1. Run `--check`. Expected: `exit=0`.

- [ ] **Step 7: Mutation run.** The TSV is not committed yet, so do not use `git checkout`. Save it with `cp docs/qa/yardstick-a.tsv <scratch>/yardstick-a.tsv`. Change the tier of one row from `CORE` to `COMMON`. Run `--check`. Expected: `exit=1` and the changed line. Restore with `cp <scratch>/yardstick-a.tsv docs/qa/yardstick-a.tsv`. Run `--check` and `cmp` against a saved copy. Expected: `exit=0`.

- [ ] **Step 8: Commit** the explicit paths of the table, `scripts/qa/yardstick.py`, and `docs/qa/yardstick-a.tsv`. The CI step for the yardstick goes in T2.

---

### Task 2: Completeness gate

**Goal:** The gate of D section 3: milestone order, registry, tool with R1 to R9, the QA0 fixture, 12 mutations, three directory cases, the self-test, and the CI step.

**Seat and model:** `backend-engineer`, Sonnet 5.5. `qa-lead` reviews the instrument (review mode): which build makes each case red.

**Depends on:** T1. Wave 2.

**Files:**

- Create: `scripts/qa/completeness.py`; `docs/qa/milestones.tsv`; `docs/qa/specs.tsv`; `tests/qa/completeness/cases.tsv`; `tests/qa/completeness/mut-f.md` to `mut-l.md`; `tests/qa/completeness/no-client.md`; `tests/qa/completeness/dirs/r8/specs.tsv` and `dirs/r8/specs/<one file>.md`; `tests/qa/completeness/dirs/r9/specs.tsv` and `dirs/r9/specs/<one MC file>.md`; `tests/qa/completeness/dirs/r1r3/specs.tsv` and `dirs/r1r3/specs/<one MC file>.md` (registry row `client: yes`, `gate: draft`; the spec has no coverage table and no `Player path`).
- Modify: `tests/qa/completeness/green.md` and `mut-a.md` to `mut-e.md` (from T1: the disposition syntax of D section 3.2; `excluded:S:<line>` becomes `excluded:D<nn>`; `later:M<n>` becomes a milestone id of `milestones.tsv`); `.github/workflows/ci.yml` (the `agents` job, after `ci.yml:143`).

**Inputs fixed by D:**

- `milestones.tsv`: columns `order`, `id`. 20 rows in the order of S section 14 (S:718-737): M1 1, M2 2, M3 3, QA0 4, MC 5, MP1 6, MP2 7, M4 8, M4b 9, M5 10, MS 11, MB 12, M6 13, M7 14, M8 15, M9 16, M10 17, M11 18, M12 19, M13 20. The integer is the row position (D gives no numbering; this is the cheaper choice).
- `specs.tsv`: the seven rows of D section 3.1, one for each file of `docs/superpowers/specs/` today. The QA0 row has `gate: gated`, because D:3 records the gate 1 pass (the table at D:154 shows `draft`, written before the pass).
- Lists in the tool, by file path (D section 3.3, 3.4): `EXEMPT` = the M1, M2, M3, M4 spec files while their `gate` is not `gated`; `NO_CLIENT` = {`2026-10-29-qa0-instrument-design.md`}; `NOT_MILESTONE` = {`2026-10-25-studio-agent-team-design.md`, `2026-10-26-m2-authoring-example.md`}; `MIN_CASES = 21`.
- R6 reads the decision rows of `RTS-Engine-Project-Specifications.md` with the pattern `^\| D<nn> \|` (S:30 is `| D01 |`).
- Spec mode and the client value. D says the `client: no` case runs "as milestone QA0" (D:260-261), but gives no other key for spec mode. Plan choice: in spec mode, `as_milestone` QA0 or MS (the two "No client" rows of S section 14, D:198-199) runs R4 to R8; every other milestone runs R1 to R8. In `--enforce` mode the tool keys only on file paths (R9), as D section 3.3 requires.

**`cases.tsv`** (columns `mode`, `file`, `as_milestone`, `expected_rules`). Write it before the tool. These are the sets of D sections 3.4 and 3.5:

| # | mode | file | as_milestone | expected_rules |
| --- | --- | --- | --- | --- |
| 1-4 | spec | the M1, M2, M3, M4 spec files | M1, M2, M3, M4 | R1,R3 |
| 5 | spec | `tests/qa/completeness/green.md` | M4 | - |
| 6 | spec | `mut-a.md` (drop PC02) | M4 | R1,R4 |
| 7 | spec | `mut-b.md` (OR24 no test) | M4 | R2 |
| 8 | spec | `mut-c.md` (no player path) | M4 | R3 |
| 9 | spec | `mut-d.md` (CA01 later) | M4 | R4 |
| 10 | spec | `mut-e.md` (PC02 bound to absent H9) | M4 | R2 |
| 11 | spec | `mut-f.md` (test id only in prose) | M4 | R2 |
| 12 | spec | `mut-g.md` (`later:M3`) | M4 | R5 |
| 13 | spec | `mut-h.md` (`excluded:D99`) | M4 | R6 |
| 14 | spec | `mut-i.md` (`done:` a missing path) | M4 | R7 |
| 15 | spec | `mut-j.md` (duplicate row) | M4 | R1 |
| 16 | spec | `mut-k.md` (id not in the TSV) | M4 | R1 |
| 17 | spec | `mut-l.md` (bad disposition) | M4 | R1 |
| 18 | spec | `no-client.md` (green.md without `Player path`) | QA0 | - |
| 19 | enforce-dir | `tests/qa/completeness/dirs/r8` | - | R8 |
| 20 | enforce-dir | `tests/qa/completeness/dirs/r9` | - | R9 |
| 21 | enforce-dir | `tests/qa/completeness/dirs/r1r3` | - | R1,R3 |

Row 21 is not in D. It proves on each CI run that `--enforce` applies R1 to R3 to a `client: yes` spec that is not exempt (S:721). Without it, a bug that skips R1 to R3 in `--enforce` stays green until the next milestone spec (gate 2 rigour M2).

`green.md` must hold at least one `done:<path>` row with a real path (for example `crates/turts_client/tests/acquire.rs`), one `later:` row, one `mod` row, and one `excluded:D<nn>` row, so that R5, R6 and R7 each have a green case.

**Acceptance items closed:** A2, A3, A4, A5; first half of A21 (local red on a mutation).

**Steps:**

- [ ] **Step 1:** Write `milestones.tsv`, `specs.tsv`, the ported fixtures, the new mutations, `no-client.md`, the three `dirs/`, and `cases.tsv`.
- [ ] **Step 2: Red first.** Write `scripts/qa/completeness.py` with the argument parser, the case loop, the set comparison, and the three self-test conditions of D section 3.4. Its rule functions return no red line yet. Run `python3 scripts/qa/completeness.py --self-test; echo exit=$?`. Expected: `exit=1`. Rows 1-4, 6-17 and 19-21 fail with `want [...] got []`. Rows 5 and 18 pass. This is the "gate that exits 0 on everything" build of D section 3.6.
- [ ] **Step 3: Green.** Implement R1 to R9 (D section 3.3; R1 to R4 port `docs/qa/probes/coverage/completeness_gate.py`). Each red line is `RED <rule> <file> <item id or ->: <text>`. Run `--self-test`. Expected: `exit=0`, 21 of 21 cases. Run `python3 scripts/qa/completeness.py --enforce; echo exit=$?`. Expected: `exit=0` and `checked specs: 1`.
- [ ] **Step 4: Commit** the explicit paths of steps 1 to 3, before any trust run.
- [ ] **Step 5: Trust runs.** (a) In a scratch copy of the tool, remove R2 and R4. Run `--self-test` from the copy. Expected: `exit=1`; rows 6, 7, 9, 10, 11 fail. (b) to (e) edit the committed `tests/qa/completeness/cases.tsv` in the tree. After each run: `git checkout -- tests/qa/completeness/cases.tsv`, then `git status --porcelain -- tests/qa/completeness/cases.tsv` is empty. (b) Header only. Expected: `exit=1`, conditions 1, 2 and 3 (D:231-233). (c) Delete row 4 (M4). Expected: `exit=1`, condition 1 names `2026-10-28-m4-terrain-design.md`, condition 3 `20<21`. (d) Change row 6 to `R1` only. Expected: `exit=1`, `want ['R1'] got ['R1', 'R4']`. (e) Delete row 21. Expected: `exit=1`, condition 3 `20<21`.
- [ ] **Step 6: CI.** Add two steps to the `agents` job after `ci.yml:143`:

    ```yaml
          - name: Yardstick list in sync
            run: python3 scripts/qa/yardstick.py --check
          - name: Completeness gate
            run: python3 scripts/qa/completeness.py --self-test && python3 scripts/qa/completeness.py --enforce
    ```

- [ ] **Step 7:** Run both CI commands locally with `time`. Expected: exit 0, under 1 s each (D section 9).
- [ ] **Step 8:** Commit `.github/workflows/ci.yml`.

---

### Task 3: Pixels-only driver rule (K2 option A)

**Goal:** Roles for every browser test file, the static check DR1 to DR7 with Part A and Part B (D section 4.4), the ledger, the samples, the self-test, and the CI step.

**Seat and model:** `web-platform-engineer`, Sonnet 5.5. `qa-lead` reviews the instrument.

**Depends on:** T1 (samples); T2 (`ci.yml`). Wave 3.

**Files:**

- Create: `scripts/qa/driver_rule.py`; `tests/browser/legacy-bridge.tsv`; `tests/qa/driver/cases.tsv`; `tests/qa/driver/ok-oracle.js`, `no-role.js`, `two-roles.js`, `bad-role.js`, `requires-legacy.js`, `legacy-helper.js`, `bad-oracle.js`; `tests/qa/driver/dirs/dr6-unlisted/`, `dirs/dr6-repaired/`, `dirs/dr7/` (each with its `*.js` files and `legacy-bridge.tsv`).
- Modify: the 28 samples of `tests/qa/driver/` from T1 (add a role line at line 1); the 12 files `tests/browser/*.js` (add a role line at line 1, before `"use strict";` or the first comment); `.github/workflows/ci.yml` (`agents` job, after the T2 steps).

**Roles** (D section 4.2, option A column): `legacy-bridge`: `t7.js`, `m3-human.js`, `m2.js`, `m3.js`, `m3-mobile.js`. `instrument`: `measure.js`, `spike_p8.js`. `unit`: `m2-projection.test.js`, `m2-audit.test.js`, `t7-launch-options.test.js`. `pure`: `m2-audit.js`, `t7-launch-options.js`. No `harness` file exists at QA0 (option C slips). The line is `// turts-test-role: <role>`. A comment before the `"use strict"` directive keeps the directive active.

**Ledger** `tests/browser/legacy-bridge.tsv`: header `file	until`, five rows (the five `legacy-bridge` files), `until` = `MC`. DR7 limit in the tool: 5. Each of the five files has a Part A item (this plan's model run: `m2.js` A1, A3, A4; `m3.js` A1, A4, A7; `m3-mobile.js` A1, A6, A7; `t7.js` A1, A6, A7; `m3-human.js` A7 `node:fs` and A6 `playwright-core` of its own at `tests/browser/m3-human.js:5-6`, and A1 through `m2.js`). So DR6 "repaired file" stays green on the tree.

**Sample roles:** `driver` for the 20 evasion samples, `ok-driver.js`, `zlib-driver3.js`, `var-key.js`, `requires-legacy.js`. `pure` for `bad-helper.js`, `m2-audit-copy.js`, `png-pure.js`, `literal-key.js`, `index-ok.js`. `oracle` for `ok-oracle.js`, `bad-oracle.js`. `legacy-bridge` for `legacy-helper.js`.

**Output format:** `RED <DRn> <An or -> <file>:<line>: <text>`. The rule set of a case is the set of `DRn` and `An` tokens. Exit 0 green, 1 red, 2 usage.

**`tests/qa/driver/cases.tsv`** (columns `mode` = `file` or `dir`, `path`, `expected`). The 20 evasion sets come from the model receipt `docs/qa/probes/driver/repair3-driver.txt`.

| Group | Rows | Expected |
| --- | --- | --- |
| Evasions, file mode | `evade.js` {DR2,A2}; `plain-bad.js` {DR2,A7}; `ctor.js` {DR2,A2}; `dynimport.js` {DR2,A5}; `global.js` {DR2,A3,A5}; `concat-req.js` {DR2,A5}; `concat-bridge.js` {DR2,A4}; `alias-req.js` {DR2,A5}; `proc.js` {DR2,A4}; `fetch.js` {DR2,A6}; `global-evade.js` {DR2,A3,A4}; `via-helper.js` {DR2,A1}; `pw.js` {DR2,A6}; `builtin.js` {DR2,A4}; `io-ctor.js` {DR2,A4}; `fs-ticklog.js` {DR2,A7}; `mod-parent.js` {DR2,A4}; `req-cache.js` {DR2,A5}; `mod-require.js` {DR2,A4,A7}; `via-fs-pure.js` {DR2,A7} | 20 red |
| Other rules, file mode | `no-role.js`, `two-roles.js`, `bad-role.js` {DR1}; `requires-legacy.js` {DR3} (`legacy-helper.js` has no Part A item); `bad-helper.js` {DR4,A1}; `bad-oracle.js` {DR5} (it calls `page.mouse.click`) | 6 red |
| Ledger, dir mode | `dirs/dr6-unlisted` {DR6}; `dirs/dr6-repaired` {DR6}; `dirs/dr7` (6 ledger rows, 6 legacy files) {DR7} | 3 red |
| Legal, file mode | `ok-driver.js`, `ok-oracle.js`, `zlib-driver3.js`, `png-pure.js`, `literal-key.js`, `index-ok.js`, `m2-audit-copy.js`, `var-key.js` (accepted residual, D:393), `tests/browser/m2-audit.js`, `tests/browser/t7-launch-options.js` | 10 green |
| Live tree, dir mode | `tests/browser` | 1 green |

40 rows. Self-test conditions, the reading of "with the set equality of section 3.4" (D:398): set equality for each row; each of DR1 to DR7 and A1 to A7 is in at least one expected set; the file has at least 40 rows (the tool fixes 40). So an empty or a cut cases file is red.

**Acceptance items closed:** A6, A7; first half of A21.

**Steps:**

- [ ] **Step 1:** Add the role lines (samples and `tests/browser/*.js`). Write the new samples, the three `dirs/`, the ledger, and `cases.tsv`.
- [ ] **Step 2:** `node --check` on each of the 12 `tests/browser/*.js` files, and `node --test tests/browser/m2-projection.test.js tests/browser/m2-audit.test.js tests/browser/t7-launch-options.test.js` (the CI command, `ci.yml:108`). Expected: exit 0. A role line must not change a test result.
- [ ] **Step 3: Red first.** Write `scripts/qa/driver_rule.py` with the role parser (DR1), the case loop, the set comparison and the self-test conditions only. No Part A table and no DR2 to DR7 yet. Run `python3 scripts/qa/driver_rule.py --self-test; echo exit=$?`. Expected: `exit=1`. Every red row fails with `want [...] got []`, except the three DR1 rows. The 11 green rows pass.
- [ ] **Step 4: Green.** Implement Part A (A1 to A7), Part B, and DR1 to DR7 (D section 4.4; port `docs/qa/probes/driver/dr_model3.py`). Run `--self-test`. Expected: `exit=0`, 40 of 40. Run `python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?`. Expected: `exit=0`.
- [ ] **Step 5: Commit** the explicit paths of steps 1 to 4 (the role lines included), before any trust run.
- [ ] **Step 6: Trust runs.** (a) In a scratch copy of the tool, remove the A7 check. Expected: `--self-test` `exit=1` on `plain-bad.js`, `fs-ticklog.js`, `mod-require.js`, `via-fs-pure.js`. (b) In a scratch copy of the tool, remove the DR6 check. Expected: `exit=1` on both `dr6` dirs. (c) In the tree, change the committed role line of `tests/browser/t7.js` to `driver`; run `driver_rule.py tests/browser`. Expected: `exit=1`, with a `DR2 A1` line on `t7.js` among others (`t7.js:213` calls `window.turts_move`). Restore with `git checkout -- tests/browser/t7.js`; then `git status --porcelain -- tests/browser/t7.js` is empty.
- [ ] **Step 7: CI.** Add to the `agents` job:

    ```yaml
          - name: Pixels-only driver rule
            run: python3 scripts/qa/driver_rule.py --self-test && python3 scripts/qa/driver_rule.py tests/browser
    ```

- [ ] **Step 8:** Run the CI command with `time`. Expected: exit 0, under 1 s. Commit `.github/workflows/ci.yml`.

---

### Task 4: Capability check, version table mechanism, typo hint, engine 0.2.2, client lockfile check

**Goal:** All Rust changes that do not need the library rule.

**Seat and model:** `mod-runtime-engineer`, Sonnet 5.5. `qa-lead` reviews the tests.

**Depends on:** nothing. Wave 1.

**Files:**

- Modify: `crates/turts_mod_api/src/lib.rs:33` (`ENGINE_API_VERSION = "0.2.2"`; add `RELEASED_ENGINE_APIS: &[&str] = &["0.1.0", "0.2.0", "0.2.1", "0.2.2"]` and `KNOWN_CAPABILITIES: &[&str] = &[]` next to it).
- Modify: `crates/turts_mod_runtime/src/manifest.rs:9-59` (capability rule after the `engine_api` check at `:34-42`).
- Create: `crates/turts_mod_runtime/src/version_table.rs` (private module, `pub(crate) fn check_version_table`, an in-file `#[cfg(test)]` module); modify `crates/turts_mod_runtime/src/lib.rs:3-11` (`mod version_table;`) and `crates/turts_mod_runtime/src/archive.rs:132-134` (call the check after `validate_m3`, with the real row list, which is empty in T4).
- Modify: `crates/turts_mod_runtime/src/parse.rs:20-22` (hint for `NoSuchStructField` only).
- Modify: `crates/turts_pack/src/main.rs:22-28` (one help line on capabilities); `crates/turts_pack/tests/cli.rs:99` (add `"capabilit"` to the checked words).
- Modify: `crates/turts_client/src/acquire.rs:50-56` (`check_lockfile(bytes, advertised_hash, advertised_mod_id)`); `crates/turts_client/src/join.rs:167-168` (pass the `mod_id` of `acquisition.advertisement`, as `advertised_lockfile_hash` does at `join.rs:199-205`); call sites `crates/turts_client/tests/acquire.rs:80,85,94` and `crates/turts_client/tests/t2d_client_hash.rs:67,81` (D:596-602).
- Tests: `crates/turts_pack/tests/bad_packages.rs` (new cases with `assert_refused`, `bad_packages.rs:66-76`; update the pinned text at `:87-97`); `crates/turts_mod_runtime/tests/t2c_package_rejects.rs:57` (`0.2.1` → `0.2.2`) and `:262-275` (pinned text gets the hint); `crates/turts_mod_runtime/tests/archive.rs` (SC1, SC2); `crates/turts_client/tests/acquire.rs` (A16); `crates/turts_mod_api` unit test for the version list.

**Fixed texts** (D gives the capability text; the others are plan choices, the cheapest form that names what D requires):

- Unknown capability, field `capabilities` (D:450): `unknown capability `scripts.server`; known: none`.
- Duplicate capability, field `capabilities`: `capability `scripts.server` is listed twice`. The duplicate check runs before the unknown check. With an empty `KNOWN_CAPABILITIES`, an unknown check first would hide the duplicate rule, and its test could not go red.
- Typo hint: the message `unknown field `helth` at line L column C` gets `; did you mean `health`?`. The hint names the allowed name with the smallest edit distance, only when that distance is 2 or less (ties: the first in the allowed list). No hint for an unknown variant (cheaper; D:553 asks for fields).
- Version table message. The error is `ModError::new(<manifest id>, "engine_api", ..).with_file(MANIFEST_PATH)`, so the stderr starts `zeta/manifest.ron: engine_api: ` (`crates/turts_mod_api/src/error.rs:46-62`). Form of the message (`docs/qa/probes/modrt/repair-modrt.txt:26`): ``<row name>` needs engine api <since_api>; `<requirement>` admits <v>`, where v is the highest released version below `since_api` that the requirement matches.
- Lockfile: `lockfile: package `<id>` has two entries` and `lockfile: the advertised package `<id>` has no entry`.

**Acceptance items closed:** A9, A16, A17, A18 (SC1), A20 first half (SC3). A19 (SC2) is a guard, not a test: no O1 build makes it red (Risk 4). It is not counted as closed.

**Steps:**

- [ ] **Step 1: Tests first.**
    - `bad_packages.rs`: `an_unknown_capability_is_refused` (edit `capabilities: []` → `capabilities: ["scripts.server"]`; exact stderr `zeta/manifest.ron: capabilities: unknown capability `scripts.server`; known: none`); `a_duplicate_capability_is_refused` (`["scripts.server", "scripts.server"]`; exact stderr with "listed twice"); update `a_field_typo_is_refused` (`:95-96`) to the text with `; did you mean `movement`?` before ` (allowed: ...)`.
    - `t2c_package_rejects.rs`: `:57` asserts `0.2.2`; `:272-273` gets the hint for `movenment`.
    - `archive.rs`: `sc1_typo_helth_names_health` (a unit file with `helth: Some((max: 5))`: the message holds `did you mean `health``, and `allowed` holds `health`); `sc1_far_name_gives_no_hint` (`zzzzzzzz`: no `did you mean`); `sc2_duplicate_unit_key_is_refused` (`health` twice: `duplicate field `health``).
    - `version_table.rs` in-file: `sc3_row_since_0_2_1_refuses_caret_0_2` and `sc3_row_since_0_2_1_accepts_caret_0_2_1`, with an injected row (D:555).
    - `turts_mod_api`: `released_list_ends_with_engine_version` (the last entry of `RELEASED_ENGINE_APIS` equals `ENGINE_API_VERSION`).
    - `cli.rs:99`: the help names capabilities.
- [ ] **Step 2: Signature first, so each new test compiles and can fail.**
    - Add `RELEASED_ENGINE_APIS` with `"0.2.2"` as its last entry, and `KNOWN_CAPABILITIES = &[]`. Keep `ENGINE_API_VERSION = "0.2.1"` until step 4.
    - Add `version_table.rs` with the row type and a stub `check_version_table` that returns `Ok(())`.
    - Change `check_lockfile` to take `advertised_mod_id: &str` and ignore it. Update the six call sites. Write `a_lockfile_with_two_entries_for_one_id_is_refused` and `a_lockfile_without_the_advertised_mod_id_is_refused` in `crates/turts_client/tests/acquire.rs`.
- [ ] **Step 3: Run red.** `cargo test -p turts_pack --test bad_packages --test cli; echo exit=$?`, `cargo test -p turts_mod_runtime --test t2c_package_rejects --test archive; echo exit=$?`, `cargo test -p turts_mod_runtime --lib version_table; echo exit=$?`, `cargo test -p turts_client --test acquire; echo exit=$?`, `cargo test -p turts_mod_api; echo exit=$?`. Every test compiles. Expected red, each with `test <name> ... FAILED`: the capability tests (exit 0 today, `docs/qa/probes/pack/runs-pack.txt` p1); the three pinned-text tests (the A17 red is `t2c_package_rejects.rs:57`, which now asserts `0.2.2` against engine 0.2.1); `sc1_typo_helth_names_health`; `sc3_row_since_0_2_1_refuses_caret_0_2` (the stub accepts); the two lockfile tests (accepted today, `acquire.rs:52-56`); `released_list_ends_with_engine_version` (last entry 0.2.2, engine 0.2.1); the help test. Expected green now, and why: `sc1_far_name_gives_no_hint` (guards a hint without a distance limit; T6 row 16 covers the hint); `sc3_row_since_0_2_1_accepts_caret_0_2_1` (guards a table that refuses everything); `sc2_duplicate_unit_key_is_refused` (the ron derive refuses it, `docs/qa/probes/pack/rig-pack2.txt` case `dup`; guard only, Risk 4).
- [ ] **Step 4: Implement.** Bump `ENGINE_API_VERSION` to 0.2.2. Add the capability rule. Fill `check_version_table` (row: name, presence test `fn(&Manifest) -> bool`, `since_api`; the definition of D:532-535). Add the hint. Add the two lockfile rules.
- [ ] **Step 5: Run green.** The five commands of step 3. Expected: exit 0. Then the merge commands of the execution contract, which include `cargo test --workspace --locked` (all `mods/` packages still load with `^0.2` and `^0.2.1`, A17) and the wasm32 check (`turts_client` changed).
- [ ] **Step 6:** Commit the explicit paths.

---

### Task 5: Library rule, resolver, the lift, call sites, and the two table rows

**Goal:** D sections 5.2 and 7, and the two real rows of D section 6.1.

**Seat and model:** `mod-runtime-engineer`, Sonnet 5.5. `qa-lead` reviews the tests.

**Depends on:** T4 (engine 0.2.2; `manifest.rs`, `main.rs`, `cli.rs`, `version_table.rs`). Wave 2.

**Files:**

- Modify: `crates/turts_mod_runtime/src/manifest.rs:44-59`. Remove `:44-49`. Add LR1 and LR2 through one shared function that `resolve` calls too. Make the map rule read `kind`: a Game exports exactly one map (the text of `:55` stays); a Library exports zero maps.
- Create: `crates/turts_mod_runtime/src/resolve.rs`; modify `crates/turts_mod_runtime/src/lib.rs:3-11` (`pub mod resolve;`).
- Modify: `crates/turts_mod_runtime/src/version_table.rs` (the two real rows, in this order: non-empty `dependencies` since 0.2.2; `kind: Library` since 0.2.2).
- Modify: `crates/turts_pack/src/lib.rs:13-14` (`resolve(&archive.manifest, &[])?` as the first step of `load_package`, before `archive.content()`); `crates/turts_pack/src/lib.rs:41-46` (`check_package` matches on `kind`: Game → `load_package` and `visuals_by_content`, as now; Library → `archive.content()` and `visuals_by_content`, no world, no `resolve`). **Constraint:** the `load_package` call must stay in the body of `check_package` itself, in a `match` arm. Do not move it into a helper function. The guard `crates/turts_checks/tests/m2_public_path.rs:36-39` requires the call inside `check_package`; its syn visitor finds a call in a `match` arm, not a call in a helper.
- Modify: `crates/turts_pack/src/main.rs:22-28` (one help line on libraries and dependencies); `crates/turts_pack/tests/cli.rs:99` (add `"librar"`).
- Tests: `crates/turts_mod_runtime/tests/resolve.rs` (new); `crates/turts_mod_runtime/tests/t2c_package_rejects.rs:71-79`; `crates/turts_pack/tests/bad_packages.rs`; `crates/turts_server/tests/unit_boot.rs` (boot helper `:52-57`, error pattern `:163`).

**`resolve` shape** (D section 7): `pub fn resolve(root: &Manifest, available: &[Manifest]) -> Result<Vec<PackageId>, ModError>`. Output: dependencies first, ties by id, root last. Each rule is one function, called on one line of `resolve`, in this order: `lr7_root_kind`, `lr1_fields`, `lr2_self`, `lr3_provided`, `lr4_version`, `lr5_library_only`, `lr6_acyclic`, `lr8_reachable`. One call per line lets T6 remove one rule with one `sed` line. Errors carry `with_file(MANIFEST_PATH)`. Each LR test asserts a word that only its own rule prints (for example `itself` for LR2, `cycle` for LR6), not only the field. LR1, LR2 and LR3 all use the field `dependencies`; a test on the field alone stays green when its rule is removed and LR3 fires.

**Fixed texts:**

- LR7, field `kind`: exactly `a library cannot be the root of a match` (D:575).
- LR3, field `dependencies`: `dependency `<id>` `<requirement>` is not provided` (D:571 asks for the id and the requirement). Two provided versions: `dependency `<id>` is provided <n> times`.
- Library with a map, field `exports.maps`: `a library exports no map, found <n>`.

**Note, traced at gate 2 (rigour G2; mod-runtime answer 2):** `turts-pack` packs `zeta` with a map file that the manifest does not export (exit 0, the file is in the tar). `pack.rs` checks no export list, and `validate_m3_presence` reads only the listed maps (`content_m3_validation.rs:606-650`). So the Library packages can keep `maps/arena.ron`.

**Test packages** (`bad_packages.rs` edits the `zeta` fixture, `bad_packages.rs:16-18,35-63`; D section 5.2 says "a copy of `mods/acme`". Plan choice: `zeta` through the existing helper. Both are one-map Game packages with `capabilities: []`; the helper exists and has an exact-stderr check.) Edits: `kind: Game` → `kind: Library`; `maps: ["maps/arena.ron"]` → `maps: []`; `engine_api: "^0.2"` → `"^0.2.2"`; `dependencies: {}` → `{"ai_core": "^1.0"}`.

**Acceptance items closed:** A10, A11, A12, A13, A14 (tests; the red is in T6), A15, A20 second half (SC4).

**Steps:**

- [ ] **Step 1: Tests first.**
    - `bad_packages.rs`, process tests:
        - `a10_library_with_a_map_is_refused`: Library, `^0.2.2`, map kept. Exact stderr `zeta/manifest.ron: exports.maps: a library exports no map, found 1`.
        - `a11_library_without_a_map_is_packed`: Library, no map, `^0.2.2`. Exit 0, `zeta.tar` exists.
        - `a11_library_with_dependencies_is_packed`: the same plus `{"ai_core": "^1.0"}`. Exit 0.
        - `a15_game_with_a_dependency_is_refused`: Game, `^0.2.2`, `{"ai_core": "^1.0"}`. Exact stderr `zeta/manifest.ron: dependencies: dependency `ai_core` `^1.0` is not provided`.
        - `sc4_library_needs_api_0_2_2`: Library, no map, `^0.2`. Exact stderr `zeta/manifest.ron: engine_api: `kind: Library` needs engine api 0.2.2; `^0.2` admits 0.2.1`.
        - `sc4_dependencies_need_api_0_2_2`: Library, no map, `^0.2`, `{"ai_core": "^1.0"}`. Exact stderr names `dependencies` (first row).
    - `bad_packages.rs`, in-process `a12_library_root_is_refused_by_load_package`: copy and edit `zeta` (Library, no map, `^0.2.2`), `turts_mod_runtime::pack::pack_dir`, `PackageArchive::parse(..).unwrap()`, then `turts_pack::load_package` on a `headless_app` world. `assert_eq!(error.message, "a library cannot be the root of a match")` (D:496-504). Factor the copy-and-edit part of `pack_edited` (`:35-57`) into a helper; do not copy it.
    - `t2c_package_rejects.rs:71-79` → `t2c_dependencies_parse_then_resolve_refuses`: `PackageArchive::parse` accepts `{"lib": "^1"}`; `resolve(&archive.manifest, &[])` gives field `dependencies` and a message with `` `lib` ``, `` `^1` `` and `not provided` (A13).
    - `unit_boot.rs`: `boot_refuses_a_root_with_a_dependency` (acme with `^0.2.2` and `{"ai_core": "^1.0"}`; `boot::boot(..).unwrap_err()` has field `dependencies` and `not provided`).
    - `resolve.rs`: one test for each of LR1 to LR8 on in-memory manifests, each asserting the field and a rule-specific word; plus `order_is_dependencies_first_then_id` (root `a` needs `b` and `c`, `b` needs `c` → `[c, b, a]`) and `a_root_without_dependencies_resolves_to_itself`.
    - `cli.rs:99`: the help names libraries.
- [ ] **Step 2: Stub first.** Add `pub mod resolve;` (`crates/turts_mod_runtime/src/lib.rs:3-11`) and a stub `pub fn resolve(root: &Manifest, available: &[Manifest]) -> Result<Vec<PackageId>, ModError>` that returns `Ok(vec![PackageId(root.id.clone())])`. Do not call it from `load_package` yet. Now every test of step 1 compiles.
- [ ] **Step 3: Run red.** `cargo test -p turts_pack --test bad_packages --test cli; echo exit=$?`, `cargo test -p turts_mod_runtime --test t2c_package_rejects --test resolve; echo exit=$?`, `cargo test -p turts_server --test unit_boot; echo exit=$?`. Expected red, each `FAILED`: A10 (exit 0 today, `runs-pack.txt` p2); both A11 (exit 2, "a package exports exactly one map at M1, found 0", `manifest.rs:55`, or "not supported at M1", `manifest.rs:47`); A12 (`parse` panics on the map rule); A13 (`parse` refuses with "not supported at M1", `manifest.rs:47`); A15 and the boot test ("not supported at M1"); both SC4 (they fail on `exports.maps`, not `engine_api`); the help test; in `resolve.rs`, the eight LR tests (the stub returns `Ok`) and `order_is_dependencies_first_then_id` (the stub returns `[a]`). Expected green now: `a_root_without_dependencies_resolves_to_itself` (a guard against a resolver that refuses every root).
- [ ] **Step 4: Implement** in the order of the Files list. Replace the stub.
- [ ] **Step 5: Run green.** The three commands of step 3. Expected: exit 0. Then the merge commands of the execution contract.
- [ ] **Step 6:** Commit the explicit paths.

---

### Task 6: Verify

**Goal:** Run every check on the integration branch, break each rule on purpose, and tick the acceptance list of D section 11 one by one.

**Seat and model:** `qa-lead` (test mode), Sonnet 5.5. `mod-builder` judges the six new `turts-pack` error texts (consumer lens, D sections 5 and 7).

**Depends on:** T1 to T5. Wave 4.

**Files:** none in the repository. The mutation script and the logs go to the session scratchpad. The receipt goes to `vault/Work Log/`.

**Steps:**

- [ ] **Step 1: Full run**, each command with `; echo exit=$?`:

    ```bash
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked
    time cargo test --workspace --locked
    bash scripts/boundary.sh
    bash scripts/acceptance.sh
    time python3 scripts/qa/yardstick.py --check
    time (python3 scripts/qa/completeness.py --self-test && python3 scripts/qa/completeness.py --enforce)
    time (python3 scripts/qa/driver_rule.py --self-test && python3 scripts/qa/driver_rule.py tests/browser)
    node --test tests/browser/m2-projection.test.js tests/browser/m2-audit.test.js tests/browser/t7-launch-options.test.js
    bash scripts/t7.sh
    cargo run -q -p turts_pack -- --help
    ```

    Expected: every `exit=0`. `boundary.sh` keeps its fixture red inside the script. `--enforce` prints `checked specs: 1`. `t7.sh` shows that a 0.2.2 client joins a 0.2.2 server with the role line in `t7.js`. The help names capabilities and libraries. Record the three `time` values (Risk 3).

- [ ] **Step 2: Mutation runs.** On a clean tree. For each row: apply the edit, run the command, read the output, restore with `git checkout -- <file>`, then `git status --porcelain` must be empty. A Rust red counts only when the output holds `test <name> ... FAILED`. A compile error is not a red: fix the mutation, not the test.

    | # | Edit | Command | Expected red | Item |
    | --- | --- | --- | --- | --- |
    | 1-8 | comment out the call line `^\s*lrN_` in `resolve.rs`, for N = 1 to 8 | `cargo test -p turts_mod_runtime --test resolve` | the LRN test | A14 |
    | 9 | comment out `lr7_root_kind` (row 7) | `cargo test -p turts_pack --test bad_packages a12` | `a12_...`, the text is "a package exports exactly one map, found 0" (`crates/turts_pack/src/lib.rs:20`) | A12 |
    | 10 | comment out the `resolve` line in `load_package` | `cargo test -p turts_pack --test bad_packages a15` and `cargo test -p turts_server --test unit_boot boot_refuses` | both: the package packs and boots | A15 |
    | 11 | remove the Library zero-map rule in `manifest.rs` | `cargo test -p turts_pack --test bad_packages a10` | `a10_...` | A10 |
    | 12 | remove the `kind: Library` row | `cargo test -p turts_pack --test bad_packages sc4_library` | `sc4_library_needs_api_0_2_2` | A20 |
    | 13 | remove the `dependencies` row | `cargo test -p turts_pack --test bad_packages sc4_dependencies` | `sc4_dependencies_need_api_0_2_2` | A20 |
    | 14 | remove the `since_api` comparison in `version_table.rs` | `cargo test -p turts_mod_runtime --lib version_table` | `sc3_..._refuses_caret_0_2` | A20 |
    | 15 | remove the capability rule | `cargo test -p turts_pack --test bad_packages capability` | both capability tests | A9 |
    | 16 | remove the hint | `cargo test -p turts_mod_runtime --test archive sc1` | `sc1_typo_helth_names_health` | A18 |
    | 17 | remove the two lockfile rules | `cargo test -p turts_client --test acquire lockfile` | both new lockfile tests | A16 |
    | 18 | change one TSV row of `docs/qa/yardstick-a.tsv` | `python3 scripts/qa/yardstick.py --check` | exit 1 | A1, A21 |
    | 19 | delete the M4 row of `tests/qa/completeness/cases.tsv` | `python3 scripts/qa/completeness.py --self-test` | exit 1, condition 1 | A4, A21 |
    | 20 | add `docs/superpowers/specs/zz-probe-design.md` (no coverage table) and its row `client: yes` | `python3 scripts/qa/completeness.py --enforce` | exit 1, R1 and R3 on that file | A5 |
    | 21 | set the role line of `tests/browser/t7.js` to `driver` | `python3 scripts/qa/driver_rule.py tests/browser` | exit 1, a `DR2 A1` line on `t7.js` | A6, A21 |
    | 22 | comment out the call line `^\s*lr3_provided` in `resolve.rs` | `cargo test -p turts_mod_runtime --test t2c_package_rejects t2c_dependencies` | `t2c_dependencies_parse_then_resolve_refuses` (`resolve` returns `Ok`) | A13 |
    | 23 | delete row 21 of `tests/qa/completeness/cases.tsv` | `python3 scripts/qa/completeness.py --self-test` | exit 1, condition 3 `20<21` | A5 |

    Row 20 creates a file: remove it with `rm`, not `git checkout`. Then `git status --porcelain` must be empty.

- [ ] **Step 3: Flakiness.** Run the three Python CI commands and `cargo test -p turts_pack --test bad_packages` a second time. Expected: the same result.
- [ ] **Step 4: Acceptance list.** Tick each item of D section 11 with its command, the exit code, and the red row of step 2:

    | Item | Green command (step 1) | Red seen |
    | --- | --- | --- |
    | A1 | `yardstick.py --check`; T1 awk counts 355 / 181 | row 18; T1 step 7 |
    | A2 | `--self-test` rows 1-4 | T2 step 2 |
    | A3 | `--self-test` row 5 | T2 step 2 (the empty tool passes it; the T2 trust runs and rows 6-17 show the parser refuses) |
    | A4 | `--self-test` | row 19; T2 step 4 |
    | A5 | `--enforce`, `checked specs: 1` | rows 20, 23; cases 19, 20, 21 |
    | A6 | `driver_rule.py tests/browser` | row 21 |
    | A7 | `driver_rule.py --self-test` | T3 steps 3 and 5 |
    | A8 | deferred to MC (can slip) | — |
    | A9 to A18, A20 | `cargo test --workspace` | T4 step 3, T5 step 3, rows 1-17 and 22 |
    | A19 (SC2) | `cargo test --workspace` | none: guard only, Risk 4 |
    | A21 | the three steps on the QA0 merge commit | rows 18, 19, 21 locally. The orchestrator checks the CI run after the push (`gh run list --branch main`). |

- [ ] **Step 5:** `mod-builder` reads the six texts of T4 and T5 as a modder, and says whether each one tells the modder what to change.

---

## Risks

1. **Driver rule gap: a key built in a variable.** `const k = "con" + "structor"; io[k][k]` passes Part A (D:746-748). `var-key.js` is a green case, so the gap is visible. Review catches it, not the check. The stronger fix (a separate Node process with the permission model) is NOT TRACED on Node 20.
2. **CI runner Python version: NOT TRACED.** The `agents` job has no `actions/setup-python` (`ci.yml:138-143`). Local Python is 3.12.4. Mitigation: standard library only, no feature newer than 3.8. The first push of T2 shows the result in under one minute.
3. **Run time of the new steps.** The three `agents` steps: the probes took 0.14 s together (D:646). The new Rust tests: NOT TRACED (D estimate 10 to 30 s in the `check` job). Process tests in `bad_packages.rs` start `turts-pack` once each; T4 adds two and T5 adds six. T6 step 1 records the times.
4. **SC2 cannot go red under O1.** The ron derive refuses a duplicate key. The only measured red build is the naive O2 dispatch (`docs/qa/probes/modrt/repair-modrt.txt:16`). SC2 is a guard, not a trusted test, until a build exists that makes it red.
5. **Live self-test cases.** Rows 1-4 read the real M1 to M4 specs. When MC or the M4 amendment adds a coverage table, the expected set of that row changes, and the self-test goes red until `cases.tsv` changes. This is intended, but the editor of the spec must know it.
6. **T5 is the largest task.** It holds the resolver, the library rule, two call sites and five test files. The repair entry of the budget is most likely here.
7. **The 0.2.2 bump splits clients.** A 0.2.1 client refuses a 0.2.2 server (`crates/turts_client/src/acquire.rs:41-45`). Server and browser build from one commit, so this is expected (D section 8).

## Doc updates for the orchestrator

1. `CLAUDE.md:154`: the `turts_scene` text lists only `cameras_3d` and `units`. The bridge also gives `clip_from_world`, `viewport_logical_rect`, and `ground_screen_center` for each unit (`crates/turts_client_web/src/lib.rs:412-416,471-479`).
2. `CLAUDE.md`, "Build and test commands": add `python3 scripts/qa/yardstick.py --check`, `python3 scripts/qa/completeness.py --self-test` / `--enforce`, and `python3 scripts/qa/driver_rule.py --self-test` / `tests/browser`. Add the role line rule for `tests/browser/*.js` and the ledger `tests/browser/legacy-bridge.tsv`.
3. `CLAUDE.md`, the `turts-pack` entry: a Library takes the library path (no map, no world, no `resolve`); a Game with dependencies is refused with LR3 at QA0 (K5).
4. `CLAUDE.md`, the CI sentence: add the three `agents` steps.
5. `CLAUDE.md`, the crate table: `turts_mod_runtime` adds `resolve`.
6. The design: D cites `QP/...` and the coverage probe paths in the scratchpad. After T1, the files are under `docs/qa/probes/` (T1 table). The design is gated; the orchestrator decides whether to re-point the paths.
7. Vault decision log: record K6 (the CEO changes the CORE list by a line in the log, after a `game-designer` proposal).
8. When the amended M4 spec passes gate 1, the same commit sets its `specs.tsv` row to `gate: gated` (D:221-222), and updates `cases.tsv` row 4.
9. The design table D:154 gives the QA0 registry row `gate: draft`. T2 writes `gated`, because D:3 records the gate 1 pass. The tool reads `gate` only for the exempt files, so behaviour does not change. The orchestrator can align D:154.
10. The design: the self-test has 21 rows, not 20 (D:233, D:262-263, A4). Row 21 `dirs/r1r3` is a plan addition (gate 2 rigour M2).

## Evidence for this plan

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| Coverage probe on M1-M4, `fx_green.md`, 5 mutations at `bfef807`, exit code read for each run | T1 step 4 and T2 rows 1-10 expected sets (D:114) | another set or exit | M1-M4 exit 1 {R1,R3}; green exit 0; mutations as D section 3.5 |
| `dr_model3.py` on the 20 samples, 3 green drivers, 5 `pure` files, `t7.js`, `m3-human.js` | T1 step 4; T3 evasion sets | a sample green or a legal file red | 22 RED, 8 GREEN |
| `dr_model3.py` on `m2.js`, `m3.js`, `m3-mobile.js` | DR6 stays green on the five ledger files | a ledger file with no Part A item | each has A1 plus others |
| heading-bounded `awk` on Y | A1 counts | other counts | 355, 181, 4 |
| `rtk proxy grep -rn "check_lockfile("` | the six call sites | another site | `join.rs:168`, `acquire.rs:80,85,94`, `t2d_client_hash.rs:67,81` |
| `rtk proxy grep -rln "unknown field"` | which tests pin the typo text | none | `bad_packages.rs:95-96`, `t2c_package_rejects.rs:272-273` pin exact text |
| read `ids.rs:14-27` | `ai_core` is a valid `PackageId`, so A15 reaches LR3, not LR1 | `_` refused | `_` allowed |
| read `manifest.rs`, `archive.rs`, `turts_pack/src/lib.rs`, `main.rs`, `acquire.rs`, `join.rs`, `parse.rs`, `boot.rs`, `ci.yml` | every `path:line` in the Files lists | a moved line | as cited |
| scratch git repository (revision 1): commit a new `cases.tsv` and a role-line edit of `t7.js`, then edit, `git checkout --`, `cmp` with a saved copy; control: `git checkout --` on an uncommitted new file | the commit-first restore rule of the execution contract | a restore that loses the role line, or a non-empty porcelain | both checkouts exit 0, both `cmp` exit 0, porcelain empty; control exit 1 |
| read `m3-human.js:1-8`, `m2_public_path.rs:25-38` | T3 DR6 reason; T5 `check_package` constraint | no direct Part A item; no guard | A7 at `:5`, A6 at `:6`; guard at `:37` |

**NOT TRACED:** the CI runner Python version; the run time of the new Rust tests; whether other `turts_checks` tests scan the whole repository (gate 2 read only `no_content.rs`, which reads `mods/` and a temporary `src`). Resolved at gate 2 and moved: the packer accepts an unexported map file (T5 note); no test pins a lockfile with two entries (mod-runtime answer 4).
