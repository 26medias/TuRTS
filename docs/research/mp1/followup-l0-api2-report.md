RESULT=PASS

# Follow-up of L0-API2 (review `review-l0-api2.md`), branch `mp1/l0-api2`

Head `0f15148`, pushed to origin. The worktree is clean. Not merged into integration. Labels `l0api2-fu-*`; the
receipts are in `target/mc-evidence/l0api2-fu-*`. The scripts are in `scratchpad/l0api2-mine/`.

## Commits (on top of fa7984b)
- `4723ec1`: merge commit of `origin/mp1/integration` (f372c77: E-GPU). `L0-CORE` was not on origin at that
  moment, so it is not in the merge.
- `edf35a0`: the five stale `l0-raw.json` records re-pinned.
- `4906a2a`: H15 seats, K8 arm tests, L1 to L4 code, texts, errata.
- `0f15148`: plant records of this follow-up (11 new, 1 changed).
Each commit ends with the Co-Authored-By line.

## Items
| Item | Result | Evidence |
| --- | --- | --- |
| M1 plant records | done | `l0-raw.json` [4], [5], [8], [9], [10] re-pinned with the same fault each (`edit` form, anchor occurs once). A dry `prepare()` of all records (`scratchpad/l0api2-mine/dryprep.py`): 160 records / 5 refused before, 0 after (`l0api2-fu-dry1`, `-dry2`). The five are red and restored (`l0api2-fu-raw5`). `mp1_plants.py --all --list` exit 0 (`l0api2-fu-list2`). Note: `--list` never checks anchors, so it exited 0 before the fix too. Only the full run shows the refusal. **`--all` on the repository folder: 170 plants, 170 red-restored, 0 failed, exit 0** (`l0api2-fu-plants-all`, 101 command groups, tree clean after). |
| M2 K8 arms | done | Unit test `variants::tests::removal_with_a_queue_is_found_for_each_queue_capability` (producer, machine, research, constructor). Package test `k8_removal_with_a_queue_is_refused_at_0_2_3_for_each_queue` (producer on `cedar`; machine and research on `forge`; each at `^0.2.3`, exact table text). The constructor arm cannot be written at `^0.2.3` (the RAW row `constructor` refuses first), so only the unit test covers it. 5 plants: research arm removed (package test and unit test: red), constructor arm (unit test), machine arm (package), producer arm (package). All red and restored. |
| M3 H15 | done | The design has no seat count in the match file or the map file (`MatchDef` has 3 fields, 6.7.4). D:998 says "The seat count is the player-slot count (2 in MP1, `slots.rs:7`)". So the engine constant `SLOT_COUNT` moved to `crates/turts_mod_api/src/lib.rs` (`pub const`); `turts_sim::slots` re-exports it (`pub use`); `plugin.rs` is unchanged. Red first: the case "H15 the seats of the lobby, not the slots of the placements" (map with slot 0 only, one colour) gave "the package passes, a refusal was expected" (`l0api2-fu-h15-red2`, exit 101). Green after the change. Plants: the placement count back (red), `SLOT_COUNT` = 3 (red, text "needs 3 different slot colours"). Erratum E-API-9. The workspace builds and tests (turts_sim re-export). |
| L1 font hint | done | The font lookup uses `unknown(...)`. Test `h11_h13_h24_an_unknown_font_gets_the_hint_and_the_allowed_list` (H24 and H13 fonts, allowed list `mono, ui_font`, a font that exists passes). Plant: red. |
| L2 KEY1 (b) text | done | ``actions[4].default[0]: `KeyP` is also the default key of `key_produce`; each default key must be unique``. Tests `key1_b_unique_default_keys` and the CLI test updated. Plant: red. |
| L3 K2 limit | done | Test `k2_the_gap_limit_is_inclusive` (gap 2.0 passes, 1.99 refused with the design text). The reviewer's fault `<` to `<=` is a plant, red. |
| L4 errata and texts | done | E-API-10 (the four rules). O9 text ``units: a `Surrender` order must list no units; write `units: []` ``; K1 text with "; a production is a recipe or a building, not both"; U8 and M3 kinds in backticks. All three are in E-API-7 and in the code and tests. |
| Errata additions | done | `world.float_text` is owned by `L-FX-PRES`, `sound` stays with `FX2`. A1, A2 on the resolved set and the u64 overflow of `total` are written as an open plan item with no owner (E-API-8). |

## Gates on the merged head 4906a2a (code); 0f15148 adds only plant records
| Gate | Exit | Detail |
| --- | --- | --- |
| fmt | 0 | |
| clippy `--workspace --all-targets --locked -D warnings` | 0 | |
| tests of api, runtime, pack, checks | 0 | |
| `cargo test --workspace` | 0 | 1920 passed, 0 failed, 36 ignored |
| wasm32 check | 0 | |
| `boundary.sh` | 0 | |
| `mp1_plants.py --self-test` | 0 | |
| yardstick `--check`; completeness `--self-test` and `--enforce`; `driver_rule.py` self-test and `tests/browser` | 0 each | |
| F-PACK | ok | `acme`, `base`, `cedar`, `forge`, `haul` and `muster` each exit 0 |
| F-FROZEN | ok | sha256 `bcbf72c2…e08e370b` unchanged |
| `mp1_plants.py --all --list` | 0 | |
| `mp1_plants.py --all` | 0 | 170 of 170 red and restored |

## Notes
- Changed records and plants: the five `l0-raw` records, the changed `VAL1 H15` record, and 11 new records
  (K8 x5, M3 x2, L1, L2, L3), all in the repository folder run above.
- The `--all` run took about 50 minutes (Q-PREBUILD records are slow). My second run `l0api2-fu-plants-qpre`
  was refused by the lock of the first (`REFUSED another mp1_plants.py run holds ...`), because the first was still
  running. No harm: it ran no command.
- The plants folder rewrite of `l0-raw.json` is a full re-indent (same records, 5 re-pinned), so the diff of that
  file is large.

## NOT TRACED
- `L0-CORE` is not in the merge (not on origin when I fetched).
- The constructor arm of K8 at `^0.2.3` through the packer (not writable there; unit test only).
- Headed and browser rows: not run.
