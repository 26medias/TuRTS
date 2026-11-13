VERDICT=PASS HIGH=0

# Re-check of the A1d repair pass (qa-lead, 2026-11-12)

Candidate `e2ba109c` on `pre-mp1/a1d-oracle` (3 commits on `afb9c6de`). Ruling (ii),
`docs/research/mp1/ruling-slice-a1d.md`. Lane `/home/julien/Projects/TuRTS-pre-a1d-review`, cache
`target/mc-pre-a1d-review`, labels `a1d-rerev-*` under `/home/julien/Projects/TuRTS/target/mc-evidence/`.
Scripts and outputs: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/a1d-review/`
(`plant.py`, `run_plant.sh`, `zz_a1drerev_c6.rs`, `meta_check.py`, `r-*.out`).

The ruling is met. The repair has 0 High findings. It has two Medium findings in the new scan: a
write that the scan does not see is still possible in real code. Fix both before MP1 adds server
systems. The repair does not need to stop for them.

## 1. The revert

- `git show afb9c6de:<file> | cmp - <file>` gives exit 0 for each of the 9 named files:
  `conservation.rs`, `common/mod.rs`, `common/forge_support.rs`, `forge_cases.rs`, `forge_fixed_point.rs`,
  `forge_renamed_clone.rs`, `forge_timing.rs`, `crates/turts_pack/Cargo.toml`, `crates/turts_audit/src/lib.rs`.
- `crates/turts_audit/src/source.rs` does not exist. `rtk proxy grep -rn 'turts_audit::source' crates`
  gives exit 1.
- The `cargo metadata --locked --no-deps` dependencies of `turts_audit` are serde, serde_json, turts_mod_api,
  turts_net_protocol and turts_sim. There is no `ron`. `turts_pack` has no `turts_audit` dependency.
  Instrument check: with a planted `ron` line in `turts_audit/Cargo.toml`, my parse goes red (`ron present: True`,
  exit 1) (`a1d-rerev-meta-ron`). After the restore, `git status` is empty.
- `git diff afb9c6de e2ba109c -- Cargo.lock` removes only `"bevy"`, `"bevy_app"` and `"bevy_math"` from the
  `turts_audit` block.

## 2. Fault table

A plant is applied, the command runs, the plant is restored, and `git status --porcelain` is empty after each
run. The restored sha256 is equal to the pristine sha256 each time.

| Fault | Command | Result |
| --- | --- | --- |
| C6, my probe (`zz_a1drerev_c6.rs` in `turts_server/tests`; `conservation.rs` by `#[path]`), clean build | `cargo test -p turts_server --test zz_a1drerev_c6` | RED at `:170` "plant C6 is not active (clean build refunds 5)"; `ledger=Ok source=[]` |
| C6, probe, planted | same | GREEN: `refunded=10 ledger=Ok(()) source=["REFUND: job 1 refunds …10}, expected …5}"]`. A tampered refund +1 makes the ledger Err in both runs |
| C6, registry | M3 runner, `--case all` | RED `operation 20: REFUND: …`, RESTORED GREEN |
| SRV-B (`get_resource_mut::<turts_sim::MapBounds>()` in `session.rs`) | `run_constant_writers` | RED `("…session.rs", "method", "MapBounds") pinned 0, found 1 at …session.rs:384` |
| `ResMut<turts_sim::MapBounds>` parameter in `session.rs` | same | RED `"mut-param", "MapBounds" … found 1 at session.rs:382` |
| `ResMut<MovementPools>` parameter in `economy/combat.rs` | `movement_writers` | RED `("…combat.rs", "mut-param", "MovementPools", 1) pinned 0, found 1 at combat.rs:82` |
| `remove_resource::<UnitTypes>()` in `economy/combat.rs` | `run_constant_writers` | RED `"method", "UnitTypes" … found 1 at combat.rs:83` |
| N12 (bounds grow, `movement/ecs.rs`) | combined set | 830 passed, 38 failed (the same as the builder: 37 before, plus the new scan); the two log tests are red at `economy.rs:140` |
| OWN-D (real durations +1) | `--test economy` | 5 red; the two log tests are red at `economy.rs:148` |
| OWN-R (`ProductionDurations` removed) | same | 5 red; red at `economy.rs:144` |
| OWN-U (`UnitTypes` radius +0.001) | same | 5 red; red at `economy.rs:152` |
| The three `A1d-run-constants-*` rows | `tests/m2/refutations.py --case <3 ids>` | 3 RED, 3 RESTORED GREEN. Each panic is at `economy.rs:148`, `:152` or `:144`, which is `assert_run_constants`. The text occurs only at `economy.rs:142-154` |
| R-PACK (`get_resource_mut::<MapBounds>()` in `turts_pack/src/lib.rs`) | `run_constant_writers` | RED `found 1 at …turts_pack/src/lib.rs:14` |
| R-MAP2 (a second `MapBounds {}` in the allowed file `map.rs`) | same | RED `"construct", "MapBounds") pinned 1, found 2 at map.rs:24, map.rs:158` |
| R-BOOT2 (`get_resource_mut::<MapBounds>()` in the allowed file `bootstrap.rs`) | same | RED `"method", "MapBounds") pinned 0, found 1 at bootstrap.rs:441` |
| W1 (mutant: the `&mut` arm of `support/writers.rs:94` inverted) | both scans | RED in `movement_writers.rs:166` (fixture) and in both pinned tests. The fixture test of `run_constant_writers` stays GREEN (finding L1) |
| R-FORMS (eight write forms in one server file; `cargo check -p turts_server` compiles them, 8 dead-code warnings) | `run_constant_writers` | Only 1 of 8 is seen (finding M1, table in section 3) |

