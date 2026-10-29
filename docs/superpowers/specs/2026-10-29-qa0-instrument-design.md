# QA0 instrument and schema groundwork — design

Date: 2026-10-29. Revision 3. Status: **DRAFT, passed gate 1 (2026-10-29): spec-rigour-reviewer
pass-with-changes, mod-runtime-engineer pass.** Revision 3 folds in the four non-blocking items of
the rigour final re-check. No call is confirmed.
Author: `qa-lead`. Implementer: a domain builder for each plan task. `qa-lead` writes or reviews
each instrument.
Commit of record for every `path:line` in this document: `f56125d`.

Revision 1 repairs the gate 1 findings: rigour (`fail`: B1, H1 to H5, M1 to M4, L1 to L3) and
mod-runtime (`pass-with-changes`: B1, H1 to H3, N1 to N4). The orchestrator made decisions 1 to 12
for this repair. Revision 2 repairs the re-check findings: rigour (`fail`: N1 to N5) and
mod-runtime (`pass-with-changes`: R1, R2), with orchestrator decisions 1 to 6 of the second pass.
Section 14 maps each finding to its fix.

Sources of record:

- `RTS-Engine-Project-Specifications.md` (S), version 0.8. Section 14: S:699-737. Terms: S:705-711.
  Row QA0: S:721. Player check rule: S:703. Libraries: S:229 (Decided), S:231 (Baseline).
  Client acquisition: S:302 (Baseline). Capability limits: S:304. D33 S:62, D35 S:64, D48 S:77.
- Gap register `docs/research/review-2026-10/00-gap-register.md` (R): row 0 R:93, process fixes
  R:114-118, the two `turts-pack` defects R:47-48 and R:146-148, finding F7 R:42.
- Yardstick: `docs/research/review-2026-10/inventory-engine.md` (Y), Part A, 181 CORE items.
- Probes from 2026-10-28. This design reuses them and does not rebuild them. Directory
  `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/qa-probe/`:
  the coverage probe `completeness_gate.py` with `fx_green.md` and five `fx_mut_*.md` files; the
  bridge probe `hook_ban.py` with `fx_blind.js`. Report: `docs/research/review-2026-10/gap-qa-lead.md:64-67`.
  The register calls these two probes "C1" and "C2". This document calls them "the coverage probe"
  and "the bridge probe".
- QA0 probes. Directory (QP below)
  `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/qa0/`.
  Revision 0: `runs-pack.txt`, `runs-c2.txt`, `runs-c2-detail.txt`, `runs-schema.txt`.
  Gate 1: `modrt-probe/`, `runs-modrt-o2.txt`, `rig-*.txt`. Revision 1: `repair-c1.txt`,
  `repair-selftest.txt`, `repair-driver.txt`, `repair-modrt.txt`.

### Calls for the CEO

Each call needs a CEO answer before gate 2. The recommendation is the lean of `qa-lead`.

| # | Call | Recommendation | Reason | Alternative and its cost |
| --- | --- | --- | --- | --- |
| K1 | How the schema grows (section 6). Needs decision. | **O1**: keep `deny_unknown_fields`. Add each new field with `serde(default)` and a patch bump. Add a version table that `PackageArchive::content()` checks, and a nearest-name hint for a typo. About 1 plan task. | The mod-runtime probe shows that O2 can reach parity with the derive, but each new field still needs four edits (`QP/gate-modrt.md` H1). O1 needs two edits for each field: the struct field and one table row. The ron derive already rejects a duplicate key (`QP/repair-modrt.txt:6`). | **O2**: a hand-written `Deserialize` with a field table for `UnitDef` and `Exports`. Parity measured: 20 of 20 unit files parse equal and round-trip equal (`QP/runs-modrt-o2.txt:1`). Cost: about 2 plan tasks and about 120 lines for each type, with no extra power. |
| K2 | What happens to the two red browser tests (section 4.6). | **Option C**: split the T7 test now into `t7-driver.js` and `t7-oracle.js`. Keep `m3-human.js` red in the ledger until MC. This is in the "can slip to MC" tier (section 10). | The split proves the harness on a real build, not only on a sample file. MC replaces the panel model (R:94), so MC discards any pixel locator for the current panels. No script and no CI step runs `m3-human.js` (section 12). | **Option A**: both files go in the ledger. About 0.5 plan task. The harness is then green only on sample files until MC. **Option B**: split both now. About 1 plan task more than C. MC discards the `m3-human` work. |
| K3 | What a `kind: Library` package can hold (section 5.2). S:229 is Decided: "Importing a library must not silently activate gameplay". | **A Library exports zero maps. A Library never boots as the root of a match. A Library can export definitions (units, resources, and the other content kinds).** | A map holds placements, and a boot installs them. That is gameplay activation. Strict now, open later: a later manifest rule can permit a map library. | A Library can export maps but never boots as root. Then the register case (a Library with units and a map) exits 0 on the library path, and the defect R:47-48 stays open as written. |
| K4 | Do QA0 and MS carry a coverage table and a `Player path` section? S:703 says that they have no player check. S:709 says that each milestone spec has a `Player path`. | **No.** The gate does not apply R1 to R3 to a spec with `client: no`. | QA0 and MS put zero player items in scope. A table of 181 `later` rows is a form, not a check. | Yes: QA0 and MS each carry 181 rows. The rows make no defect visible. |
| K5 | Does QA0 make the server serve libraries? | **No. The server serves libraries at MP2** (bot framework, S:819). | `HostConfig` holds one `game_package` (`crates/turts_server/src/config.rs:68`). A server with many packages is new work, and QA0 has no consumer for it. **Effect that the CEO must see:** at QA0 the server replaces the refusal "not supported at M1" with a resolver check that always fails for a root with dependencies (LR3, "not provided"). | Yes: about 1 more plan task (config, boot, HTTP routes, lockfile order). |
| K6 | Who can change the CORE list after QA0 makes it a CI input. | **The CEO, by a line in the vault decision log, after a `game-designer` proposal.** | The list sets the bar for every milestone gate. An edit that removes an item turns a red gate green. | Any seat changes it in a reviewed commit. This is faster, but the bar can drift without the CEO. |

## 1. Contract and status

QA0 is the row S:721. The row text: "The completeness gate and the pixels-only test driver go into
CI before any new milestone is gated. Fix the two `turts-pack` validation defects. Open the schema
for additive growth. Replace the 'dependencies not supported at M1' refusal with a real library
check." The CI gate of the row: "the completeness gate is red on M1 to M4 as they stand and green
on the QA0 fixture". QA0 has no client and no player check (S:703).

### 1.1 Included

1. Completeness gate: a machine-readable item list, a milestone order file, a spec registry, a
   repository tool, the QA0 fixture, mutations, and a self-test. In CI.
2. Pixels-only driver rule: one role for each test file, a driver contract, an oracle rule, a
   static check, and a ledger. In CI. The T7 split if K2 option C stays in scope (section 10).
3. `turts-pack`: refuse an unknown capability. Apply the K3 rule to `kind: Library`.
4. The schema growth mechanism of K1. QA0 makes no new modder field valid.
5. A dependency resolver. The manifest check stops the refusal of dependencies. `load_package`
   calls the resolver, so the server boot and the Game path of `turts-pack` call it. The client
   checks the lockfile without a resolver.
6. `ENGINE_API_VERSION` goes from 0.2.1 to 0.2.2 (section 8).

### 1.2 Excluded

MC, MP1 and every player feature. A pixel locator for the current HUD panels. The library format:
how a game uses library content, exports, entry points, and scripts. Library content in the
catalog. A server that serves more than one package (K5). A `--lib` option for `turts-pack`
(orchestrator decision 5). Removal of the test bridges from player builds
(`crates/turts_client_web/src/lib.rs:78-79`). Any field from the 35-field probe
(`docs/research/review-2026-10/gap-mod-runtime-engineer.md:128`). Performance budgets.

### 1.3 Decision IDs and statuses this design depends on

D33 (S:62, Decided: libraries are packages that can depend on libraries). D35 (S:64, Decided:
exact lockfiles, one library version for each configuration). D48 (S:77, Decided: AI libraries).
D15 (Decided: versioned mod API, not Bevy internals). D43 (Decided: agent-first tools, built for a
real need). S:229 (Decided: no silent activation). S:231 (Baseline: nine manifest fields; reject
missing dependencies and cycles). S:302 (Baseline: the client never resolves versions; it does not
download server-only packages). The section 14 order (S:701, Decided, G10). The version scheme stays
Open: semver is the M1 placeholder (`crates/turts_mod_runtime/src/manifest.rs:8`).

## 2. Verified baseline

