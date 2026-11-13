VERDICT=FAIL HIGH=1
RULING-PROPOSAL=(ii) Revert the move: `conservation.rs` stays in the test tree as the independent source oracle, A1d closes with the hand-over items, and D `:73-74` changes (NEEDS-TD).

# Review of pre-MP1 slice task A1d (qa-lead, 2026-11-12)

Candidate `a921096b` on `pre-mp1/a1d-oracle` (2 commits on `afb9c6de`). Lane
`/home/julien/Projects/TuRTS-pre-a1d-review`, cache `target/mc-pre-a1d-review`, labels `a1d-rev-*`
under `/home/julien/Projects/TuRTS/target/mc-evidence/`. Scripts:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/a1d-review/`
(`plant.py`, `run_plant.sh`, `zz_a1drev_c6.rs`, `summary.py`, outputs `*.out`).

The verdict is FAIL under ruling (ii). Under ruling (i), the High becomes a plan amendment and the
candidate is PASS with the two Medium findings open.

## A. The deviation

What A1d is for. D `:73-74` (design section 3, gated): "The ledger oracle of the tests
(`conservation.rs`) is the seed of the domain files, and is then replaced by `turts_audit`, so one
oracle remains." TDI `:40`: "Then `turts_pack` imports `turts_audit`, so one oracle remains, not
two." TDI step 4 `:81`. The debt is two implementations of the same ledger rules (stock, reservations,
refund, dangling, shot). A change to one rule must then be made in two places.

The result does not remove that debt. The file moved byte-equal (section C). `turts_audit` now holds
two rule sets with the same overlap as before. The plan text is met only by its letter ("replaced by
calls to `turts_audit`"). Its exit line "one oracle" is not met.

Does the purpose conflict with oracle independence? Partly. The C6 probe below proves that a log-only
rule cannot see a fault that changes the installed catalog and the outcome together. But that is a
property of the catalog source, not of the number of rule implementations. A merged rule could take
its catalog as a parameter. The real conflict is different: two rule sets that were written apart are
diverse redundancy. A merge makes one defect blind both oracles (common mode). Also TDI `:35` says
"Damage is out" of the ledger, but rows C14 and C15 pin `DMG` from `conservation.rs`. So a full
replacement was never possible as written.

C6 probe (mine, it asserts; `zz_a1drev_c6.rs`, put in `crates/turts_pack/tests/` for the run only):
- Clean build: RED at `zz_a1drev_c6.rs:149` "plant C6 is not active (clean build refunds 5)";
  `refunded=5 ledger=Ok(()) source=[]`.
- Plant C6 (registry anchor in `bootstrap.rs`): GREEN. `refunded=10 ledger=Ok(())
  source=["REFUND: job 1 refunds {…shell: 10}, expected {…shell: 5}"]`.
- Instrument check in both runs: the logged refund plus 1 gives `validate_ledger` Err
  "production: job 1 refunds … 11 … give … 10 (operation 20)". So the ledger reads the refund and
  takes the ratio from the logged catalog.
- Restore: `git status --porcelain` empty, `bootstrap.rs` sha256 equal to pristine.
- The builder's probe (`scratchpad/a1d/zz_a1d_probe_c6.rs`) asserts nothing. It prints the verdict
  and passes on every build.

Ruling options.
- (i) Accept the move. Cost: none now. Against it: the debt stays; a forge-shaped test oracle and a
  second package reader enter an engine crate (finding M2); identifiers were renamed to pass the scan.
- (ii) Revert the move (recommended). Keep `assert_run_constants`, its three rows and the
  `test_refutations.py` test, the removal of the `bevy*` dependencies, and the E8 note. Drop
  `pub mod source`, `ron` in `turts_audit`, the `turts_audit` dev-dependency of `turts_pack`, and the
  import edits. Cost: about 1 run. NEEDS-TD for D `:73-74`. Amendment text: "The ledger oracle of the
  tests (`crates/turts_pack/tests/common/conservation.rs`) is the seed of the domain files. It stays in
  the test tree as the source oracle: it reads the catalog from the package source RON, and the ledger
  audit reads it from the log. The two stay independent on purpose (A1d review: plant C6 is red only
  in the source oracle)." Plan A1d: Files "conservation.rs stays"; Exit "the same 17 texts; the A1c
  hand-over items closed".
- (iii) A real merge. Five of the seven invariants overlap (BAL-ore, BAL-shell, RES, REFUND,
  DANGLING; SHOT partly). DMG and the tick checks stay test-side. The shared core must return
  structured errors with two formatters, because the 17 M3 texts and the A1b and A1c ledger rows pin
  different bytes. It also needs a catalog parameter (log or source RON). Cost: about 1,100 lines
  rewritten, 3 or more runs, one new review. Cover risk: common mode (one defect blinds both oracles),
  and a rule that moves from `AuditState` to the narrower `LedgerAudit` can lose a stock place.

## Findings

H1 (High). `crates/turts_audit/src/lib.rs:44`, `crates/turts_audit/src/source.rs:1-35`. A gated
design line (D `:73-74`, "one oracle remains") is not met, and the builder did not stop for a
ruling. Fix: the ruling above.

M1 (Medium). `crates/turts_server/tests/economy.rs:73-75`, `:138-156`. A run constant that server
code changes is seen by no non-ignored test. Plant SRV-B: `admit_player_commands`
(`crates/turts_server/src/session.rs:382`) adds 1 to `MapBounds.size.y` each tick. Result: 865 passed,
0 failed. A marker file proved that the system ran. `assert_run_constants` covers only sim code in two
tests. Fix: an immutable digest of `bounds`, `resources`, `compiled_durations` and `unit_types` in
`LedgerAudit` with a ledger rule "unchanged after Initial", or a writer scan for these resources.
Needs decision (the log format changes).

M2 (Medium). `crates/turts_audit/src/source.rs:76-113`, `:128-170`; `crates/turts_audit/Cargo.toml:12`.
The engine crate holds a forge-shaped oracle (one node item, one ammo output, `BAL-ore` and
`BAL-shell`) and its own package reader. The boundary forbids `turts_mod_runtime` to this crate
(`scripts/boundary.sh:71-75`), and D `:63-64` does not list `ron`. The scan passes only because it
matches whole tokens (section B). This goes away with (ii).

L1 (Low). `tests/m2/refutations.json` rows `A1d-run-constants-*`. The plants change the reporter
`audit_state` in `views.rs`, not the world state. My real-state plants were red (section D), so the
check holds. Fix: move the row plants to a real write.

L2 (Low). A merge conflict with `pre-mp1/a2b1-fallback` in `tests/m2/refutations.json` (section F).

## B. Product crate and content rule

- `turts_audit` is a normal dependency of `turts_server` (`crates/turts_server/Cargo.toml:10`).
  `ron` was already one (`:20`). So `source.rs` is compiled into a crate that the server links.
- `nm -C` on the debug `turts_server` binary: 38 `turts_audit::` symbols, 0 `turts_audit::source`
  symbols. The linker drops the oracle. The release binary was not checked (NOT TRACED).
- The crate table of `CLAUDE.md` has no `turts_audit` row (this is old debt). `boundary.sh` passes.
  The defect is one of design (M2), not of the binary. If (i) is chosen, put `source` behind a
  feature that only the `turts_pack` dev-dependency turns on.
- Scan probe SCAN in `source.rs`: `let shell` (line 212), `"shell"` (214) and the use of `shell` (216)
  are red, 4 hits each. `shell_count` (213) and `"BAL-shell"` (215) are green. The rule matches whole
  tokens (`crates/turts_checks/tests/support/mod.rs:614-662`, `:686-697`). So `ore_items` and
  `shell_items` (`source.rs:135`, `:159`) and the texts pass. This is a hole in the scan for compound
  names, not a real exception. It was already in the scan before A1d.

## C. Byte-equal texts and rules

`git diff --word-diff=porcelain` (token regex) of `afb9c6de:…/conservation.rs` against
`a921096b:…/source.rs`. The only changed tokens are: `ore` to `node_item` (5), `shell` to `ammo_item`
(2), `belt` to `line` (2); the module header `//!`; and the removal of the free function `named`
(old `:176-178`), which had no caller (the method `CheckFailure::named`, old `:27`, stays). No
literal, comparison, or order of checks changed. `tests/m3/refutations.json` is not edited.

