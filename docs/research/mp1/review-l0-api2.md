VERDICT=PASS HIGH=0

# Review of L0-API2 (mod-builder, consumer lens)

Candidate: `fa7984b` (branch `mp1/l0-api2`, 9 commits on `9044820`). The worktree is
`/home/julien/Projects/TuRTS-mp1-l0-api1-review`. Date: 2026-11-12. Plan: task `L0-API2` (plan:664-678).
Design: 6.10, revision 6.2, the validation and error-text columns, and row VAL1 (D:2444). Builder errata:
`docs/research/mp1/errata-l0-api.md` (E-API-1 to E-API-8).

Counts: 7 findings (0 High, 3 Medium, 4 Low). 3 faults of my own (1 red, 2 green, which show test gaps).
47 sample packages through the validation column, and 12 sample packages through `turts-pack`.

## Summary

- Every sample package that breaks one validation row gets a refusal that names the package, the file and
  the field path, and says what is wrong. I wrote 47 one-fault packages, from 6.10, over all 8 groups (A, C,
  U and S, K and P, O, M, H, KEY1). Every one is refused.
- 20 of the refusals give the exact text of the design row. The other refusals have no design text.
- Each lookup of a reference, a key or a placeholder gives "did you mean" and the allowed list. Font lookups
  do not (L1).
- The lifted rules K7 and K8 stay refused for every edition. A `^0.2.3` package now gets the version-table
  text, and a `^0.2.4` package gets the `UNBUILT` text.
- All six repository packages pack with exit 0.
- No source line in the diff names a package id, so the default mod has no private shortcut.
- The task gate (F-RUST, F-QA, F-PACK, F-FROZEN) is green.
- The follow-up items of my L0-API1 review (M1, M2, M3, L1, L2, L3) are closed. I checked each one with a run.

## Findings

### Medium

**M1. The repository-wide plant run refuses 5 stale records, so F-REF cannot run.**
- `python3 -I -B scripts/qa/mp1_plants.py --all` (and `--files` on these paths) gives exit 2 with
  "5 record(s) refused, no command ran". The 5 records are `l0-raw.json[4]`, `[5]`, `[8]`, `[9]` and `[10]`,
  on the files `raw_presence.rs`, `version_table.rs`, `main.rs` and `archive.rs`.
- I ran `git apply --check` on the blobs:
    - Every record applies at `51f1223`.
    - `[9]` (`turts_pack/src/main.rs`) applies at `9044820` and is stale at `fa7984b`. This task broke it,
      because it inserted the help lines into the context of that patch.
    - `[4]`, `[5]`, `[8]` and `[10]` are already stale at `9044820` (L0-API1). My L0-API1 review did not
      see this.
- The builder ran its plants with a private `--plants-dir` that holds only `l0-api2.json`
  (receipt `target/mc-evidence/l0api2-plants-final/receipt.json`). So that run could not see the stale records.
- Fix: re-pin `[9]` in this task, and re-pin `[4]`, `[5]`, `[8]` and `[10]` on the integration branch. Then
  `--all` must give exit 0.

**M2. A K8 detector arm has no test that can fail, so a `^0.2.3` package can lose a refusal with no red test.**
- The old M3 rule refused `removal` on a unit with a producer, a machine or research.
- Now the old rule is gone, and the version row "removal with a queue" is the only guard for older editions
  (`crates/turts_mod_runtime/src/variants.rs:122-128`).
- My fault F2 removes the `research` arm. All 551 tests of `turts_mod_runtime` and `turts_pack` stay green.
- With that fault, the packer accepts a copy of `mods/forge` at `^0.2.3` whose `lab` has
  `removal: Some((by_command: true, contents: Destroy))`: exit 0.
- On the clean build, the same package is refused:
  `` forge/manifest.ron: engine_api: `removal with a queue` needs engine api 0.2.4; `^0.2.3` admits 0.2.3 (written in units/lab.ron: removal) ``.
- Fix: one package test for each arm (producer, machine, research, constructor) at `^0.2.3`, and a plant.

**M3. Row H15 counts the wrong slots. Ruling: count the lobby seats, not the map placements.**
- E-API-7 counts the distinct `owner_slot` values of the map placements (`mp1_validation.rs:1281-1303`).
- My probe: with a map that places units for slot 0 only, `AllSeated` with one colour passes.
- But the lobby has `SLOT_COUNT` = 2 seats (`crates/turts_sim/src/slots.rs:7`). So two seats can get one
  colour, which is the failure mode of row LOB2 ("two seats get one default colour").
- Ruling: the number is the number of lobby seats, which is the engine slot count of the match. If you want
  to keep the placement count, use the greater of the two values. Needs decision on where the packer reads
  the slot count, because `turts_mod_runtime` cannot depend on `turts_sim`.

### Low