| Fact | Evidence |
| --- | --- |
| No production code reads `Manifest.capabilities` or `Manifest.kind`. | `rtk proxy grep -rn "capabilities\|PackageKind" crates`: hits are `crates/turts_mod_api/src/schema.rs:13,107,112`, `crates/turts_mod_api/src/lib.rs:25`, tests, and `turts_checks/examples`. |
| `turts-pack` accepts `capabilities: ["scripts.server"]` with exit 0. | `QP/runs-pack.txt` case p1, on a copy of `mods/acme`. |
| `turts-pack` accepts `kind: Library` with units and a map with exit 0. | `QP/runs-pack.txt` case p2. |
| `turts-pack` refuses a Library with no map: "a package exports exactly one map at M1, found 0", exit 2. The rule is inverted: the gameplay library passes, and the pure library fails. | `QP/runs-pack.txt` case p6; `manifest.rs:51-59`. |
| `validate_manifest` refuses any dependency: "dependencies are not supported at M1", exit 2. A test pins this text. | `QP/runs-pack.txt` case p3; `manifest.rs:44-49`; `crates/turts_mod_runtime/tests/t2c_package_rejects.rs:71-79`. |
| `validate_manifest` has one call site: `PackageArchive::parse`. So server, client and `turts-pack` all get a manifest rule. | `crates/turts_mod_runtime/src/archive.rs:42` (`QP/gate-modrt.md`, grep). |
| An unknown field fails with the allowed names. There is no nearest-name hint. | `QP/runs-pack.txt` cases p4, p5; `crates/turts_mod_runtime/src/parse.rs:20-23`. |
| The ron derive rejects a duplicate field. | `QP/rig-pack2.txt` case `dup`, exit 2; `QP/repair-modrt.txt:6`. |
| 46 schema types carry `deny_unknown_fields`. | `rtk proxy grep -c deny_unknown_fields crates/turts_mod_api/src/schema.rs` = 46. |
| The M3 version gate is hand-written for one milestone. It tests one version, 0.2.0. | `crates/turts_mod_runtime/src/content_m3_validation.rs:511-520`. |
| `PackageArchive::content()` runs on the server path and the client path. | `archive.rs:91-135`; `crates/turts_pack/src/lib.rs:14`; `crates/turts_client/src/content.rs:39` (`QP/gate-modrt.md` H2). |
| The released engine API versions are 0.1.0, 0.2.0, 0.2.1. | `git log -G` on `crates/turts_mod_api/src/lib.rs`: commits `13c472a`, `119773d`, `e534a03`. |
| The client drops server-only packages before it fetches. The lockfile entry has no `dependencies` or `kind`. | `crates/turts_client/src/acquire.rs:60-65`; `crates/turts_mod_api/src/lock.rs:15-24`. |
| The client checks only the hash and the parse of the lockfile. | `acquire.rs:52-56`. |
| The `turts_scene` bridge already gives the camera: `clip_from_world`, `viewport_logical_rect`, and a `ground_screen_center` for each unit. | `crates/turts_client_web/src/lib.rs:412-416,471-479`. `CLAUDE.md` describes only `cameras_3d` and `units`; that text is out of date. |
| The client takes the map of the first package that has one. A Library root fails on the client only because it has no map. | `acquire.rs:83-84,98-103`. |
| `HostConfig` has no `deny_unknown_fields`. A `libraries:` key in `server.ron` is silently ignored today. | `crates/turts_server/src/config.rs:65-75` (`QP/gate-modrt.md` N4). |
| Y Part A has 181 CORE rows: 177 plain and 4 with a note in the tier cell. | `awk` count, section 12. |
| None of the M1 to M4 specs has a `Yardstick coverage` heading or a `Player path` heading. | `rtk proxy grep -c -i -E "^#+ .*(yardstick coverage\|player path)"` gives 0 for each file. |
| The coverage probe gives exit 1 with the rule set {R1, R3} on each of M1 to M4. It gives exit 0 on `fx_green.md`. It gives exit 1 on each of the five mutations. | `QP/repair-c1.txt`. The revision 0 receipt `runs-c1.txt` showed `exit=0` for the mutations, because a pipe through `tail` replaced the exit code. |
| The bridge probe is red on 7 of 12 browser files, `t7.js` (4 uses) and `m3-human.js` (4 uses) included. | `QP/runs-c2.txt`, `QP/runs-c2-detail.txt`. |
| The T7 test sends its order through the bridge, not the pointer. | `tests/browser/t7.js:213`: `window.turts_move(x, y)`. |
| `m3-human.js` finds each control through the `turts_ui` bridge. | `tests/browser/m3-human.js:7,27,35,43` (`clickRenderedUiAction` from `m2.js`). |
| The client sends `MoveTo` on a right-click on the ground. | `crates/turts_client/src/input.rs:44-45,80-88,133-140`. |
| All units have one material. The T7 map has one unit. | `crates/turts_client/src/render.rs:405`; `mods/acme/maps/field.ron:4` (`QP/gate-rigour.md`, scope). |
| `PackageArchive.files` is private, and `parse` is the only constructor. A test outside the crate must enter through `parse`. | `crates/turts_mod_runtime/src/archive.rs:28,35`. |
| `check_lockfile` has six call sites. | `crates/turts_client/src/join.rs:168`; `crates/turts_client/tests/acquire.rs:80,85,94`; `crates/turts_client/tests/t2d_client_hash.rs:67,81` (grep). |
| `turts_debug` gives `command_results`; an applied command has `applied_tick`. | `crates/turts_client/src/debug.rs:175,196-197`. |
| `scripts/t7.sh` starts the server with `--tick-log`. | `scripts/t7.sh:52`. |
| `m2-audit.js` requires `node:fs` and sets `module.exports`. `t7-launch-options.js` reads `process.env` and sets `module.exports`. | `tests/browser/m2-audit.js:4,286`; `tests/browser/t7-launch-options.js:24,36`. |
| Python 3 runs in the `check` job of CI. The `agents` job builds no Rust. | `.github/workflows/ci.yml:37` (job `check`); `ci.yml:138-143` (job `agents`). |

## 3. Completeness gate

### 3.1 Files

| File | Content |
| --- | --- |
| `docs/qa/yardstick-a.tsv` | One row for each Part A item: `id`, `area`, `name`, `tier`, `tier_note`, `kind`, `deps`. 355 rows. 181 rows have `tier` = `CORE`. The tool extracts it from Y. |
| `docs/qa/milestones.tsv` | The milestone order of S:716-737: `order`, `id`. 20 rows, M1 to M13, QA0, MC, MP1, MP2, M4b, MS, MB. |
| `docs/qa/specs.tsv` | The spec registry: `spec`, `milestone`, `client` (`yes`, `no`, or `n/a` for a document that is not a milestone spec), `gate` (`draft`, `gated`, `done`). A commit that adds a spec adds its row. R9 limits the values `no` and `n/a` (section 3.3). |
| `scripts/qa/yardstick.py` | `--extract` writes the TSV from Y. `--check` extracts to a temporary file and compares the bytes with the committed TSV. |
| `scripts/qa/completeness.py` | The gate. A port of the coverage probe, with rules R5 to R9 added. Modes: `--self-test` and `--enforce`. Options `--specs-dir <dir>` (default `docs/superpowers/specs`) and `--registry <file>` (default `docs/qa/specs.tsv`). Python standard library only. |
| `tests/qa/completeness/green.md` | The QA0 fixture (S:711): a port of `fx_green.md`, with dispositions in the new syntax. |
| `tests/qa/completeness/mut-*.md` | One file for each mutation (section 3.5). |
| `tests/qa/completeness/cases.tsv` | One row for each self-test case: `mode`, `file`, `as_milestone`, `expected_rules`. `mode` is `spec` or `enforce-dir` (section 3.4). `expected_rules` is a comma list of rule ids, or `-` for a green case. `as_milestone` is `-` for an `enforce-dir` row. |
| `tests/qa/completeness/dirs/<name>/` | One directory for each `enforce-dir` case. It holds a `specs/` directory and a `specs.tsv`. |

The TSV is the gate input. Y stays the research record. `--check` keeps the two equal. K6 controls
changes.

Registry rows at QA0:

| spec | milestone | client | gate |
| --- | --- | --- | --- |
| `2026-10-26-m1-foundation-design.md` | M1 | yes | done |
| `2026-10-26-m2-economy-design.md` | M2 | yes | done |
| `2026-10-27-m3-industrial-design.md` | M3 | yes | done |
| `2026-10-28-m4-terrain-design.md` | M4 | yes | draft |
| `2026-10-29-qa0-instrument-design.md` | QA0 | no | draft |
| `2026-10-25-studio-agent-team-design.md` | - | n/a | - |
| `2026-10-26-m2-authoring-example.md` | - | n/a | - |

### 3.2 Spec format

A milestone spec with `client: yes` carries three parts:

