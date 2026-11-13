---
name: pre-mp1-a1d-review-2026-11-12
description: A1d review at a921096b - conservation.rs moved into turts_audit::source (two rule sets, not one oracle); C6 ledger Ok / source REFUND; server-side run-constant change green on 865 tests
metadata:
  type: project
---

Verified 2026-11-12, lane TuRTS-pre-a1d-review, cache mc-pre-a1d-review, labels a1d-rev-*. Base afb9c6de.
Report: TuRTS-mp1/docs/research/mp1/review-slice-a1d.md. Scripts: scratchpad a1d-review/ (plant.py, run_plant.sh,
zz_a1drev_c6.rs, summary.py).

- Combined set at a921096b: 76 binaries, 865 passed, 351 s. N12: 37 failed (= builder; A1c had 34, +2 run-constant
  asserts economy.rs:140, +1 sweep test).
- C6 probe (asserting): clean red at "refunds 5"; planted: refund 10, validate_ledger Ok, source oracle "REFUND: ...".
  Tamper refund +1 gives ledger Err (production rule), so the ledger reads refunds; it reads the ratio from the log.
- Builder's own C6 probe printed the verdict and asserted nothing.
- Real-state plants in movement/ecs.rs (durations +1, remove ProductionDurations, UnitTypes radius +0.001 via a
  pub(crate) helper in registry.rs) red at economy.rs:148/:144/:152 in the two live-world tests only.
- SRV-B: server `admit_player_commands` grows MapBounds each tick: all 865 green; marker file proved it ran.
  Server-side run-constant writes are seen by no non-ignored test.
- no_content scan matches whole tokens: `shell` ident and literal "shell" red; `shell_count` and "BAL-shell" green.
- nm on debug turts_server: 0 `turts_audit::source` symbols (38 turts_audit symbols): linker drops the test oracle.
- Serial M3 runner (tests/m3/refutations.py) makes its own fresh cargo target; point --build-root at scratch, delete.
- merge-tree vs a2b1-fallback f119dc39: one append conflict in tests/m2/refutations.json; A1d anchors survive.

See [[pre-mp1-a1c-review-2026-11-12]].

Re-check 2026-11-12 at e2ba109c (ruling (ii); labels a1d-rerev-*): PASS, 0 High. Report rereview-slice-a1d.md.
- Revert cmp-equal (9 files), lock diff = 3 bevy lines; `cargo metadata --no-deps --locked` succeeds with a planted
  ron line (the lock check needs full metadata), so parse the dependency list instead (scratch meta_check.py).
- New run-constant scan (turts_checks run_constant_writers.rs + support/writers.rs): all ruling faults red.
  NOT seen (all compile): copy-modify-insert_resource (MapBounds is Copy), commands.insert_resource, 
  get_resource_or_init / get_resource_or_insert_with (not in either method list), type alias, `use as`, macro body.
  Seen: resource_scope closure `Mut<T>`. Proposed fix: runtime change-tick guard (needs decision).
- C6 probe after revert: put it in turts_server/tests with `#[path]` to conservation.rs (turts_pack lost turts_audit).
- Serial runners (tests/m2|m3/refutations.py --build-root scratch): m3 all 468 s, 3 m2 rows 116 s; ~480 MB build.
- Merge test method: merge-tree oid -> archive -> union JSON -> git init scratch lane + empty target/mc-* -> runner.
