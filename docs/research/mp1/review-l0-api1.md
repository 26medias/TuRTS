# Review of L0-API1 (mod-builder, consumer lens)

VERDICT=PASS FINDINGS=6 HIGH=0 RUNS=24 NOT_CHECKED=6

Candidate: `7c51ac0` in `/home/julien/Projects/TuRTS-mp1-l0-api1-review` (base `51f1223`). Date: 2026-11-12.
Reviewer: mod-builder. Design: revision 6.2, section 6.10. Plan: task `L0-API1` (plan:644-662), R2 (plan:129).

## Summary

A third-party modder can write a 0.2.4 manifest from the public types and section 6.10 for most rows. The
version table has one row for each 6.10 field row. Each row has `since_api` 0.2.4
(`crates/turts_mod_runtime/src/version_table.rs:203-447`). K7 and K8 are not in the table yet. This agrees
with plan:670, which gives them to `L0-API2`. The `UNBUILT` refusal names the package, the file and the
field path. I found no private shortcut for a default mod: no source line in the diff names a package id. The
R2 refactor keeps the behaviour of today. The fast gate is green. No finding is High. Two Medium findings
are design gaps that the code filled with its own shapes. Each one needs an erratum before `UI-PLACE`,
`UI-HUD` or a package author writes those fields.

## Findings

### Medium

M1. `crates/turts_mod_runtime/src/unbuilt.rs:270-277`. The refusal text is not the text of the design or
the plan. Design D:1394-1395 gives `` `<feature>` is not built in this engine build ``. Plan:658 puts "the
owner task in a comment". The code adds ``(task `L-TGT`)`` to the message that a modder reads. A third-party
modder cannot act on a plan task id. Also, the message does not tell the modder what to do. Reproduction:
`turts-pack` on a copy of `mods/base` with `engine_api: "^0.2.4"` and
`acquire: Some((range: 6.0, leash: 4.0))` on `units/rifleman.ron` (probe `p02_acquire_024`) prints
``base/units/rifleman.ron: acquire: `acquire` is not built in this engine build (task `L-TGT`)``, exit 2.
Fix: keep the owner in the line (code) and remove it from the message. Add an action, for example
`; leave it out, or use an engine build that has it`. Every test that asserts the text changes with the fix
(`unbuilt.rs:405-423`, `tests/api_0_2_4.rs:105-113`). Needs decision if the orchestrator wants the owner in
the text for lane debugging.

M2. Design D:1591 (row H11) and `crates/turts_mod_api/src/interface.rs` (the new `LineStyle`, `BarStyle` and
`FloatTextStyle` structs). The design gives fields for `placement` only. For `build_area_ring`, `gap_ring`,
`progress_bar`, `cargo_bar`, `rally` and `float_text` it gives only "colours, sizes, font id". The code makes
these shapes: `LineStyle {colour, width}`, `BarStyle {width, height, background, fill}` and
`FloatTextStyle {colour, font?, size, sound?}`. A modder cannot write them from the design. My guess
`build_area_ring: Some((colour: "#ffffff", size: 2))` (probe `p10_ring_guess_024`) uses the wrong field name
(`size`, not `width`). The `UNBUILT` line hides that error, so the modder sees it only after `UI-PLACE`
merges. No test parses a `Some` form of these six styles. `crates/turts_mod_api/tests/api_0_2_4.rs:560-578`
parses only `placement` and `unit_labels`. `tests/api_0_2_4.rs:518` in the runtime crate writes only
`float_text: None`. Fix: an erratum to H11 with the field list and one RON example for each style. Add a
parse test for each `Some` form.

M3. Design D:1587 (row H7) and `crates/turts_mod_api/src/interface.rs` (`Value`, `Source`). Needs decision.
The design lists the new variants of "`Value` and `Source`" together, and it does not say which enum gets
which variant. The code puts all 11 variants in `Value`. It also puts `ConstructionQueue`, `Sites`,
`Notices` and `SelectionTypes` in `Source`. So `Text(text: Bind(Sites))` parses, and no document says what
it shows. Options: (a) the four list names are `Source` only, so remove them from `Value`. (b) Keep them in
both enums, and the design defines the `Value` reading (for example a count). Evidence: design 7.5
(D:1901-1904) names the `Value::` variants of that list and none of the four.

### Low

L1. `crates/turts_client/src/relation.rs:28-46`. The client test cannot fail for a wrong `Ally` or `Enemy`
mapping. With `Teams::OWN_TEAM`, `Ally` equals `Same` and `Enemy` equals `Other`. My fault 2 maps
`Ally => Same` and `Enemy => Other`, and all 444 `turts_client` tests stay green. Today the behaviour is the
same. When the client gets a team table, a mapping fault in this second copy has no red test. Fix: test
`relation_of` directly, or add a test with a team table that puts slots 2 and 5 in one team.