- `## Yardstick coverage`: a table `| id | disposition | test |`, with one row for each CORE item.
- `## Player path`: numbered steps.
- An acceptance section: a heading that contains "Acceptance", with a table whose first cell is the
  test id.

Dispositions:

- `in`: this milestone delivers the item.
- `done:<repository path>`: an earlier milestone delivered the item. The path is the test that
  shows it.
- `later:<milestone id>`.
- `mod`: the item is mod policy. The engine exposes the hook in another item.
- `excluded:D<nn>`: a register row excludes the item, for example D10.

The coverage probe used `excluded:S:<line>`. Spec line numbers drift: line 207 of version 0.7 is
line 229 of version 0.8. A decision id does not drift.

### 3.3 Rules

Each red line names the rule, the file, and the item id.

| Rule | Red when |
| --- | --- |
| R1 | A CORE item has no row or two rows. Or a row id is not in the TSV. Or the disposition does not parse. |
| R2 | An `in` row has no test id. Or the id is not the first cell of a row in the acceptance section. The coverage probe accepted the id anywhere in the section text, so a prose mention such as "H9 is not used" passed. |
| R3 | The spec has no `## Player path` section with at least one numbered step. |
| R4 | An `in` item at milestone X has a CORE dependency that is not `in` at a milestone at or before X, and not `done:` in a spec at or before X. |
| R5 | `later:Y` names a milestone that is not in `milestones.tsv`, or that is not after X. |
| R6 | `excluded:Dnn` names a row that is not in S section 2 (pattern `^\| Dnn \|`). |
| R7 | `done:<path>` names a path that does not exist. |
| R8 | A spec in `specs.tsv` does not exist. Or a file in the specs directory has no row in `specs.tsv`. |
| R9 | A row has `client: no` and its file is not in the literal list `NO_CLIENT` of the tool. Or a row has `client: n/a` and its file is not in the literal list `NOT_MILESTONE` of the tool. |

The tool names every list by file path, never by the `milestone` value, because the author of a
row sets that value. R9 lists at QA0:

- `NO_CLIENT` = {`2026-10-29-qa0-instrument-design.md`}. S section 14 marks two milestones "No
  client, no player check": QA0 (S:721) and MS (S:728). The MS spec does not exist yet. The commit
  that adds the MS spec adds its file path to the list.
- `NOT_MILESTONE` = {`2026-10-25-studio-agent-team-design.md`,
  `2026-10-26-m2-authoring-example.md`}.

A change to either list is a change to the tool, and a reviewer sees it. Every other spec is
`client: yes`. So an author cannot skip R1 to R3 by a registry value.

### 3.4 Modes

**`--enforce`** runs R1 to R8 on each spec with `client: yes`, and R4 to R8 on each spec with
`client: no` (K4). It runs R9 on the registry. Options `--specs-dir <dir>` and `--registry <file>`
select the inputs (defaults: `docs/superpowers/specs` and `docs/qa/specs.tsv`). It skips one
closed set: the exempt specs. The tool fixes the exempt set in its code, by file path:
`2026-10-26-m1-foundation-design.md`, `2026-10-26-m2-economy-design.md`,
`2026-10-27-m3-industrial-design.md`, and `2026-10-28-m4-terrain-design.md`, while the `gate` of the
row is not `gated`. So a new spec that claims the milestone `M4` in its row is not exempt. The set
can shrink. It cannot grow without a reviewed edit of the tool. The mode prints the count of checked specs. At
QA0 the count is 1 (the QA0 spec, R4 to R8 only).

The CI step is the gate. A new spec with `client: yes` must be green when its commit adds it to
the registry, or the merge is blocked. The `gate` column records the gate 1 result for people. The
tool reads it only to end an exemption. When the orchestrator records the gate 1 pass of the
amended M4 spec, the same commit sets `gate: gated`. From that commit, `--enforce` checks M4.

**`--self-test`** runs every row of `cases.tsv`. A row with `mode: spec` runs the gate on `file` as
milestone `as_milestone`. A row with `mode: enforce-dir` runs `--enforce --specs-dir
<file>/specs --registry <file>/specs.tsv`. A case passes only when the set of rule ids in
its red lines is equal to the expected set. A subset is a failure, and a superset is a failure. So
a mutation that loses one of its rules, or that goes red for a wrong extra rule, turns the
self-test red. The self-test also turns red in each of these conditions:

1. An exempt spec has no case that expects it red.
2. A rule of R1 to R9 has no case that expects it.
3. The file has fewer rows than the count that the tool fixes (20 at QA0).

The live specs M1 to M4 are cases with the expected set {R1, R3}. So the self-test proves that the
gate is red on M1 to M4 as they stand (S:721) on each CI run.

Orchestrator decision 8 says "every registry spec with `gate` not `done`". Under decision 7, M1 to
M3 have `gate: done` and M4 has `gate: draft`, and S:711 requires red on all four. This design
reads the decision as "every exempt spec". Section 14 records this reading.

Model run (`QP/repair-selftest.txt`): an empty cases file is red on conditions 1 to 3. A case file
in which `fx_mut_a` expects only {R1} is red, because the probe gives {R1, R4}. A case file
without the M4 row is red on condition 1. The probe cases with the rule list cut to R1 to R4 are
green. With all of R1 to R9 the probe cases are red, because the probe has no R5 to R9 case. The
rigour final re-check ran a model with R1 to R9 and 20 rows: the empty file, the file without R9,
and a green `dirs/r9` are red; the full file is green (`QP/rig-final-selftest.txt`).

### 3.5 Mutations

The five probe mutations carry over, with their measured rule sets (`QP/repair-c1.txt`):
`mut-a` drops PC02 {R1, R4}; `mut-b` OR24 has no test {R2}; `mut-c` no player path {R3};
`mut-d` CA01 deferred {R4}; `mut-e` PC02 bound to the absent H9 {R2}.

QA0 adds seven: `mut-f` the test id is only in prose {R2}; `mut-g` `later:` an earlier milestone
{R5}; `mut-h` `excluded:D99` {R6}; `mut-i` `done:` a missing path {R7}; `mut-j` a duplicate row
{R1}; `mut-k` an id that is not in the TSV {R1}; `mut-l` a bad disposition {R1}.

Two `enforce-dir` cases: `dirs/r8` holds one spec file with no registry row {R8}; `dirs/r9` holds
a registry that marks an MC spec `client: no` {R9}. The `client: no` case is a copy of the QA0
fixture with no `Player path`, run as milestone QA0, and it is expected green. So the cases file
holds 20 rows: 4 live specs, the QA0 fixture, 12 mutations, the `client: no` case, and the two
`enforce-dir` cases.

### 3.6 Test and red build

- Today the tool, the TSV, the registry and the cases are not in the repository. CI has no step.
- The live red: M1 to M4 have no coverage heading and no player-path heading (section 2). So the
  coverage probe gives {R1, R3} on each (`QP/repair-c1.txt`). The port must give the same sets.
- A wrong instrument build: a gate that exits 0 on any of M1 to M4, or on any mutation. The
  self-test turns red on that build.
- Trust run in the plan: port the tool, run the self-test, and record red and green. Then remove
  R2 and R4 in a scratch copy of the tool, and record the self-test red.

## 4. Pixels-only driver rule

S:710: "a browser test driver that uses only pointer events, key events, and screenshots. Test
bridges (for example `turts_ui`, `turts_move`) belong to the test oracle only."

### 4.1 Terms

One meaning each, in this document:

- **Driver**: a file with role `driver`. It acts as the player. It gets only the `io` object.
- **Oracle**: a file with role `oracle`. It decides pass or fail. It reads bridges, server output,
  and the harness input log. It sends no input.
- **Harness**: a file with role `harness`. It starts the browser, builds `io` for the driver and
  `view` for the oracle, and runs both. The harness does not decide pass or fail.
- **Driver sample**: a file under `tests/qa/driver/` that the self-test of the driver rule runs.

### 4.2 Roles

Every file `tests/browser/*.js` and `tests/browser/harness/*.js` (not `node_modules`) starts with
one line: `// turts-test-role: <role>`. A file has one role. Roles: `driver`, `oracle`, `harness`,
`instrument`, `unit`, `pure`, `legacy-bridge`. A `pure` file is a local helper that a driver can
require. DR4 applies Part A of section 4.4 to it.

Roles at QA0, with the bridge probe count:

| File | Role (K2 option C) | Role (K2 option A) | Bridge uses |
| --- | --- | --- | --- |
| `t7.js` | removed: its launch code moves to `harness/run.js` | `legacy-bridge` | 4 |
| `t7-driver.js` (new) | `driver` | not created | n/a |
| `t7-oracle.js` (new) | `oracle` | not created | n/a |
| `harness/run.js`, `harness/driver-io.js` (new) | `harness` | `harness` | n/a |
| `m3-human.js` | `legacy-bridge` | `legacy-bridge` | 4 |
| `m2.js`, `m3.js`, `m3-mobile.js` | `legacy-bridge` (M2 and M3 had no player check, S:714) | same | 24, 6, 2 |
| `measure.js`, `spike_p8.js` | `instrument` | same | 2, 0 |
| `m2-projection.test.js`, `m2-audit.test.js`, `t7-launch-options.test.js` | `unit` | same | 7, 0, 0 |
| `m2-audit.js`, `t7-launch-options.js` | `pure` | same | 0, 0 |

