VERDICT=pass-with-changes FINDINGS=8 HIGH=2 RUNS=17 NOT_CHECKED=5

# MC plan gate 2: technical-director (boundary and sequencing)

Artefact: `docs/superpowers/plans/2026-10-29-mc-player-control.md` at `f69badf` (716 lines).
Mode: review-plan. Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate2/td/`.
Status of the items: D15 Baseline (no Bevy type in the mod API); D18 Decided (lightyear); D65, D67,
D73 Decided line, design Proposed; D74 Proposed; K1 to K27 decided by the CEO (plan:13).

## Findings

1. **High.** `plan:519-521`, `plan:564`, `plan:567`. Tasks 5 and 15 put headless tests in
   `crates/turts_pack/tests/` (`mc_orders.rs`, `mc_feedback.rs`) that call `orders::to_sim`,
   `judge_hello` and the rate buckets. These live in `turts_server`, which has no lib target
   (`crates/turts_server/src/` has `main.rs`, no `lib.rs`; no crate depends on it). Probe: a
   dev-dependency gives "ignoring invalid dependency `turts_server` which is missing a lib
   target" and `E0432`. Fix: put the tests in `crates/turts_server/tests/` with
   `#[path = "../src/orders.rs"]` (pattern of `crates/turts_server/tests/economy_export.rs:1`,
   `export_m3.rs:5`) or in `mod tests` of the binary. A new library crate is a new boundary: needs decision.
2. **High.** `plan:399`, `plan:406`; D:1084-1088. The no-content scan hits design names of
   `muster`: unit `marker` (6 hits, first `crates/turts_client/src/live_view.rs:268`) and order
   `gather` (17 hits, first `crates/turts_server/src/export.rs:11`, module `gather`). Forge order
   ids also hit: `enable` 2, `disable` 5, `remove` 37, `research` 115. Task 18 has a rename rule
   (`plan:386`); task 19 stops on a hit (`plan:406`), and the exception list is fixed
   (`crates/turts_checks/tests/support/mod.rs:232-245`). Effect: a stop in wave 2 before tasks
   5, 10 to 13, 17, 22. Fix (needs decision, as O1): rename `marker`; give task 19 the rename
   rule for order ids. Zero hits: `beacon`, `picket`, `pawn`, `decoy`, `order_gather`,
   `trooper`, `post`, `dummy`, `rush`, `ore`, `stop`, `hold`, `attack`, `cancel`, `move`.
3. **Medium.** `plan:335`, `plan:391`. Expected greens are false. Task 3 step 1: the API bump makes
   `crates/turts_mod_runtime/tests/t2c_package_rejects.rs:57` red (asserts "0.2.2"). Task 18: after
   `acme` goes to `^0.2.3`, t2c `:52`, `:64` are red (the anchor `engine_api: "^0.2"` no longer
   occurs), and `e10_cedar_uses_public_pack_and_boot` (`crates/turts_server/tests/economy.rs:3182`,
   `expect_err` at `:3187`; run by `scripts/acceptance.sh:92`) panics. None of these files is on a
   Files line. Fix: list them in tasks 3 and 18 with the new anchors.
4. **Medium.** `plan:42-49`. The Codex contract has gaps. It does not state that each task branch
   starts from `codex/mc-integration` after its Needs merge. It names no worktree per lane (trunk
   `dist/` and the checkout are shared). It has no re-gate on the integration branch after each
   merge (semantic conflicts, for example task 12 removes `economy_ui.rs` while task 11 edits
   `render.rs` in wave 5). It has no push and CI read after each wave (QA0 brief
   `vault/Reports/2026-10-29 QA0 Codex brief.md:58` had it; MC reads CI only at `plan:634,665`).
5. **Medium.** `plan:144-173`. The shared-file table misses edits. `Cargo.lock` records member
   edges (`Cargo.lock:8596-8610`), and `--locked` refuses a new path dependency (probe exit 101).
   So task 9 (`turts_client` to `turts_interaction`) edits `Cargo.lock`, and tasks 5, 14, 15 edit
   `crates/turts_pack/Cargo.toml` and `Cargo.lock` for dev-dependencies (14 and 15 share wave 7).
   Task 4 adds sim reasons, so it must edit `crates/turts_server/src/outbound.rs:55-90`
   (exhaustive `wire_reason`) and `export.rs:431`; `outbound.rs` is not listed for task 4.
6. **Medium.** `plan:104`, `plan:110`, `plan:153`. Debug values have no owner. `turts_debug` is
   `debug_json` (`crates/turts_client_web/src/lib.rs:6`, `crates/turts_client/src/debug.rs:31`)
   and `--out` is `report_json` (`debug.rs:212`). Task 9 adds keys "with an empty value". Tasks 12
   (`cursor`), 13 (`selection`), 15 (`results`, `audio`, `alerts`, `notices`), 16 (`focus`,
   `contextmenu`, `player_id`) and 10 (`--out` `selection`, `groups`, D:273) need values, but the
   table gives `debug.rs` to 2b, 9, 12 only; 10 and 12 share wave 5. Fix: task 9 makes each key
   read a typed resource that the owning module writes.