**L1. A font lookup gives no hint and no allowed list.**
- The text is `world.unit_labels.font: unknown font asset `ui_fnt`` (`mp1_validation.rs:1573-1586`).
- A sound lookup gives both (`:87-96`, `:1542-1552`).
- Design 6.10 asks for the hint on every name lookup (D:1455-1459). "Font" is not in its list, but H11, H13
  and H24 say "font exists".
- Fix: use `self.unknown(..., "font asset", ...)`.

**L2. The KEY1 (b) text has no index in its path and no backticks.**
- Text: `val1/input/bindings.ron: actions: KeyM is the default of key_move and key_train; each default key must be unique`
  (`mp1_validation.rs:1389-1396`).
- Better: ``actions[7].default[0]: `KeyM` is also the default key of `key_move`; each default key must be unique``.

**L3. The K2 boundary is not tested.**
- My fault F3 changes `placement.gap < least` to `<=` (`mp1_validation.rs:646`). All 269 tests stay green.
- The design example (D:1536) makes 2.0 the limit for a tank of diameter 1.8. My probe shows that gap 2.0
  passes on the clean build.
- Fix: add the case "gap exactly 2.0 passes".

**L4. The builder made rules that are not in E-API-7. Each needs an erratum line.**
- `repairer.cost[].resource` must be a `Balance` resource (`:872-877`). R1 gives no resource kind.
- `weapon.damage_type` with no damage table is refused: "the package declares no damage table" (`:541-547`).
- `damage_types` and `armour_classes` entries must be a nonempty local id without `:` (`:434-440`).
- `KeyUnitsLost` with an empty list: "needs at least one unit" (`:1314-1319`). This row is in the design (M1).
- I accept all four rules. They must be in the design.

## Question 2: the lifted rules

- **K7 and K8.** `validate_m3` no longer has the two rules (`content_m3_validation.rs:373-375`, `:423-424`).
  The two COMBO rows (`version_table.rs:396-397`) run at `archive.rs:283`, before `validate_m3` (`:297`).
  So every edition below 0.2.4 is still refused (probes `k7_023`, `k8_research_023`). The edition 0.2.4 gets
  the `UNBUILT` line of `L-DES` (probe `k7_024`).
- **The two changed tests** (`tests/content_m3.rs:329-347`, `:562-581`). The requirement stays `^0.2.1`, and
  the package is still refused. Only the text changed, so no regression is hidden.