### 4.3 Driver and oracle contracts

A driver sets `exports.drive = drive`. The harness owns the Playwright page. It gives the driver
only `io`:

    io.click(x, y, button)
    io.drag(x0, y0, x1, y1, button)
    io.key(code, modifiers)
    io.screenshot()        // PNG bytes
    io.save(name, bytes)   // write an artefact
    io.wait(ms)

`harness/driver-io.js` builds `io` from `page.mouse`, `page.keyboard` and `page.screenshot` only.
It writes each `io` call to the input log (call, arguments, wall time). The driver cannot write to
the input log. The driver never holds `page`.

An oracle sets `exports.judge = judge`. The harness gives the oracle only `view`:

    view.bridge(name)      // calls window[name]() for turts_debug, turts_scene, turts_ui, turts_memory_bytes
    view.inputLog()        // the io calls of the driver
    view.screenshot()      // PNG bytes
    view.serverFile(path)  // a server output file, for example the tick log

`view.bridge` refuses `turts_move`, because that bridge sends an order.

### 4.4 Static check `scripts/qa/driver_rule.py`

The rule is about the bridges and about dynamic code. It is not about Node built-ins. The check
removes `//` comments, then tests identifiers with identifier boundaries. For a driver it follows
each literal local `require` (`./x` or `../x`; it adds `.js` when the name has no extension). The
driver and all the files that it requires, directly or through other files, are its **require
closure**.

**Part A. Banned in a driver and in each file of its require closure:**

| Item | Banned |
| --- | --- |
| A1 | A bridge name: `turts_ui`, `turts_move`, `turts_debug`, `turts_scene`, or any other `turts_` identifier. |
| A2 | Dynamic code: `eval`; the Function constructor, written as `Function` (so `new Function`), as a `.constructor` member, or as `["constructor"]`. |
| A3 | `global` or `globalThis`. |
| A4 | A computed member access with a key that is not one string literal, on `process`, `global`, `globalThis`, `window` or `module`. Also the `process` members that load code: `binding`, `_linkedBinding`, `dlopen`, `mainModule`, `getBuiltinModule`. Also any `module` member other than `exports`, so `module.parent`, `module.children`, `module.require` and `module.constructor` are red (`module.parent` reaches the exports of the requiring module: rigour run F3, Node v24.10.0). `require.cache` and `require.main` are red by A5. Also, on any object, a computed key that joins strings (`+` next to a string literal, or a template literal with `${`). |
| A5 | `import(`. A `require` whose argument is not one string literal. `require` used as a value (an alias). |
| A6 | A second channel to the browser or the server: the built-ins `child_process`, `vm`, `worker_threads`, `inspector`, `module`, `net`, `http`, `https`, `http2`, `dgram`, `cluster`, `repl`; the globals `fetch` and `WebSocket`; an npm package that is not in the literal allow list of the tool (empty at QA0; `playwright-core` is never on it). |
| A7 | The file system: `node:fs`, `node:fs/promises`, `fs`. A driver could read the server tick log and click where the log puts the unit (rigour run F1). The driver writes screenshots through `io.save` (section 4.3). DR2 applies A7. DR4 does not apply A7, so `m2-audit.js` (`pure`, `node:fs` at `tests/browser/m2-audit.js:4`) stays green; a driver that requires it is red by DR2 through its closure. |

A6, the `process` members in A4, and the string-built key in A4 are not in orchestrator decision
3. They close three routes. A driver can run code without `eval` (`vm`,
`process.getBuiltinModule`). It can drive a second browser (`playwright-core`). It can send orders
to the server directly (`fetch`, `WebSocket`, `net`).

**Part B. Allowed:** every other Node built-in by its literal name, `node:zlib` included (the
driver decodes PNG). A local file with role `driver` or `pure`, if the file passes Part A. A literal
member access, for example `process.env` and `module.exports`.

| Rule | Red when |
| --- | --- |
| DR1 | A file has no role line, two role lines, or an unknown role. |
| DR2 | A driver, or a file of its require closure, has a Part A item. The red line names the file and the item. |
| DR3 | A driver requires a local file whose role is not `driver` or `pure`. So `./m2.js` (`legacy-bridge`) is red. |
| DR4 | A `pure` file has a Part A item other than A7. A `pure` file that uses `node:fs` is legal, but no driver can require it (DR2, A7). |
| DR5 | An `oracle` contains `mouse`, `keyboard`, `touchscreen`, `dispatchEvent`, `turts_move`, `evaluate`, or `page`. Or it has a Part A item A2 or A5. Or it requires `playwright-core`. |
| DR6 | A `legacy-bridge` file is not in `tests/browser/legacy-bridge.tsv`. Or it is in the ledger and has no Part A item: a repaired file must become a `driver`. |
| DR7 | `legacy-bridge.tsv` has more rows than the count that the tool fixes (5 at QA0 under option A, 4 under option C). The ledger can shrink. It cannot grow without a reviewed edit of the tool. |

Model run (`QP/repair3-driver.txt`; model `QP/repair-driver/dr_model3.py`):

- Red, 20 of 20 driver samples. The 14 samples of revision 2: `eval("req" + "uire")` (A2); `[].constructor.constructor` (A2);
  `import("node:fs")` (A5); `globalThis.require` (A3, A5); `require("node:" + "fs")` (A5);
  `window["turts_" + "move"]` (A4); `const r = require` (A5); `process.binding` (A4); `fetch(...)`
  (A6); the gate 1 re-check sample `global["pro" + "cess"]["mainModule"]["req" + "uire"]` (A3, A4);
  a driver that requires a `pure` file that calls `window.turts_move` (A1, through the closure);
  `require("playwright-core")` (A6); `process.getBuiltinModule` (A4);
  `io["con" + "structor"]["con" + "structor"]` (A4, string-built key). Revision 3 adds six:
  `require("node:fs")` alone (A7); the rigour sample that reads the tick log (A7);
  `module.parent` (A4); `require.cache` (A5); `module.require("node:fs")` (A4, A7); a driver that
  requires a copy of `m2-audit.js` (A7, through the closure).
- Green, drivers: `ok-driver.js`; a driver that saves through `io.save` and requires the `pure`
  PNG file on `node:zlib`.
- Green, `pure` files under DR4: `m2-audit.js` (`node:fs`, `module.exports`),
  `t7-launch-options.js` (`process.env`, `module.exports`), the PNG file, `process["env"]`, and
  plain index keys `o[k]`, `a[i + 1]`.
- Green, the accepted residual: a key built in a variable (`var-key.js`). Review covers it.
- Control: `t7.js` and `m3-human.js` (through `m2.js`) are red.

Driver samples in `tests/qa/driver/`: `ok-driver.js` and `ok-oracle.js` (green), and the green
samples of the model run. Also one red sample for each rule DR1 to DR7 and for each Part A item. A
`--self-test` mode runs the samples with expected rule sets, with the set equality of section 3.4.

### 4.5 Test and red build

- Today the bridge probe is red on `t7.js:213` and on `m3-human.js:7,27,35,43`
  (`QP/runs-c2-detail.txt`). The probe also flags `t7.js:134-135,152`. Those lines read the
  bridges for the result. Under option C they move to `t7-oracle.js`. DR5 permits them there.
- A wrong instrument build: a check that is green on any red sample. The self-test turns red.

### 4.6 The two red tests (K2)

| Option | What happens | Cost |
| --- | --- | --- |
| A. Both stay red in the ledger | `t7.js` and `m3-human.js` get role `legacy-bridge`. DR6 keeps them in the ledger until MC rewrites them. | About 0.5 plan task, inside the driver rule task. Risk: T7, the M1 acceptance, keeps its order on a bridge (`t7.js:213`). The harness is green only on samples until MC. |
| B. Both split now | `t7-driver.js` and `m3-human-driver.js` find targets in screenshots. | About 3 plan tasks. MC replaces the panel model (R:94), so MC discards the `m3-human` pixel locator. |
| C. Split T7 only (recommended) | `t7-driver.js` finds the unit by its colour in `io.screenshot()`, left-clicks it, and right-clicks a ground pixel. `t7-oracle.js` decides. `m3-human.js` stays in the ledger. | About 2 plan tasks: the harness, and the split. The scene bridge already gives the camera (section 2). A PNG decoder: a small decoder on `node:zlib` in a `pure` file, or one pinned npm package. |