7. **Medium.** `plan:206`, `plan:130-140`. Budget: the line "Gate 1 and gate 2: 14" is spent.
   Gate 1 left 17 lens reports in `docs/research/mc/` (5 lenses, up to 3 re-checks). The Needs
   list omits 13 needs 4 and 6 (`UnitOrderRecord`, `Blocked`), and 15 needs 4 (per-unit
   validation, ORD2 plant in `command.rs`). The waves still order them (`deps.py`).
8. **Low.** `plan:374`. The T1X copy does not copy `Cargo.lock`; `boundary.sh:15` runs
   `cargo tree` without `--locked`, so the copy resolves fresh and needs the network. With the
   lock copied, my probe gave the exact line. The `forbid` names are exact: `wgpu-types` or
   `wgpu-core` alone passes. Fix: copy the lock; optionally add those names.

Checks that hold: K1 tree; T1X line; waves against Needs; critical path 51 and subtotal 106;
frozen hash; stream JSON holds package ids only, so the task 18 version bump keeps G-FROZEN;
version table semantics at 0.2.3; `members = ["crates/*"]` takes the new crate; t7 and
server tests pick free ports; R1 to R11 change no Decided line and no K call.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `bash scripts/boundary.sh; echo exit=$?` | baseline green, `plan:712` | exit 1 | 11 `checked:` lines, `boundary ok`, exit=0 |
| scratch crate, own `[workspace]`, `cargo tree -e normal --prefix none` host and wasm32 | `turts_interaction` deps hold K1, `plan:21,371` | a `bevy`, `wgpu`, `winit`, `turts_sim` line | 0 lines on both; glam v0.33.12 once |
| `boundary.sh --rule no-bevy-prefix --manifest <copy + bevy_render>` host, wasm32 | T1X line, `plan:374` | other text or exit 0 | exit 1, `forbidden edge: turts_interaction -> bevy_app` on both; clean copy `rule ok` |
| `python3 files.py` over the Files lines | no file shared in one wave, `plan:114` | a same-wave path | none by Files lines; findings 5, 6 come from code |
| `python3 deps.py` | waves and 51 runs, `plan:142` | a violation or other sum | 0 violations, 51, 106 |
| `ls crates/turts_server/src`; probe dev-dependency and `cargo check` | `turts_pack` tests can call server code, `plan:521` | it compiles | missing lib target, `E0432` |
| `cargo test -p turts_checks` scratch probe on `scan_rust` | names pass the scan, `plan:399,406` | 0 hits | 182 hits for the names of finding 2; 0 for the candidates |
| scratch tree, API 0.2.3, `cargo test --test t2c_package_rejects` | task 3 step 1 green, `plan:335` | pass | 1 failed (`:57`) |
| same plus `acme` at `^0.2.3` | task 18 G-RUST green | pass | 2 failed (`:52`, `:64`) |
| same, `cargo test --test bad_packages` | my suspicion that `bad_packages.rs` breaks | a failure | 16 passed: suspicion refuted |
| python replace on migrated `cedar` | e10 anchor still matches | text changes | no-op |
| cargo `--locked` probe after a new path dependency | Cargo.lock needs no edit for task 9 | exit 0 | exit 101 |
| `sha256sum tests/m3/m2_operation_stream.json`; JSON keys | G-FROZEN after task 18 | another hash; a version field | `bcbf72c2...e370b`; `package` id only |
| `grep` of `wire_reason`, `debug_json`, `turts_debug`, port code, `CommandRejectReason::` | findings 5, 6; wave 1 additive 2a | no exhaustive match; no client match | `outbound.rs:55-90`, `export.rs:431`; no client match |
| `df -h`; disk audit | lane target dirs fit | under 100 GB free | 485 GB free; M3 used 433 GiB (audit:3) |

## Not checked

`unit_boot.rs:177` after migration (no-op replace; server build NOT RUN). PROJ1 headless adapter
(NOT TRACED, plan risk 5). Lightyear message size (NOT TRACED, task 2b step 1). Whether task 4
must edit `session.rs` tests (`session.rs:1195-1201` asserts `NotMobile`): NOT TRACED, it would
collide with task 7 in wave 4. Bevy `audio` feature system libraries on CI (ALSA): NOT TRACED.

## What I changed

`docs/research/mc/gate2-td.md` (this file, on the orchestrator's request); my memory file.

## Open questions for the orchestrator

1. Finding 2: rename `marker` and the muster order ids now (design amendment, as O1), or let task 19 rename by rule?
2. Finding 1: tests in `turts_server` with `#[path]` (no new boundary), or a library crate for the envelope mapping (new crate row and T1 row)?