- **Consumer note on the new text.** The old text gave the fix that a 0.2.3 modder can use ("a unit with
  health requires removal"). The new text says "needs engine api 0.2.4". With 0.2.4, the modder then gets
  `UNBUILT` "leave it out". This is the form that VAL1 asks for, so it is not a finding. Option: add
  "(or add `removal`)" to the two rows.
- **K4** (`content.rs:142-149`, `:552-561`). The lift has no edition check. But `building_ids` is not empty
  only when a unit has `constructor`. The RAW row refuses `constructor` below 0.2.4 at `archive.rs:203`,
  before `validate_content_with_m3` (`:249`). Below 0.2.4 the old rule still applies, and my fault F1 shows it.
  The public `content::validate_content` has no manifest, so it applies the lift with no edition. Its only
  callers are tests (grep).
- **The validation column** has an edition check (`mp1_validation.rs:47`, `:69`). The cedar and muster
  pairs show this: `queue_ful` and `damage: 0` give exit 0 at `^0.2.3` and are refused at `^0.2.4`.
- **F-PACK:** all six repository packages give exit 0.

## Question 1: the design column against the code, row by row (gaps only)

- A1, "one damage table in the resolved package set", and A2, "the root Game package owns it". The code
  checks one package only (E-API-8). In MP1 a Game cannot have a dependency (rule LR3), so the result is the
  same today. No plan task owns the resolved-set check: `grep "resolved package set\|one damage table\|root Game"`
  on the plan finds nothing. This is an open question for MP2.
- P1 and design 6.4: a `total` that overflows u64 is not checked. The packer checks `Q` only (E-API-8). Accepted.
- V1 and V2 belong to `PRES-MODELS` (plan:668).
- O1, "the `content` of the target is a production id of `constructor.buildings`": this is a run-time fact
  of the order, with no pack-time field. Not a gap.
- The order targets of O1 to O4 and O9 are checked in `content_mc_validation.rs:1305-1320` (L0-API1). Not a gap.
- Every other row of the column has its check. Evidence: 47 probes and `tests/val1.rs` (33 tests).

## Question 3: the E-API-7 texts

| Item | Judgement |
| --- | --- |
| O9 | The check is right: the column says `units: []`, so a list with units is refused. The design text is ambiguous. Use: ``units: a `Surrender` order must list no units; write `units: []` `` |
| K1, a production in both lists | Accept. Add: "; a production is a recipe or a building, not both" |
| K6, no match file | Accept: ``manifest.ron: exports.match: a package with a constructor needs a match file that sets `open_sites_per_slot` `` |
| U8 and M3, the target kind | Accept the content. Put the kinds in backticks, as the convention says (D:1452): ``takes a `Point` target, not a `Node` target`` |
| "must be finite and at least 0, found x" and "must be positive" | Accept |
| H15 | Reject the placement count (M3) |
| KEY1 (b) | Use the text of L2 |

## Question 4: the owner of `world.float_text`

`L-FX-PRES`. The style is the colour, font and size of the "+N" float. Plan task `L-FX-PRES` draws that
float in stage 1 (plan:963; R14 at plan:141; design X14 at D:266). The `sound` stays with `FX2` (plan:509).
The builder's line (`unbuilt.rs:206-214`) agrees. The plan 5.3 table needs that line.

## Question 5: a private shortcut

None. In the diff of `crates/*/src`, a grep for every repository package id and for `manifest.id ==` finds nothing.

## Closure of the L0-API1 follow-up items (runs on this packer)

- **M1:** closed. `p02_acquire_024` gives ``base/units/rifleman.ron: acquire: `acquire` is not built in this engine build; leave it out until an engine build supports it``.
- **M2:** closed. The H11 fields are typed, and E-API-1 gives the list.
- **M3:** closed. `Bind(Sites)` gives a parse error, and the error lists the `Value` variants.
- **L1:** closed. Plants L1/a and L1/b are red in my plant sample.
- **L2:** closed. `turts-pack --help` has lines 20-26.
- **L3:** closed. `float_text` gives the `UNBUILT` text.

## My faults

| Fault | Change | Command | Result | Restore |
| --- | --- | --- | --- | --- |
| F1 | `content.rs:552`: `building = true` (K4 lifted for every production) | `cargo test -p turts_mod_runtime --no-fail-fast` | red, exit 101: `capability_and_recipe_matrix` (`tests/content.rs`) and `k4_a_building_production_may_have_a_static_output` | `cmp` 0, git clean, fresh green 269/0 (`l0api2-rev-faults-green`) |
| F2 | `variants.rs:124`: `research` arm removed | `cargo test -p turts_mod_runtime -p turts_pack` | green, 551/0, a gap (M2). The packer with the fault accepts the forge lab with removal at `^0.2.3` | `cmp` 0. After a rebuild, the packer refuses the package |
| F3 | `mp1_validation.rs:646`: `<` to `<=` | `cargo test -p turts_mod_runtime` | green, 269/0, a gap (L3) | `cmp` 0 |

## Plant sample

- `mp1_plants.py --files` (4 files) on the repository plants: exit 2. One stale record is refused (M1).
- `mp1_plants.py --all`: exit 2. Five records are refused.
- `mp1_plants.py --plants-dir <copy of l0-api2.json> --files` (5 files): 9 plants, 9 red and restored,
  0 failed. The 9 plants are K4, K7, K8, M1/a, L1/a, L1/b, L2/a, L2/b and L3/a.
- `mp1_plants.py --self-test`: exit 0.

## Commands run

All cargo commands ran through `exec_in_lane_v4.py --head fa7984b`. The labels are `l0api2-rev-*`.

| Label | Command | Exit |
| --- | --- | --- |
| fmt | `cargo fmt --all --check` | 0 |
| test-runtime | `cargo test -p turts_mod_runtime --locked --no-fail-fast` | 0 (269 passed, val1 33/33) |
| build-pack | `cargo build -p turts_pack --locked` | 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| test-dependents | `cargo test -p turts_mod_api -p turts_pack -p turts_server -p turts_checks -p turts_client --locked` | 0 (1042 passed) |
| frozen | F-FROZEN, the three tests | 0; the sha256 is `bcbf72c2…`, equal to the pin |
| boundary, qa-yard, qa-cself, qa-cenf, qa-dself, qa-drv, plants-selftest | F-RUST boundary and F-QA | 0 each |
| probe-samples, probe-h15 | a temporary test file `tests/zz_mb_probe.rs`, deleted after each run | 0 (the printed results are above) |
| f1-k4-wide-red, f2-k8-research, f3-gap-boundary, faults-green | the fault runs | 101, 0, 0, 0 |
| f2-restored-build-pack | rebuild on the clean tree | 0 |
| plants-sample, plants-all-check | `mp1_plants.py --files ...`, `--all` | 2, 2 (stale records) |
| plants-sample2 | `mp1_plants.py --plants-dir <l0-api2 copy> --files ...` | 0 |

I also ran `turts-pack` without the wrapper on 12 copies: the six repository packages, t2/t3/k7 pairs on
cedar and muster, the forge lab with removal, Bind(Sites), and the L0-API1 probes again. Each result is
quoted above. The worktree is clean after the review: `git status` is empty, and there is no `__pycache__`.

## NOT TRACED

- The order envelope check of O1, at run time.
- The resolved-set checks of A1 and A2 (no owner task).
- Whether the real pack path refuses a map with `owner_slot` 2 before H15. My 3-slot probe reached only `check_api_0_2_4`.
- The headed rows. They are out of scope.
- The 49 other builder plants that I did not run.