The colour search is safe at QA0 for one reason only: the T7 map has one unit, and all units share
one material (section 2). A map with more units needs team colours. Team colours are MC scope
(S:722).

Option C oracle. `t7-oracle.js` passes when all four conditions are true:

1. The input log holds one right-click at pixel (px, py) after the left-click.
2. `view.bridge("turts_debug")` gives `command_results` with one entry that has `applied_tick`, and
   that tick is after the right-click (`crates/turts_client/src/debug.rs:175,196-197`).
3. The oracle computes the ground point G under (px, py) from `clip_from_world` and
   `viewport_logical_rect` of the `turts_scene` bridge
   (`crates/turts_client_web/src/lib.rs:471-479`), with the plane y = 0 of
   `crates/turts_client/src/input.rs:14-21`. The bridge coordinates are window logical pixels
   (`lib.rs:473`). `io.screenshot()` calls `page.screenshot({ scale: "css" })`
   (playwright-core 1.64.1, `tests/browser/node_modules/playwright-core/types/types.d.ts:13702`),
   so a screenshot pixel, a mouse coordinate, and a bridge coordinate are the same unit. The client
   needs no change.
4. In the server tick log, the unit at rest stands within 0.011 world units of G. `scripts/t7.sh:52`
   writes the log to `$OUT/server-tick.log`. 0.011 is the sum of two existing tolerances:
   `SCENE_TOLERANCE` 0.01 for the unprojection (`tests/browser/t7.js:45`) and `POS_TOLERANCE`
   0.001 for the arrival (`t7.js:44`).

Red build: comment out the `MoveTo` submit at `crates/turts_client/src/input.rs:134-140`. Conditions
2 and 4 fail. The run goes twice to find flakiness. A flaky driver stays out of CI and does not count
as a pass.

## 5. `turts-pack` validation defects

### 5.1 Unknown capability

Status: this refines S:231 (Baseline, "requested capabilities"). It is a refinement, not a change
of a Decided item.

Design: `turts_mod_api` gets `KNOWN_CAPABILITIES: &[&str]`, empty at QA0. `validate_manifest`
(`manifest.rs:9`) refuses a capability that is not in the list, and a duplicate. Field path:
`capabilities`. Message: "unknown capability `scripts.server`; known: none". An engine feature that
defines a capability adds a row (S:304, "apply capability limits"). All packages in `mods/` and in
`crates/turts_pack/tests/fixtures/` use `capabilities: []` (`QP/gate-rigour.md` run 12), so the
rule breaks no shipped package.

Files: `crates/turts_mod_api/src/lib.rs` (or a new `capability.rs`),
`crates/turts_mod_runtime/src/manifest.rs`, `crates/turts_pack/src/main.rs:22-28` (the help list),
`crates/turts_pack/tests/bad_packages.rs` (a new case with `assert_refused`).

Red today: exit 0 (`QP/runs-pack.txt` p1). No code reads the field (section 2).

### 5.2 `kind: Library` (K3)

Design under K3:

1. `validate_manifest` reads `kind`. A Game exports exactly one map (the current rule,
   `manifest.rs:51-59`). A Library exports zero maps. A map in a Library is an error on the field
   `exports.maps`.
2. `turts_pack::check_package` (`crates/turts_pack/src/lib.rs:41-46`) has two paths.
    - Game path: `load_package`, as now. `load_package` calls `resolve` (section 7).
    - Library path: `archive.content()` (`archive.rs:91-135`) and `visuals_by_content`. No world.
      **No `resolve`.** At QA0 the Library path parses the `dependencies` field, checks LR1 and LR2
      (in `validate_manifest`), and keeps the field in the manifest. It does not resolve the
      dependencies of a library. MP2 resolves them, when a server serves libraries (K5).
3. `load_package` (`lib.rs:13`) runs its steps in this order:
    1. `resolve(root, [])` (section 7). Its first rule is LR7, the kind check: a Library root is
       refused with the text "a library cannot be the root of a match". Then LR1 to LR6 and LR8.
    2. `archive.content()`, which runs the version table (section 6.1).
    3. The map count and the boot, as now (`lib.rs:15-37`).
   The kind check comes before `content()`, so the version table cannot hide it. There is one
   kind check, not two. This protects the server boot (`crates/turts_server/src/boot.rs:37`).
4. The client needs no new rule at QA0. A Library root fails on the client with "the packages have
   no map" (`acquire.rs:103`). That refusal is an accident of the map rule. A later relaxation of K3
   (map libraries) must change `acquire.rs:83-84,98-100` too, because the client takes the first
   map that it finds.

Tests, in `bad_packages.rs` or a new `crates/turts_pack/tests/library.rs`. Each test package is a
copy of `mods/acme` with the edits in the first column. Each declares `engine_api: "^0.2.2"`
unless the row says otherwise, so the version table (section 6.1) accepts `kind: Library` and
`dependencies`.

| Case | Expected | Today |
| --- | --- | --- |
| Library with units and a map | exit 2, field `exports.maps` | with `^0.2`: exit 0 (`QP/runs-pack.txt` p2) |
| Library with units and zero maps | exit 0 on the library path | exit 2 on `engine_api`, because engine 0.2.1 refuses `^0.2.2` at `manifest.rs:34` (`QP/repair2-pack.txt` p7); with `^0.2`: exit 2 at `manifest.rs:55` (p6) |
| Library with units, zero maps, and `dependencies: {"ai_core": "^1.0"}` | exit 0 on the library path: no `resolve` | exit 2: `manifest.rs:34`, or `manifest.rs:44-49` with `^0.2` (p3) |
| Library as root: `load_package` on a Library archive with zero maps | the error text is exactly "a library cannot be the root of a match" | the archive fails in `PackageArchive::parse` at `manifest.rs:34` or `manifest.rs:55`; `load_package` is not reached |

Entry of the fourth test. `PackageArchive.files` is private (`archive.rs:28`), and `parse` is the
only constructor (`archive.rs:35`). So the test packs the directory, calls `PackageArchive::parse`,
then calls `load_package` on the result. After QA0, `parse` accepts the archive: zero maps is the
Library rule, and the version table is not in `parse`. Then `load_package` refuses it at step 1
(LR7). The test asserts the exact text. So the test turns red when LR7 is removed: the package has
no dependencies, so `resolve` passes; `content()` accepts `^0.2.2`; and step 3 gives the map-count
error ("a package exports exactly one map, found 0", `lib.rs:16-23`).

## 6. Schema open for additive growth (K1)

Problem (R:42, F7): every schema type is closed. A modder's field fails as "unknown field". Each
milestone that adds a field edits the struct, bumps the API, and writes a version gate by hand
(`content_m3_validation.rs:511-520`). Requirement: a later milestone adds a field without a new
hand-written gate, and a typo still fails.

Rule for all options: a field is valid only when the engine implements it. A field that the engine
accepts and ignores is a silent defect. QA0 adds no modder field.

| Option | Mechanism | Typo fails? | Edits for each new field | Cost |
| --- | --- | --- | --- | --- |
| O1 (recommended) | Keep `deny_unknown_fields`. A new field gets `serde(default)` and a patch bump (the M3 K4 pattern: 0.2.0 to 0.2.1, `docs/superpowers/specs/2026-10-27-m3-industrial-design.md:30`). Add the version table and the nearest-name hint. | yes | 2: the struct field, one table row | About 1 plan task. |
| O2 | `UnitDef` and `Exports` get a hand-written `Deserialize`. Fixed keys stay. A table dispatches every other key. | yes | 4: the typed struct field, a match arm, a table row, an entry in a second name list (`QP/gate-modrt.md` H1) | About 2 plan tasks and about 120 lines for each type. |
| O3 | A free `ext: { "vendor.key": value }` block. | no, unless the keys are registered, and then it is O2 | n/a | Later: library-to-mod data needs it. That is library format. |
| O4 | Drop `deny_unknown_fields` and print a warning. | no | 1 | Rejected: a typo is silently ignored. |
| O5 | A `components: { ... }` map syntax. | yes | 2 | High: rewrites all five `mods/` packages, the test packages, and `turts_checks/examples/m2_variants.rs`. Same power as O2. |

A minor bump (0.3.0) is not an option for additive fields: `^0.2` and `^0.2.1` refuse engine 0.3.0
at `manifest.rs:34` (`QP/repair-modrt.txt`, lines that start with `V`).

### 6.1 Version table

Each table row has a name, a presence test (for O1: the `Option` field is `Some`, or a manifest
value is present), and `since_api`.