The rows plant real state now: all three plant in `movement/ecs.rs` (`world.get_resource_mut`,
`world.remove_resource`, `UnitTypes` registration). The test `tests/m2/test_refutations.py:187-190` pins that.

## 3. The new scan: forms that it does not see

R-FORMS put each form in `crates/turts_server/src/session.rs` before `admit_player_commands`. `cargo check`
compiles all of them, so each one is legal code.

| Form | Seen | Real code style? |
| --- | --- | --- |
| `let mut b = *world.resource::<MapBounds>(); b.size.y += 1.0; world.insert_resource(b);` | no | Yes. `MapBounds` is `Copy` (`turts_sim/src/world.rs:40`). `bootstrap.rs:442-450` inserts received values in this form |
| `commands.insert_resource(next)` (a copy from `Res<MapBounds>`) | no | Yes (a deferred system) |
| `world.get_resource_or_init::<UnitTypes>()` (returns `Mut<T>`, bevy_ecs 0.20 `world/mod.rs:2434`) | no | Yes. `UnitTypes` is `Default` (`registry.rs:23`) |
| `get_resource_or_insert_with` (`world/mod.rs:2387`) | no, unless the closure builds `T {..}` | Yes |
| `type ZzBounds = MapBounds;` + `get_resource_mut::<ZzBounds>()` | no | Rare |
| `use turts_sim::MapBounds as ZzArea;` + `ResMut<ZzArea>` | no | Rare |
| a write inside a `macro_rules!` body | no | Rare |
| `resource_scope(\|_, mut b: Mut<MapBounds>\| …)` | yes (`mut-param`, `session.rs:407`) | Yes (`economy/gather.rs:229` has this form) |

The ruling items `boot_m3.rs:1172-1178` and `economy/transaction.rs:1807-1822` are in `#[cfg(test)]` code, so
they are not pinned. This is correct: `boot_m3.rs:626-627` opens `#[cfg(test)] mod tests` (it ends at `:1254`).
`transaction.rs:1638-1639` opens `#[cfg(test)] mod raw_audit_capture_tests` (it ends at `:1828`). The builder
cites `:1104-1105`, which is a different test module, but its conclusion holds.

The allowed entries are pinned by (file, kind, type, count) (`run_constant_writers.rs:80-100`). A second write
in an allowed file is red, whether it has the same kind (R-MAP2) or a new kind (R-BOOT2). An entry is a file,
not a function. So a boot write that moves into a per-tick function of the same file, with the same count,
stays green. That is Low, and code review sees it.

## 4. The moved visitor

- `support/writers.rs` is the old private visitor of `movement_writers.rs`, plus the `mut-param` arm
  (`:102-116`) and the method list as a parameter. The token diff shows no other change.
- The movement table (`PINNED`) is unchanged. The old fixture lines are unchanged; three lines are new
  (`fixtures/movement_writers/planted.rs:43-55`).
- The movement scan is red on its fixture under W1, and red on my `ResMut<MovementPools>` plant.
- The movement scan also does not have `get_resource_or_init` or `get_resource_or_insert_with`. `MovementPools`
  is `Default`, so M1 applies to that scan too.

## 5. Gates (all exit 0 unless named)

- Full set `-p turts_audit -p turts_sim -p turts_server -p turts_pack -p turts_checks -p turts_client`:
  106 binaries, 1300 passed, 0 failed (`a1d-rerev-full-set`).
- Combined set with `--features turts_sim/test-probes` (with `m2_variants` built first): 77 binaries, 868 passed
  (`a1d-rerev-combined-clean`).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0.
- `bash scripts/boundary.sh`: "boundary ok".
- `python3 -I -B tests/m2/test_refutations.py`: ran 21 tests, OK.
- Trace sha256: base `d4c73ebe…cdd7b2`, haul `805309b2…bff3148`, acme `19dcf9a3…b91d1d2`, cedar
  `0db10e01…5f33718`. The stream is `bcbf72c2…08e370b`. All are equal to the expected values.
