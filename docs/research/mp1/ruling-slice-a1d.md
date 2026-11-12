RULING=(ii)
VERDICT=fail FINDINGS=4 HIGH=0 RUNS=10 NOT_CHECKED=4

# Ruling on pre-MP1 slice task A1d (technical-director, 2026-11-12)

Mode: answer (a ruling). Input: the review `docs/research/mp1/review-slice-a1d.md` (qa-lead, FAIL, 1 High).
Candidate: `a921096b` on `pre-mp1/a1d-oracle` (base `afb9c6de`), lane `/home/julien/Projects/TuRTS-pre-a1d-review`.
Status of the item: D `:73-74` is gated design text (D revision 6, section 3). It comes from my own input
(TDI `:40`). It is NOT a CEO decision: the CEO decision of 2026-11-11 (`vault/Decisions/2026-11-11 MC
architecture audit.md:47`) covers the economy log validator (ledger only, in `turts_audit`), and it does not
name `conservation.rs` (grep: 0 hits in that file). So this ruling amends the design; it needs no CEO.

## 1. The ruling: (ii)

Revert the move. `crates/turts_pack/tests/common/conservation.rs` stays in the test tree as the independent
source oracle. `turts_audit` holds one rule set: the ledger audit of a log. A1d closes with the A1c hand-over
items. D `:73-74` changes (text in section 3).

Reasons:
1. The C6 probe holds. I ran it (section 6, runs R1 and R2). With plant C6 the ledger audit is `Ok(())` and the
   source oracle is red with `REFUND`. On the clean build the probe is red at its own guard (`zz_a1drev_c6.rs:149`).
   So the probe can fail, and it fails on the right build.
2. Cause in code: the ledger takes the refund ratio from the logged catalog (`crates/turts_audit/src/production.rs:37-43`,
   `initial.catalog`), and the logged catalog comes from `LedgerAudit.catalog` (`crates/turts_sim/src/economy/ledger_view.rs:124`, `:157`),
   which the code under test fills. The source oracle reads the authored RON (`crates/turts_audit/src/source.rs:177-178`).
   The M3 registry already depends on this: row C6 says "the independent refund oracle reads that authored
   ratio" (`tests/m3/refutations.json:158`).
3. The debt that D `:73-74` names is smaller than written. The source oracle is not a second engine
   implementation of the ledger. It is fixture-bound: it tracks one node item and one ammo item
   (`source.rs:408-410`) of the forge package. It is test code. A rule change that must be stated in the code
   and in an independent oracle is the cost of an oracle, not architecture debt.
4. (i) puts a forge-shaped test oracle and a second package reader (`ron`, `source.rs:177-178`) into an engine
   crate that `turts_server` links (`crates/turts_server/Cargo.toml:10`). D `:63-64` does not list `ron`. That is
   new architecture debt (review M2). Rejected.
5. (iii) costs about 1,100 lines and 3 or more runs, and it makes one defect blind both oracles (common mode).
   It lowers quality. Rejected.
6. (ii) costs about 2 runs. It removes M2. It keeps the 17 M3 texts byte-equal with no work (the file returns
   unchanged). It fits "speed with no loss of quality, no deep architectural debt".

## 2. Rulings on the other items