L2. `crates/turts_pack/src/main.rs:20-37`. `turts-pack --help` lists the checks, but it does not list the
`UNBUILT` refusal or the edition rule. A modder who gets the refusal has no document for it. Fix: add one
line for each to the help text.

L3. Plan 5.3 (plan:509) and `unbuilt.rs:365`. `world.float_text` (without `sound`) has no `UNBUILT` line, and
no task in plan 5.3 owns it. A `^0.2.4` package with `float_text: Some((colour: "#ffffff", size: 14))` packs
with exit 0 (probe `p09_float_text_024`). Which task renders it is NOT TRACED. If no lane renders it, a
modder gets a field with no effect and no warning. Question for the orchestrator: which task owns the
`float_text` style?

## R2 (the five `SlotRelation` matches)

Closed. Each of the five sites now calls `turts_rules::relation` through one mapping:
`crates/turts_sim/src/orders/run.rs:466-469` (the plan's `orders.rs:1126` after S2),
`crates/turts_sim/src/economy/combat.rs:312-315`, `crates/turts_client/src/core_adapter.rs:344-348`,
`crates/turts_client/src/widgets/cursor.rs:431-432` and `:627-628`. A grep for `SlotRelation::(Other|Same|Any) =>`
finds only the two mappings (`crates/turts_sim/src/rules_adapter.rs:18-20`, `crates/turts_client/src/relation.rs:13-15`)
and `variants.rs:100`. There is no change of behaviour for the three old filters. `turts_rules/src/relation.rs:53-63`
gives Same = `a == b`, Other = `a != b` and Any = true, which are the old arms. The argument order
`(own, other)` does not change the result, because each relation is symmetric. The ratchet baseline moves
from 15 paths to 2 × 5 (`crates/turts_checks/tests/formula_ratchet.rs:39-60`). The plan's failure mode is
"two copies of the relation rule" (plan:129). The rule has one copy. The mapping has two copies, and the code
comment explains why (`rules_adapter.rs:1-8`: the client cannot depend on `turts_sim`). See L1 for the test gap.

## Errata judgement

- STATE_KEYS (23): the code is right, and the design is wrong. Design D:1622-1628 cites
  `crates/turts_net_protocol/src/economy.rs:90-108` and stops one line short. `GatherWaitReason::Blocked` is at
  `economy.rs:109` at the base `51f1223` (`git show 51f1223:...economy.rs`), and the simulation sends it
  (`crates/turts_sim/src/economy/gather.rs:66`). `crates/turts_client/tests/api_keys.rs` checks the list
  against the wire enums. The erratum must also correct plan:463 (`STATE_KEYS` (22)).
- H7: the design is ambiguous, and the code made a choice. Needs decision (M3).
- H11: the design is incomplete, and the code shapes are plausible because they copy `WorldHealthBarStyle`
  and `WorldTooltipStyle`. An erratum must list the fields (M2).
- H14: the code is right, and the design notation is wrong. `HintShownFor(id, secs: f32)` mixes a
  positional field and a named field, and RON cannot parse that form. Probe `p14_hint_024`, written as the
  design writes it, gives `base/interface/hud.ron: cannot parse: expected `,` at line 26 column 111`.
  The code form `HintShownFor(id: "h1", secs: 2.0)` parses (probe `p14b_hint_024` reaches the `UNBUILT`
  refusal of `hints`). The error-text example of the row also writes `HintDone(build_powr)` with no quotes.
  RON needs `HintDone("build_powr")`.

## My faults

Fault 1 (red). In `version_table.rs:52` I changed `" (written in {}: {})", self.file, self.path` to
`" (written in {})", self.file`. This is the field-path half of [mod L3]. Result:
`cargo test -p turts_mod_runtime --locked --no-fail-fast` gave exit 101. The `api_0_2_4` test failed 54 of
59, the lib failed 1 of 44, and one other binary failed 2 of 13. Example: `row armour ... left: "(written in f.ron)"
right: "(written in f.ron: armour)"`. I restored the file from a copy: `cmp` with `git show 7c51ac0:` gave
exit 0, `git diff` was empty, and `git status` was clean. The fresh run gave exit 0 with 0 failed. Logs:
`target/mc-evidence/l0api1-rev-fault1-red/`, `l0api1-rev-fault1-green/`.

Fault 2 (instrument probe; it stayed green, which gives L1). In `crates/turts_client/src/relation.rs` I
changed `Ally => Relation::Same` and `Enemy => Relation::Other`. Result: `cargo test -p turts_client --locked`
gave exit 0, 444 passed and 0 failed. I restored the file exactly (`cmp` exit 0, `git diff` empty). Log:
`l0api1-rev-fault2-client-relation/`.

Would the builder's tests fail, and on which build? `tests/api_0_2_4.rs` (runtime) compares the full text at
`^0.2.3` and at `^0.2.4` for each case. Fault 1 shows that a text change is red. The control test packs
`cedar` without a change at both requirements. Plants a to j cover row removal, the bump, `Written`, a line,
the edition gate, the presentation split, the sim mapping, the key count, the `*` walk and `rally`. The
client mapping (L1) and the `Some` style shapes (M2) have no red test.

## Sample mod (written from 6.10 only; copies of `mods/base` and `mods/muster`)

| Probe | Use | Result |
| --- | --- | --- |
| p00 | bump only, `^0.2.4` | exit 0 |
| p01, p03 | `acquire` (and `acquire: None`) at `^0.2.3` | `` base/manifest.ron: engine_api: `acquire` needs engine api 0.2.4; `^0.2.3` admits 0.2.3 (written in units/rifleman.ron: acquire) `` (the design text D:1464) |
| p12 | the same at `>=0.2.3` | the same, `` `>=0.2.3` admits 0.2.3 `` |
| p02, p04, p05, p10, p11, p14b, p08b-024 | `acquire`, `constructor`, `world.unit_labels`, `world.build_area_ring`, `exports.match`, `hints`, `kind: Supply` at `^0.2.4` | the `UNBUILT` text, with the file and the field path, exit 2 |
| p08b-023 | a new `Supply` resource at `^0.2.3` | `` (written in resources/power.ron: kind) `` |
| p13 | map `start_order` at `^0.2.3` | `` (written in maps/flat.ron: placements.start_order) `` |
| p15, p16 | `Button` `font: None` | the path is `root[hud_order_move_button].widget.font` (the widget id, not an index) |
| p17 | `float_text` with a typo `colr` | `` unknown field `colr` at line 24 column 134; did you mean `colour`? (allowed: colour, font, size, sound) `` |
| p19 | `slots: Enemy` in `muster` | `^0.2.3`: refused with the file; `^0.2.4`: exit 0 (built through `turts_rules`) |
| p06, p07 | `states` with a known key and with a typo, at `^0.2.4` | both exit 0 (T2 is `L0-API2`) |

Probe files: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/probes/`.

## Commands run (all cargo through `exec_in_lane_v4.py`, `--head 7c51ac0`, `MC_JOBS=8`)

| Label | Command | Exit |
| --- | --- | --- |
| l0api1-rev-test-api-runtime | `cargo test -p turts_mod_api -p turts_mod_runtime --locked` | 0 |
| l0api1-rev-fmt | `cargo fmt --all --check` | 0 |
| l0api1-rev-clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| l0api1-rev-test-workspace | `cargo test --workspace --locked --no-fail-fast` | 0 (1877 passed, 0 failed, 36 ignored) |
| l0api1-rev-wasm | `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked` | 0 |
| l0api1-rev-build-pack | `cargo build -p turts_pack --locked` | 0 |
| l0api1-rev-frozen | `cargo test -p turts_pack --locked --test m3_m2_stream --test forge_fixed_point --test forge_cases` | 0 |
| l0api1-rev-boundary | `bash scripts/boundary.sh` (MC_JOBS=4) | 0 |
| l0api1-rev-qa-* (5) | yardstick `--check`, completeness `--self-test`, `--enforce`, driver_rule `--self-test`, `tests/browser` | 0 each |
| l0api1-rev-fault1-red / -green | `cargo test -p turts_mod_runtime --locked --no-fail-fast` | 101 / 0 |
| l0api1-rev-fault2-client-relation | `cargo test -p turts_client --locked --no-fail-fast` | 0 (gap) |
| (no runner) | `sha256sum tests/m3/m2_operation_stream.json` | `bcbf72c2…e08e370b`, equal to the pin |
| (no runner) | `turts-pack` on copies of the 6 repository packages (F-PACK) | 0 each |
| (no runner) | `turts-pack` on 23 sample probes | see the table above |

The lane tree is clean after the review (`git status --porcelain` is empty).

## NOT TRACED

- The builder's own report (I did not get it). I judged the errata from the code and the design.
- Which task renders `world.float_text` (L3).
- The T1 to T4 edition behaviours (names only in this task; `edition.rs:18-29`).
- Headed and browser rows (out of scope).
- The `L0-EXIT` script of plan:513.
- A server join with a `^0.2.4` package.