- `probe_neutral.sh`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`. Equal.
- M3, all 17 cases (serial runner `tests/m3/refutations.py --case all`, scratch build, deleted after): 17 RED
  with their texts, 17 RESTORED GREEN, 468 s (`a1d-rerev-m3-all`).

## 6. Merge check

- Against `origin/pre-mp1/integration` (`afb9c6de`): `git merge-tree` is clean (tree `2a4f4408`).
- Against `origin/pre-mp1/a2b1-fallback` (now `19187cb8`): one conflict in `tests/m2/refutations.json`.
  `movement_writers.rs` merges with no conflict (A2b1 edits its table, and A1d moves its visitor). Test of the
  merge: I took the merge tree `da24dcc8` and wrote the JSON as the base 56 rows, plus the 3 A1d rows, plus
  the 13 A2b1 rows (72 rows; ids unique; neither side edits a base row). I committed it in a scratch lane
  and ran:
    - `python3 -I -B tests/m2/test_refutations.py`: ran 21, OK (`a1d-rerev-merge-a2b1-unittest`);
    - `cargo test -p turts_checks --test movement_writers --test run_constant_writers`: 2 passed and 3 passed
      (`a1d-rerev-merge-a2b1-scans`). So the A2b1 movement table is valid under the shared visitor, and the A2b1
      sim code has no unpinned run-constant write.
  I deleted the scratch tree. The three JSON inputs stay in `scratchpad/a1d-review/merge-a2b1/`.

## Findings

- **M1 (Medium)** `crates/turts_checks/tests/run_constant_writers.rs:45-55`, `movement_writers.rs` (its method
  list). The method lists do not have `get_resource_or_init` or `get_resource_or_insert_with`. Both return
  `Mut<T>` (bevy_ecs 0.20 `world/mod.rs:2387`, `:2434`). Reproduction: R-FORMS, line `zz_or_init`, not seen.
  Fix: add the two names to both lists, and a fixture line for each.
- **M2 (Medium)** `crates/turts_checks/tests/support/writers.rs:14-15` (the known gap). A copy of a run constant
  that is changed and inserted again (`world.insert_resource(b)`, `commands.insert_resource(next)`) is not seen.
  This is the most natural form for the `Copy` type `MapBounds`. Code review is the only cover today.
  Fix (needs decision): a runtime guard that reads the Bevy change ticks. A system after boot asserts that
  `Res<MapBounds>`, `Res<ProductionDurations>` and `Res<UnitTypes>` are not changed, and that the presence set
  is equal. It sees every form, also in server code, so it covers the forms that no static scan can see.
- **L1 (Low)** `crates/turts_checks/tests/fixtures/run_constant_writers/planted.rs`. The fixture has no
  `&mut MapBounds` line, so the `&mut` kind is not planted in this scan. Under W1 its fixture test stays green.
  Fix: add one `fn f(b: &mut MapBounds)` line.
- **L2 (Low)** `run_constant_writers.rs:80-100`. An entry is a file, not a function (section 3).
- **L3 (Low)** `docs/research/mp1/repair-slice-a1d-report.md` cites `transaction.rs:1104-1105`. The removals are
  in `raw_audit_capture_tests` (`:1638-1828`).
- Rare forms that are not seen (a type alias, `use … as`, a write in a macro body): record them in the
  known-gap line of `writers.rs`.

## Owed to the orchestrator (no browser started)

- The 8 `ci-browser` cases, headed, v6 in pre-merge mode through the shim
  `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/a1c-review/headed_v6_premerge.sh`:
  `python3 /home/julien/Projects/TuRTS/target/mc-evidence/root-regate/parallel_refutations.py --set ci-browser --worktree /home/julien/Projects/TuRTS-pre-a1d-review --evidence-root /home/julien/Projects/TuRTS/target/mc-evidence/root-regate/runs --build-root /home/julien/Projects/TuRTS-pre-a1d-review/target/par-browser --label a1d-rerev-browser --workers 4 --seed-jobs 6 --wasm-slots 2 --alsa --headed /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/a1c-review/headed_v6_premerge.sh --lane-target mc-pre-a1d-review --lane-head e2ba109c`.
  Expected: `"failed": []`. Delete `target/par-browser` after the run.
- EL1 in release: `cargo test --release -p turts_server --locked --test el1 -- --ignored --test-threads=1 --exact el1_muster_movement_and_queued_gather_validate_every_captured_tick`
  (through `exec_in_lane_v4.py --audio`; `scripts/acceptance.sh:140`). Expected: exit 0.

## NOT TRACED

- `scripts/acceptance.sh` on this head (the builder reports exit 0 in 1279 s). Thus also SRV-B against the
  ignored process tests.
- The ci-browser cases and EL1 (owed, above).
- The parallel form `--set m3-all --workers 9`: I ran the serial runner (17 of 17).
- The `NonSendMut` arm with a plant of mine. The builder's arm-off mutant and the fixture line `:51` cover it.
- The count of 49 compound names in the `no_content.rs` header (from the ruling, run R5).

## What I changed

None in the repository. Every plant and probe was removed after its run, and `git status --porcelain` was empty
at the end (HEAD `e2ba109c`). I wrote this report file (not committed) and my memory file
`.claude/agent-memory/qa-lead/pre-mp1-a1d-review-2026-11-12.md`.