**M1 (a server system that changes a run constant is seen by no default test).** Fix in the A1d repair pass,
because the A1c hand-over gives this item to A1d (plan Binding amendments, "A1d: read
`docs/research/mp1/a1c-handover.md`"). Fix shape: a writer scan in `turts_checks`, NOT a digest in the log.
- Reasons: the scan is a default test; it covers `turts_sim/src`, `turts_server/src` and `turts_pack/src`
  (the roots at `crates/turts_checks/tests/movement_writers.rs:270-280`). A digest in `LedgerAudit` changes the
  log format, and it is red only where a server log is validated, which is the `#[ignore]` process tests
  (`crates/turts_server/tests/economy.rs:73-75`). So a digest is still no default test.
- The digest stays an option for the M11 replay format (status: Proposed; owner `backend-engineer`).
- Owner: `backend-engineer` builds (A1d seat). `qa-lead` reviews the instrument.
- A gap that the scan must close first: the movement scan has no arm for `ResMut<T>`, `Mut<T>` or
  `NonSendMut<T>` (`movement_writers.rs:186-253`: only `&mut T`, turbofish methods and constructors). The
  SRV-B plant uses `get_resource_mut::<turts_sim::MapBounds>()` (turbofish, seen), but a system parameter
  `ResMut<MapBounds>` is not seen. `MovementPools` is a `Resource` (`crates/turts_sim/src/movement/mod.rs:102`),
  so the movement scan has the same gap today. A grep of production code finds no `ResMut<` of a scanned type
  now (run R6), so the arm adds no new allowed entry.

**Scan hole for compound names (`ore_items`, `shell_items`).** Record it; do not repair it now.
- Under (ii) the two names leave `turts_audit` (`source.rs:135`, `:159` go back to the test tree), so no
  engine instance remains.
- A split-on-underscore rule is not a small change: my rough probe (run R5) finds 49 compound identifiers
  (193 uses, 29 files) that contain a content token. The top one is `client_shell` (46 uses), a spec term
  (D72), not content. A repair needs an exception design.
- Record it as a "Known gap" line in the header of `crates/turts_checks/tests/no_content.rs`, and as a board
  item. Owner `qa-lead`. Deadline: before the first MP1 package with new content ids merges.

**What stays from a921096b.**
- Stays: `assert_run_constants` and its two call sites (`crates/turts_server/tests/economy.rs:138-156`,
  `:704`, `:979`), the doc comment `:59-75`, the removal of the `bevy*` dependencies of `turts_audit` (A1c
  hand-over), the three `A1d-run-constants-*` rows with their plants moved to real state (L1), and the
  test `test_a1d_run_constants_rows_name_their_instrument` in `tests/m2/test_refutations.py` (updated to the new
  plant paths).
- Goes: `crates/turts_audit/src/source.rs`, `pub mod source` and the `source` paragraph in
  `crates/turts_audit/src/lib.rs`, `ron` in `crates/turts_audit/Cargo.toml`, the `turts_audit`
  dev-dependency of `turts_pack`, and the import edits in `crates/turts_pack/tests/`.
- The merge with A2b1 in `tests/m2/refutations.json` (L2): keep both row sets. The task that merges second
  resolves it. A2b1 does not merge before the CEO answer (plan Binding amendments, A2b1 stop ruling), so A1d
  will very probably merge first and A2b1 resolves at its rebase. Check: `python3 -m unittest
  tests/m2/test_refutations.py` passes and each row id is unique.

## 3. Amendment texts, ready to paste

**Design, replace D `:73-74`** (`docs/superpowers/specs/2026-11-12-pre-mp1-slice-design.md`):

```
- The ledger oracle of the tests (`crates/turts_pack/tests/common/conservation.rs`) is the seed of the domain
  files. It stays in the test tree as the independent source oracle: it reads the catalog from the source RON
  of the package, and the ledger audit reads the catalog from the log, which the code under test writes. The
  two rule sets stay apart on purpose: a fault that changes the installed catalog and the outcome together is
  red only in the source oracle (row C6, `tests/m3/refutations.json:158`; A1d ruling
  `docs/research/mp1/ruling-slice-a1d.md`). `turts_audit` holds one rule set, the ledger audit of a log. It
  holds no source oracle and no package reader.
```

**Design, new section 9 item 25** (after item 24):

```
25. The A1d ruling (technical-director, 2026-11-12, `docs/research/mp1/ruling-slice-a1d.md`) replaces D
    `:73-74` and the A1d exit "one oracle". There are two oracles with different trust roots: the ledger
    audit of the log (`turts_audit`) and the source oracle of the tests (`conservation.rs`). Rule `DMG` and the
    tick checks stay in the source oracle (rows C14 and C15). The four run constants that the log does not
    hold (`bounds`, `resources`, `compiled_durations`, `unit_types`) are held by `assert_run_constants` for
    sim code and by a writer scan in `turts_checks` for the production code of `turts_sim`, `turts_server` and
    `turts_pack`. The log format does not change for them in the slice. A digest in the log is an option for
    the M11 replay format (Proposed).
```

**TDI** (`docs/research/mp1/input-technical-director.md`). The request names `:35`; at `4598804a` the line
"Damage is out" is `:38` (`:35` is the `reservations.rs` line, which does not change). Replace `:38`:

```
- `combat.rs`: the ammo debit for each shot (SHOT). Damage is out, because health is not a ledger. (Amended by the A1d ruling, 2026-11-12: rule `DMG` stays in the source oracle `conservation.rs`, rows C14 and C15.)
```

Replace `:40`:

```
Seed the domains from conservation.rs. (Amended by the A1d ruling, 2026-11-12: conservation.rs then stays in the test tree as the independent source oracle. One ledger audit of the log lives in `turts_audit`; the source oracle reads the authored RON. `turts_pack` does not import `turts_audit` for it.)
```

TDI `:81` (step 4) gets the same marker:

```
4. (Amended by the A1d ruling, 2026-11-12.) Keep `common/conservation.rs` in the test tree. Close the A1c hand-over items. Run `scripts/m3-refutations.sh`: C1 to C17 must stay red.
```

**Plan, Binding amendments block** (`docs/superpowers/plans/2026-11-12-pre-mp1-slice.md`, after the A2b1 stop
ruling paragraph):

```
**From the A1d ruling (2026-11-12, technical-director; `docs/research/mp1/ruling-slice-a1d.md`; review `docs/research/mp1/review-slice-a1d.md`).** Binding for A1d and for the orchestrator's gates:
- Ruling (ii). `crates/turts_pack/tests/common/conservation.rs` stays in the test tree as the independent source oracle. D `:73-74` is replaced by D section 9 item 25. The A1d exit "one oracle" is void.
- A1d keeps from `a921096b`: `assert_run_constants` (`crates/turts_server/tests/economy.rs:138-156`) and its two call sites, the removal of the `bevy*` dependencies of `turts_audit`, the three `A1d-run-constants-*` rows (plants moved to real state), and their test in `tests/m2/test_refutations.py`.
- A1d drops: `crates/turts_audit/src/source.rs`, `pub mod source`, `ron` in `turts_audit`, the `turts_audit` dev-dependency of `turts_pack`, and the import edits.
- Review M1 is fixed in A1d by a writer scan in `turts_checks` for `MapBounds`, `ProductionDurations`, `UnitTypes` and the removal of the resources of `AuditResourcePresence`. The scan visitor gets a `ResMut<T>` / `Mut<T>` / `NonSendMut<T>` arm, and the movement writer scan gets the same arm. No log format change.
- The no-content scan does not see compound names (`ore_items`): a recorded gap, owner `qa-lead`, before the first MP1 package with new content ids merges.
- The `tests/m2/refutations.json` conflict with `pre-mp1/a2b1-fallback`: the task that merges second keeps both row sets.
```

**Plan, Task A1d text.** Replace the "Files:" line and the "Exit:" line:

```
**Files:** `crates/turts_pack/tests/common/conservation.rs` stays (A1d ruling). `crates/turts_server/tests/economy.rs` (`assert_run_constants`), `crates/turts_audit/Cargo.toml` (no `bevy*`), `tests/m2/refutations.json` and `tests/m2/test_refutations.py` (three `A1d-run-constants-*` rows), `crates/turts_checks/tests/` (the run-constant writer scan and the `ResMut` arm). `tests/m3/refutations.json` is not edited.
```

```
**Exit:** the same 17 M3 texts, `conservation.rs` byte-equal to `afb9c6de`, the A1c hand-over items closed, the run-constant writer scan red on its plants. **Runs: 2.**
```

## 4. Work list for the builder's repair pass (`backend-engineer`, on `pre-mp1/a1d-oracle`)

1. Revert to `afb9c6de`: `crates/turts_pack/tests/common/conservation.rs` (restore), `common/mod.rs`,
   `common/forge_support.rs`, `forge_cases.rs`, `forge_fixed_point.rs`, `forge_renamed_clone.rs`,
   `forge_timing.rs`, `crates/turts_pack/Cargo.toml`, `crates/turts_audit/src/lib.rs`. Delete
   `crates/turts_audit/src/source.rs`. In `crates/turts_audit/Cargo.toml` remove `ron`; keep the `bevy*`
   removal.
2. `Cargo.lock`: `git diff afb9c6de -- Cargo.lock` shows only the removal of `"bevy"`, `"bevy_app"`,
   `"bevy_math"` from the `turts_audit` block. `cmp` of `conservation.rs` with `git show afb9c6de:...` exits 0.
3. L1: move the plants of the three `A1d-run-constants-*` rows from `views.rs` to real state writes (one file per
   row; the registry has one `plant_path` per row). Use the reviewer's OWN-D and OWN-R texts
   (`movement/ecs.rs`). For unit types, a one-file real write, for example `register_unit_type` on
   `UnitTypes` each step in `movement/ecs.rs` (`crates/turts_sim/src/registry.rs:89-94`). Update
   `test_a1d_run_constants_rows_name_their_instrument` to the new paths.
4. M1: a writer scan `crates/turts_checks/tests/run_constant_writers.rs` (or a shared visitor with
   `movement_writers.rs`). Types: `MapBounds`, `ProductionDurations`, `UnitTypes`; plus `remove_resource::<T>`
   for each type of `AuditResourcePresence` (`crates/turts_sim/src/economy/views.rs:1176-1191`). Methods: the
   `WRITE_METHODS` of `movement_writers.rs:37-45` plus `remove_resource`. Add the `ResMut<T>`, `Mut<T>`,
   `NonSendMut<T>` arm to both scans. Pin the allowed entries as a multiset with a reason for each (the boot
   paths `map.rs:154`, `boot_m3.rs:1174-1178`, `bootstrap.rs:424-434`, and the restore removals
   `economy/transaction.rs:1807-1822`; verify each). One planted fixture file that goes red.
5. In `crates/turts_checks/tests/no_content.rs` add one "Known gap" header line for compound names.
6. Gates: F-RUST, F-FROZEN, `--set m3-all --workers 9 --alsa` (17 of 17 red, `"failed": []`), the combined
   set (with `m2_variants` built first), `cargo clippy --workspace --all-targets --locked -- -D warnings`,
   `bash scripts/boundary.sh`, `python3 -m unittest tests/m2/test_refutations.py`, `probe_neutral.sh`
   equal to the values of `afb9c6de` (d0d1b1c572163424, aa9db740578ed5c1), trace hashes equal.

Faults that the re-check (`qa-lead`) must apply, each red, each restored with `git status` empty:
- C6 (registry) and the reviewer's C6 probe: source oracle red with `REFUND`, ledger `Ok`.
- SRV-B (server, `get_resource_mut::<turts_sim::MapBounds>()` in `session.rs`): red in the new scan.
- A `ResMut<MapBounds>` system parameter in a server file: red in the new scan.
- A `ResMut<MovementPools>` parameter in `economy/combat.rs`: red in the movement scan.
- `remove_resource::<UnitTypes>()` in `economy/combat.rs`: red in the new scan.
- N12, OWN-D, OWN-R, OWN-U: red at `economy.rs:140`, `:148`, `:144`, `:152`.
- The three `A1d-run-constants-*` rows through the registry runner: red with their texts.
- Instrument checks: `rtk proxy grep -rn 'turts_audit::source' crates` exit 1; `ron` absent from
  `cargo metadata --locked` dependencies of `turts_audit` (a planted `ron` line makes the check red).

## 5. Findings of this ruling

- `docs/research/mp1/review-slice-a1d.md:32`: Low. The cite TDI `:35` for "Damage is out" is `:38` at
  `4598804a`. Fix: use `:38` in the amendments (done above).
- `crates/turts_checks/tests/movement_writers.rs:186-253`: Medium. No `ResMut<T>` arm; a resource writer of
  `MovementPools` is not seen. Fix: work item 4. Seen by reading and a grep; no plant run (NOT TRACED as red).
- `crates/turts_server/tests/economy.rs:73-75`: Medium (review M1). Fix: work item 4.
- `crates/turts_checks/tests/support/mod.rs:614-662`: Low. Compound-name gap. Fix: record (work item 5).

## 6. What I ran

| Run | Claim tested | Refuted if | Observed |
| --- | --- | --- | --- |
| R1 `run_plant.sh C6 a1d-td-c6-planted --probe -- cargo test -p turts_pack --locked --test zz_a1drev_c6` | review `:38-42`: under C6 the ledger is Ok and the source oracle is red | `ledger=Err` or `source=[]` | `refunded=10 ledger=Ok(()) source=["REFUND: job 1 refunds ... 10}, expected ... 5}"]`, `ledger_tampered=Err(...11...10...)`; restore clean, bootstrap.rs sha256 b42c8e55 equal |
| R2 same, plant NONE, label `a1d-td-c6-clean` | the probe can go red | probe green on clean | red at `zz_a1drev_c6.rs:149`, `refunded=5 ledger=Ok(()) source=[]`; tree clean |
| R3 grep `refund_ratio` in `turts_audit/src/production.rs` | review `:41-42`: ratio from logged catalog | ratio from a source file | `:37-43` reads `catalog.catalog` of the log |
| R4 grep `C6` in `tests/m3/refutations.json` | the registry depends on independence | no such note | `:158` "the independent refund oracle reads that authored ratio" |
| R5 `compound.py` (scratchpad `a1d-td/`) | "repair the compound-name hole now" is cheap | few false hits | 49 idents, 193 uses, 29 files; `client_shell` 46 |
| R6 grep `ResMut<` of scanned types in production | adding a `ResMut` arm adds no allowed entry | any hit | 0 hits |
| R7 grep `conservation` in the CEO decision file | D `:73-74` is a CEO decision | a hit | 0 hits in the decision; only D `:73` |
| R8 numbered read of TDI | "Damage is out" is at TDI `:35` | text at `:35` | it is at `:38` |
| R9 registry schema count (`tests/m2/refutations.json`) | a row can plant two files | a `plants` list | one `plant_path` per row (one form has `replacements` in one file) |
| R10 `git diff afb9c6de HEAD` (stat and files) | the revert list is complete | another user of `turts_audit::source` | 7 users, all in `crates/turts_pack/tests/` |

## 7. Not checked

- The release `turts_server` symbols (review NOT TRACED; moot under (ii)).
- The full 17 M3 cases on this branch; the re-check runs them.
- Whether the new scan sees `SRV-B` with the path form `turts_sim::MapBounds`: `written` checks every path
  segment (`movement_writers.rs:172-177`), so it should; the re-check runs it (NOT TRACED as red).
- Which compound hits of R5 are production code and which are `#[cfg(test)]` modules inside `src` files.

## What I changed

`docs/research/mp1/ruling-slice-a1d.md` (this file, not committed). My memory directory. No code change; the
lane was restored after each run (`git status --porcelain` empty, HEAD `a921096b`).