## D. `assert_run_constants`

- N12 (bounds grow in `movement/ecs.rs`): combined set 828 passed, 37 failed (the same as the
  builder). The two log tests are red at `economy.rs:140`.
- OWN-D (the real `ProductionDurations` plus 1 each step): red at `economy.rs:148`.
- OWN-R (`ProductionDurations` removed each step): red at `economy.rs:144`.
- OWN-U (`UnitTypes` radius plus 0.001 each step, a `pub(crate)` helper in `registry.rs`): red at
  `economy.rs:152`.
- Each of these ran on `--test economy`: 5 failed (the two log tests, `full_state_rebuilds_the_audit_state`,
  and two stateful oracle tests). Each was restored: `git status` empty, sha256 equal.
- What stays uncovered: the process tests (`economy.rs:73-75` says so) and server code (M1). For sim
  code, the cover is enough. For server code, it is not (needs decision).

## E. Runs

- Combined set (with the `m2_variants` example built first): 76 binaries, 865 passed, 0 failed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0.
- `bash scripts/boundary.sh`: "boundary ok", exit 0.
- Trace hashes: base `d4c73ebe…`, haul `805309b2…`, acme `19dcf9a3…`, cedar `0db10e01…`, and the
  stream `bcbf72c2…`. All equal to the expected values.
- `probe_neutral.sh`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`. Equal.
- M3 serial runner, 7 cases (C1, C5, C6, C9, C10, C13, C14): 7 RED with their texts (RES:, BAL-shell,
  REFUND, BAL-ore, DANGLING, SHOT, DMG), 7 RESTORED GREEN. The scratch build was deleted.

## F. Merge check

- Against `origin/pre-mp1/integration` (= `afb9c6de`): `git merge-tree` is clean (tree `3bc118f7`).
- Against `origin/pre-mp1/a2b1-fallback` (`f119dc39`): one conflict in `tests/m2/refutations.json`.
  Both branches append rows at the end of the array. Keep both sets. The three A1d anchors occur once
  each in the merged `views.rs`. There is no conflict in `ledger_logs.rs` or in the sim.

## Not traced

- The ignored process tests (acceptance) with plant SRV-B. I did not run them. The test comment
  (`economy.rs:73-75`) says that they do not see the four fields.
- The release `turts_server` symbols.
- The full 17 M3 cases on my side (the builder reports 17 of 17; I ran 7).
- The ci-browser and EL1 runs (not in the scope of A1d).

## What I changed

None in the repository. The probe and the plants were put in the lane for each run and removed after it
(`git status` empty after each run). My memory: `.claude/agent-memory/qa-lead/pre-mp1-a1d-review-2026-11-12.md`.
This report file (not committed).