Definition. A requirement **matches a version below `since_api`** when one version v in the
released list `RELEASED_ENGINE_APIS` has v < `since_api` and `VersionReq::matches(v)` is true. The
list is the engine versions that the project released: 0.1.0, 0.2.0, 0.2.1 (section 2), and 0.2.2
from QA0 on. A finite list avoids a semver query that does not exist ("admits any version below
X"). The M3 gate tests one version, 0.2.0 (`content_m3_validation.rs:511`). The table generalizes
that test.

Check. The table refuses a present field when the manifest requirement `engine_api` matches a
version below the `since_api` of its row. The message names the field, `since_api`, and the
admitted version. Location: `PackageArchive::content()`, after `validate_m3` (`archive.rs:132-133`).
The server and the client both run `content()` (section 2), so the two paths agree.

Real rows at QA0: non-empty `dependencies` (since 0.2.2) and `kind: Library` (since 0.2.2).
A package that uses either must declare a requirement that excludes 0.2.1, for example `^0.2.2`.
No shipped package uses either (`QP/gate-modrt.md`, grep). The M3 gate stays as it is. To move it
into the table is not in QA0.

### 6.2 Tests

| ID | Check | Red build |
| --- | --- | --- |
| SC1 | A unit field `helth`: exit 2, the allowed names, and "did you mean `health`". | Today: no hint (`parse.rs:20-23`). |
| SC2 | A duplicate key in a unit file is refused. | Today green: the ron derive refuses it (`QP/rig-pack2.txt` `dup`; `QP/repair-modrt.txt:6`). Under O1 the derive stays, so SC2 is a regression guard. Red build: the naive O2 dispatch (`QP/repair-modrt.txt:16`, accepted). |
| SC3 | A unit test calls the table check with an injected table that holds a test row with `since_api` 0.2.1. `^0.2` is refused. `^0.2.1` is accepted. | Remove the `since_api` comparison. Model: `QP/repair-modrt.txt:26-27`. |
| SC4 | `turts-pack` on a package with `kind: Library`, zero maps, and `engine_api: "^0.2"` exits 2 on field `engine_api`: the library path runs `content()`, and the table refuses the row. With `^0.2.2` it exits 0. | Remove the row. Today that package fails on `exports.maps` (p6), not on `engine_api`. |

Under O2, one more test is necessary: a golden test. Every unit file in `mods/` and in the test
packages must parse to the same values before and after the change. The mod-runtime probe shows that this test is
reachable (`QP/runs-modrt-o2.txt:1`).

## 7. Library dependency check

Design: `turts_mod_runtime::resolve::resolve(root, available) -> Result<Vec<PackageId>, ModError>`.
A pure function with a deterministic order: dependencies first, ties by id.

| Rule | Red when |
| --- | --- |
| LR1 | A dependency key is not a `PackageId`, or its value is not a semver requirement. |
| LR2 | A package depends on itself. |
| LR3 | A dependency is not provided, or two versions of it are provided (D35, S:64). The message names the id and the requirement. |
| LR4 | The provided version does not match the requirement. |
| LR5 | A dependency is a Game. Only a Library can be a dependency. |
| LR6 | The graph has a cycle. The message lists the cycle. |
| LR7 | The root is a Library. Text: "a library cannot be the root of a match". `resolve` checks LR7 first. |
| LR8 | A provided package is not reachable from the root. This is an error: the input holds an unused package. |

Call sites:

- `validate_manifest` removes `manifest.rs:44-49` and checks only LR1 and LR2.
- `load_package` calls `resolve(root, [])` as its first step (section 5.2 item 3). This is the one
  production call site. The server boot calls `load_package` (`boot.rs:37`), and the Game path of
  `turts-pack` calls it (`crates/turts_pack/src/lib.rs:43`). Under K5 the server has no libraries,
  so a root with dependencies fails with LR3 in both. So `turts-pack` and the server boot refuse the
  same packages. This keeps the invariant in `boot.rs:4-6` ("a package that it packs is a package
  that boots") and `crates/turts_pack/src/main.rs:22-23`.
- The Library path of `turts-pack` does not call `resolve` (section 5.2 item 2).
- The client does not call `resolve`. S:302 says that the client "never resolves versions itself"
  and "does not download server-only packages". The client drops `Server` entries before it fetches
  (`acquire.rs:60-65`), and the lockfile entry has no dependency graph (`lock.rs:15-24`). A client
  `resolve` over the fetched set fails with LR3 as soon as a game depends on a server-only library.
  The client checks the lockfile instead, in `check_lockfile` (`acquire.rs:52-56`):
    - each id has one entry (D35, S:64);
    - the advertised `mod_id` is the id of one entry.

  The second check needs the advertised `mod_id`, so the signature of `check_lockfile` changes
  (today `check_lockfile(bytes, advertised_hash)`, `acquire.rs:52`). The change touches these call
  sites, and the plan task P4 lists them:
    - `crates/turts_client/src/join.rs:168`. The advertisement is in `acquisition.advertisement`
      at that point (`join.rs:155`).
    - `crates/turts_client/tests/acquire.rs:80`, `:85`, `:94`.
    - `crates/turts_client/tests/t2d_client_hash.rs:67`, `:81`.

What covers the dependency check without `--lib`:

- `crates/turts_mod_runtime/tests/resolve.rs`: one test for each rule LR1 to LR8, on manifests in
  memory.
- `crates/turts_pack/tests/`: a process test. `turts-pack` on a game with `{"ai_core": "^1.0"}` and
  `engine_api: "^0.2.2"` exits 2 with LR3.
- `crates/turts_server/tests/`: a boot test. A root with a dependency and `^0.2.2` fails with LR3.
- `t2c_package_rejects.rs:71-79` changes: the parse accepts `{"lib": "^1"}`, and `resolve` refuses
  it with LR3. Today that test fails at the new assertion, because the message is "not supported
  at M1" (`manifest.rs:47`).
- `crates/turts_client/` unit tests: `check_lockfile` refuses a lockfile with two `acme` entries,
  and a lockfile without the advertised `mod_id`. Today `check_lockfile` accepts both: it checks
  only the hash and the parse (`acquire.rs:52-56`).

The resolver rules LR2 to LR8 have no meaningful red today, because the function does not exist.
A compile error is not a red. The trust comes from the mutation runs in the plan: remove each rule
and record the red.

For MP2: `HostConfig` has no `deny_unknown_fields` (`config.rs:65-75`). Add it before a
`libraries:` key exists, or a typo in that key is silently ignored.

## 8. Versions

- `ENGINE_API_VERSION` goes from 0.2.1 to 0.2.2 (`crates/turts_mod_api/src/lib.rs:33`). This is a
  patch bump, additive. QA0 removes no field and renames no field. It adds no manifest field. It
  makes two manifest values valid that 0.2.1 refused: non-empty `dependencies` and a Library with
  zero maps. The version table gates both (section 6.1). Every 0.2.1 package loads unchanged:
  `^0.2` and `^0.2.1` match 0.2.2.
- The client compares the advertised engine version for equality (`acquire.rs:41`), so a 0.2.1
  client refuses a 0.2.2 server. The browser client and the server build from one commit, so this
  is the expected effect.
- One test literal changes: `t2c_package_rejects.rs:57` asserts "0.2.1".
- The lockfile format does not change: `LockEntry` keeps its seven fields (`lock.rs:15-24`), and
  the server writes one entry (`boot.rs:43-46`). The hash rule does not change: the hash covers the
  exact archive bytes. The hash of an existing package set does not change.
- The wire protocol does not change.

## 9. CI wiring

| Job | New step | Command | Time |
| --- | --- | --- | --- |
| `agents` (`ci.yml:138`) | Yardstick list in sync | `python3 scripts/qa/yardstick.py --check` | under 1 s |
| `agents` | Completeness gate | `python3 scripts/qa/completeness.py --self-test && python3 scripts/qa/completeness.py --enforce` | under 1 s. The coverage probe and six bridge probe runs took 0.14 s together. |
| `agents` | Pixels-only driver rule | `python3 scripts/qa/driver_rule.py --self-test && python3 scripts/qa/driver_rule.py tests/browser` | under 1 s |
| `check` (`ci.yml:32`) | no new step: the new Rust tests run in `cargo test --workspace` | — | NOT TRACED. Estimate: 10 to 30 s. |
| `browser` (`ci.yml:109-114`) | the T7 step runs the split driver and oracle (K2 option C) | `bash scripts/t7.sh` | NOT TRACED. Estimate: today plus 10 s. |

The `agents` job builds no Rust, so a red gate shows on a push in less than one minute. The job has
no `actions/setup-python`. The runner Python is sufficient for the standard library (NOT TRACED:
the runner Python version).

## 10. Cut line

Two tiers. Tier **must** holds what the register and S:721 require, and what MC cannot start
without. Tier **can slip to MC** holds the rest. For each slip item the table says what replaces it.

| Item | Tier | If it slips, this replaces it |
| --- | --- | --- |
| Completeness gate, registry, QA0 fixture, mutations, self-test, CI step | must | — |
| Driver rule with roles, oracle rule, ledger, samples, CI step (K2 option A for both tests) | must | — |
| Unknown capability; Library rule | must | — |
| Resolver, the lift, the server and `turts-pack` call sites, the client lockfile check, the 0.2.2 bump | must | — |
| Version table and typo hint (O1) | must | — |
| T7 split, harness `io` and `view` (K2 option C) | can slip | K2 option A: `t7.js` is a ledger row. MC writes the first real driver. A6 and DR6 keep `t7.js` visible as debt. |
| O2 (only if the CEO picks it in K1) | can slip | O1. SC1 to SC4 stay. |
| The M3 version gate moved into the table | can slip | The M3 gate stays as it is (`content_m3_validation.rs:511-520`). |
| A process test of the T7 driver on two runs in CI | can slip | The T7 step as it is today. |

## 11. Acceptance

Each item names its red build. "Today" means commit `f56125d`. Tier: M (must) or S (can slip).

1. **A1 (M) Yardstick sync.** `yardstick.py --check` exits 0. The TSV has 181 CORE rows. Red build:
   one TSV row changed (mutation run). Today the file does not exist.
2. **A2 (M) Gate red on M1 to M4 as they stand.** Each gives exit 1 with the rule set {R1, R3}.
   Red build: exit 0 on any of them. Today the coverage probe gives {R1, R3} on all four
   (`QP/repair-c1.txt`). Cause: no coverage heading and no player-path heading in the four files.
3. **A3 (M) Gate green on the QA0 fixture** `tests/qa/completeness/green.md`. Red build: a parser
   that refuses a valid row. Today the probe is green on `fx_green.md`.
4. **A4 (M) Self-test.** All 20 cases match their sets. Red builds: a mutation that is green; a
   mutation with a subset or a superset of its rules; an empty cases file; a cases file without the
   M4 row (`QP/repair-selftest.txt`); a cases file without an R9 case.
5. **A5 (M) Enforce and registry.** `--enforce` exits 0 and prints count 1. Red builds: the
   `dirs/r8` case (R8); the `dirs/r9` case, an MC spec marked `client: no` (R9); a new
   `client: yes` spec without a coverage table.
6. **A6 (M) Driver rule on the tree.** Exit 0 with the roles of section 4.2. Red build: `t7.js` or
   `m3-human.js` with role `driver`. Today these lines are red: `t7.js:213` and
   `m3-human.js:7,27,35,43`.
7. **A7 (M) Driver samples.** The green samples are green: `ok-driver.js`, `ok-oracle.js`, a driver
   that saves through `io.save` and requires a `pure` PNG file on `node:zlib`, and the `pure` files
   of section 4.4. Each red sample gives its rule set: DR1 to DR7, and each Part A item A1 to A7,
   with the samples `global["pro" + "cess"]`, the tick-log reader, and `module.parent`. Model:
   `QP/repair3-driver.txt`.
8. **A8 (S) T7 split.** `t7-driver.js` passes DR1 to DR3. `t7-oracle.js` passes DR5. T7 is green
   on two runs in a row with the four conditions of section 4.6. Red build: `input.rs:134-140`
   commented out. If A8 slips, A6 with `t7.js` in the ledger replaces it.
9. **A9 (M) Unknown capability.** Exit 2, field `capabilities`. Today exit 0 (p1).
10. **A10 (M) Library with a map.** Exit 2, field `exports.maps`. Today exit 0 (p2).
11. **A11 (M) Library with zero maps.** The package declares `^0.2.2`. Exit 0 on the library path,
    with and without a non-empty `dependencies` field (no `resolve` on that path). Today exit 2 on
    `engine_api` at `manifest.rs:34` (`QP/repair2-pack.txt` p7); with `^0.2`, exit 2 at
    `manifest.rs:55` (p6).
12. **A12 (M) Library as root.** The test packs a Library with zero maps and `^0.2.2`, calls
    `PackageArchive::parse`, then calls `load_package`. The error text is exactly "a library cannot
    be the root of a match" (LR7, step 1 of section 5.2 item 3). Today the archive fails in `parse`
    at `manifest.rs:34`. Red build: LR7 removed; then step 3 gives "a package exports exactly one
    map, found 0" (`crates/turts_pack/src/lib.rs:16-23`).
13. **A13 (M) Manifest accepts dependencies.** The changed `t2c_package_rejects.rs:71-79` test: the
    parse accepts `{"lib": "^1"}` and `resolve` refuses it with LR3. Today red: "not supported at
    M1" (`manifest.rs:47`).
14. **A14 (M) Resolver rules.** One test for each rule LR1 to LR8. Each turns red when the plan
    removes its rule (mutation run).
15. **A15 (M) Server and `turts-pack` call sites.** The package is a Game with
    `dependencies: {"ai_core": "^1.0"}` and `^0.2.2`. The server boot fails with LR3. `turts-pack`
    exits 2 with LR3. `resolve` runs before `content()` in `load_package`, so LR3 is the first
    error. Red build: the refusal lifted and the `resolve` step missing in `load_package`. On that
    build `load_package` boots the root, because it reads no dependency
    (`crates/turts_pack/src/lib.rs:13-38`).
16. **A16 (M) Client lockfile check.** `check_lockfile` refuses two entries with one id, and a
    lockfile without the advertised `mod_id`. Today it accepts both (`acquire.rs:52-56`). The
    signature change updates `join.rs:168`, `tests/acquire.rs:80,85,94`, and
    `tests/t2d_client_hash.rs:67,81` (section 7).
17. **A17 (M) Version 0.2.2.** `ENGINE_API_VERSION` is 0.2.2. All packages in `mods/` still boot.
    Today 0.2.1 (`crates/turts_mod_api/src/lib.rs:33`).
18. **A18 (M) Typo hint (SC1).** Today: no hint (`parse.rs:20-23`).
19. **A19 (M) Duplicate key (SC2).** A regression guard. Red build: the naive O2 dispatch
    (`QP/repair-modrt.txt:16`).
20. **A20 (M) Version table (SC3, SC4).** Red builds: the `since_api` comparison removed; the
    `kind: Library` row removed.
21. **A21 (M) CI receipt.** The three new `agents` steps run on the QA0 merge commit and are green.
    Before the merge, each step was red locally on a mutation.

## 12. Risks

1. **The coverage table becomes a form.** A row can say `in` with a test id that proves nothing.
   The gate tests that the mapping is complete and ordered. It does not test that a test is good.
   Mitigation: R2 binds the id to the acceptance table. The gate reviewers still ask "which build
   turns it red". A person runs the player check (S:703, G12).
2. **The yardstick is a research document.** An item can be wrong. K6 controls changes.
3. **A static check can be evaded.** Part A closes the 20 evasions of the model run, the samples
   `global["pro" + "cess"]`, the tick-log reader and `module.parent` included
   (`QP/repair3-driver.txt`). The driver also has no
   `page` (section 4.3). Residual: a program can build a key in a variable first. Then it can use
   the key on an object that is not in the A4 list, for example
   `const k = "con" + "structor"; io[k][k]`. Review catches it, not the check. A stronger fix is
   a separate Node process for the driver, with the Node permission model. NOT TRACED: that model on Node 20.
4. **O2 loses serde guarantees.** The naive O2 build accepted a duplicate key. K1 recommends O1,
   which keeps the derive. SC2 guards both options.
5. **The lift opens the server boot.** Mitigation: A15. The client gets a lockfile check, not a
   resolver (section 7).
6. **The T7 driver is flaky on Xvfb with Vulkan.** Mitigation: two runs. A flaky driver falls back
   to option A (section 10).
7. **The bridges stay in every browser build** (`crates/turts_client_web/src/lib.rs:78-79`). The
   driver cannot reach them. Their removal from player builds is MC work.

## 13. Budget

Plan tasks, with the names P1 to P8 (P is a plan task; T7 is only the browser join test):

| Task | Content | Tier |
| --- | --- | --- |
| P1 | Yardstick TSV, milestone order, registry, completeness tool, QA0 fixture, mutations, self-test, CI step | must |
| P2 | Driver rule, roles, oracle rule, ledger (option A), samples, self-test, CI step | must |
| P3 | Unknown capability and the Library rule, with the library path in `turts-pack` | must |
| P4 | Resolver, the lift, the server and `turts-pack` call sites, the client lockfile check, the 0.2.2 bump | must |
| P5 | Version table (O1), the two real rows, the typo hint | must |
| P6 | Verify: mutation runs for every rule, `CLAUDE.md` commands, `turts-pack --help` | must |
| P7 | Harness `io` and `view` | can slip |
| P8 | T7 split into `t7-driver.js` and `t7-oracle.js`, two runs | can slip |

The conversion uses the M3 rate: 3.75 agent runs for each plan entry, gates included.

- Tier **must**: 6 tasks, 22.5 runs. With one repair entry: 7 tasks, 26 runs. **22 to 26 runs.**
- All, with the recommended calls (O1, option C): 8 tasks, 30 runs. With O2 instead of O1: one
  more task. With one repair entry: one more task. **30 to 38 runs.**

## 14. Gate 1 findings and their fixes

| Finding | Fix |
| --- | --- |
| Rigour B1 (A12) | Section 5.2 third test; A12. Orchestrator decision 1. |
| Rigour H1 (one column, two facts) | `specs.tsv` with `client` and `gate`; section 3.4. Decision 7. |
| Rigour H2 (self-test green on empty) | Set equality and conditions 1 to 3, section 3.4; model run. Decision 8, read as "every exempt spec". |
| Rigour H3 (two roles, two meanings of oracle) | Section 4.1 terms; one role for each file; DR6. Decision 9. |
| Rigour H4, mod-runtime H2 (S4 unreachable) | Section 6.1 definition and location; SC3 at 0.2.1. Decision 4. |
| Rigour H5 (eval evasion) | DR2 and DR3; model run. Decision 10. |
| Rigour M1, decision 11 (budget) | Section 13; two tiers in section 10. |
| Rigour M2 (names) | P1 to P8, SC1 to SC4, DR1 to DR7 (revision 2), A1 to A6, LR1 to LR8, "coverage probe", "bridge probe", "driver sample". |
| Rigour M3 (receipt) | `QP/repair-c1.txt`; section 2. |
| Rigour M4 (cut items) | Section 10 "replaces" column; A8 tolerances from `t7.js:44-45`. |
| Rigour L1 (LR8) | LR8 is an error. |
| Rigour L2 (STE) | Long sentences split; passive "is refused" replaced by an actor; the M4 status edit names the orchestrator. |
| Rigour L3 (capability status) | Section 5.1 states the Baseline refinement. |
| Mod-runtime B1 (client resolve) | Section 7: no client `resolve`; lockfile check. Decision 2. |
| Mod-runtime H1 (O2 cost) | K1 recommends O1. Decision 3. |
| Mod-runtime H3 (`--lib`) | `--lib` cut; section 7 lists what covers the check. Decision 5. |
| Mod-runtime N1 to N4 | "patch bump" (section 6); section 8; section 5.2 item 4; section 7 MP2 note. Decision 6. |
| Re-check: mod-runtime R1 (Library path cannot exit 0) | Section 5.2 item 2: no `resolve` on the Library path; `dependencies` parsed and kept. `resolve` lives in `load_package` only. SC4, A11, and the 5.2 test rows can exit 0. Second-pass decision 1. |
| Re-check: mod-runtime R2 (`check_lockfile` callers) | Section 7 lists `join.rs:168` and the five test call sites; A16. Decision 2. |
| Re-check: rigour N1 (`pure` files red) | Section 4.4 Part A and Part B; `node:zlib` allowed; `node:fs` allowed in `pure` files only (revision 3, A7); model run green on both `pure` files. Decision 3. |
| Re-check: rigour N2 (A12 mechanism) | Section 5.2: test packages declare `^0.2.2`; LR7 is step 1 of `load_package`, before `content()`; the A12 test enters through `parse` then `load_package`. Decision 4. |
| Re-check: rigour N3 (`global` evasion) | Part A items A3 and A4; model run red. Decision 3. |
| Re-check: rigour N4 (`client` set by authors) | Rule R9 with the literal lists `NO_CLIENT` and `NOT_MILESTONE`. Decision 5. |
| Re-check: rigour N5 (R8 case) | `cases.tsv` column `mode`, values `spec` and `enforce-dir`; `dirs/` cases. Decision 6. |
| Final re-check: rigour F-H1 (`node:fs` in a driver) | Part A item A7, applied by DR2 only. |
| Final re-check: rigour F-M1 (exempt set by `milestone`) | Exempt set and `NO_CLIENT` named by file path (sections 3.3, 3.4). |
| Final re-check: rigour F-M2 (`module.parent`) | A4: only `module.exports` is allowed. |
| Final re-check: rigour F-L1 (stale lines) | Section 3.1 tool row (R5 to R9, `--registry`); section 3.4 model text (R1 to R9). |
| Re-check: rigour NOT TRACED (oracle source, pixel scale) | Section 4.6 conditions 2 to 4: `turts_debug` `command_results`, the tick log, `scale: "css"`. |

## 15. What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `turts-pack` on six copies of `mods/acme` (`QP/runs-pack.txt`) | R:47-48: unknown capability and a Library with a map exit 0; `manifest.rs:44-49` refuses dependencies | p1 or p2 exit 2; p3 exit 0 | p1 0, p2 0, p3 2, p4 2, p5 2, p6 2 |
| Coverage probe on M1 to M4, `fx_green.md`, 5 mutations, without a pipe (`QP/repair-c1.txt`) | S:711, A2 to A4: red on M1 to M4, green on the fixture; the rule sets of section 3.5 | a spec green, the fixture red, another rule set | M1 to M4 exit 1 {R1, R3}; fixture exit 0; mutations exit 1 with the sets of 3.5 |
| Self-test model on four case files (`QP/repair-selftest.txt`) | Section 3.4: empty, subset and incomplete case files go red | any of the three green | all three red; the probe cases green with the rules cut to R1 to R4 |
| Driver rule model, revision 1 (`QP/repair-driver.txt`) | revision 1 DR2 to DR4 | an evasion green | 10 red, `ok-driver.js` green (superseded) |
| Driver rule model, revision 3 (`QP/repair3-driver.txt`, `dr_model3.py`) | Section 4.4 with A7 and the `module` members | a driver sample green; a `pure` file or green driver red | 20 of 20 driver samples red; 2 of 2 green drivers and 5 of 5 `pure` files green; `var-key.js` green (accepted residual); `t7.js`, `m3-human.js` red |
| Driver rule model, revision 2 (`QP/repair2-driver.txt`, `dr_model2.py`, superseded) | Section 4.4 Part A and B: evasions red; `pure` and `node:fs`/`node:zlib` files green | an evasion green, or a set-2 file red | 14 of 14 evasions red; `require("node:fs")` green by Part B; 7 of 7 set-2 files green; `t7.js`, `m3-human.js` red |
| `turts-pack` on p7: Library, zero maps, `^0.2.2` (`QP/repair2-pack.txt`) | A11 and section 5.2 "Today" cells | exit 0 | exit 2, `engine_api`, engine 0.2.1 |
| `rtk proxy grep "check_lockfile("` | section 7 call-site list | another site | 6 call sites as listed |
| read `debug.rs:170-200`, `t7.sh:52`, `types.d.ts:13702` | section 4.6 conditions 2 to 4 | no applied field; no tick log; no `scale` option | all present |
| Mod-runtime probe re-run, compared with `cmp` (`QP/repair-modrt.txt`) | K1 evidence; SC2; SC3 at 0.2.1 | another output | identical to `runs-modrt-o2.txt` |
| Bridge probe on all 12 browser files (`QP/runs-c2.txt`) | R:118: red on `m3-human.js` and `t7.js` | either green | red 4 and red 4; 7 of 12 red |
| `git log -G` on `ENGINE_API_VERSION` | Section 6.1 released list | another value | 0.1.0, 0.2.0, 0.2.1 |
| `rtk proxy grep` for literal "0.2.1" in crates, tests, scripts, mods | Section 8: the bump touches one test literal | more test literals | `t2c_package_rejects.rs:57` and `lib.rs:33` only (two `Cargo.lock` files hold an unrelated 0.2.1) |
| `rtk proxy grep` S:302 quotes; read `acquire.rs:28-105` | Section 7 client design | no S:302 text; a client graph | both quotes at S:302; no graph |
| `rtk proxy grep -c` headings in M1 to M4 specs | A2 cause | count above 0 | 0 in each |
| `awk` tier count in Y Part A | 181 CORE | another total | 177 + 4 with a note = 181 |
| `rtk proxy grep` for `capabilities`, `PackageKind`, `PackageArchive::parse`, `m3-human` in `scripts/` and `.github/` | no production reader of the two fields; parse sites; `m3-human.js` not in CI | a production reader; a CI hit | as stated in section 2 |
| `time` on both probes | CI time under 1 s | over 1 s | 0.14 s |

**NOT TRACED:** run times of the new Rust tests and of the T7 split on GitHub runners; the runner
Python version; whether a colour search finds the T7 unit reliably under Xvfb with Vulkan; the PNG
decoder choice; the Node permission model on Node 20; the effect of O2 on `Exports` (the
mod-runtime probe covers `UnitDef` only); the CI time of the client lockfile tests; whether D48 AI
libraries will have the `Server` context (S does not say; section 7 holds for any server-only
dependency).
