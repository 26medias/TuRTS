# MP1 design revision 4 re-gate: mod-builder

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=1

**Summary.** Revision 4 closes 9 of my 12 gate-1 findings with real changes. Three are partial. A third-party modder can write most of a second skirmish mod (`tide`) from sections 5, 6.10, 6.7.4 and 7. I had to guess 14 items, and 3 rules of section 7 have no public path. Design lines are `D:<line>` in `docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md` (5158f96). Status: D15 Baseline; D28, D25 Decided; D-I, D-L, D-M, N1 are CEO items; D70 Proposed.

## My gate-1 findings
- H1 (no API table): CLOSED. The table is at D:1256-1446, one row per item. The guesses that remain are under L2.
- H2 (two homes for construction): CLOSED. D:373-406 and row K1.
- H3 (NC1 kinds): CLOSED. D:1275-1276 and NC1 at D:2027. The bare-id rule is CEO item N1 (D:166): needs decision.
- M4 (presence check): PARTIAL. RAW is defined (D:1280-1297), but it fails on fields inside enum variants (M1 below).
- M5 (Supply shaped to frontier): CLOSED. D:474-500, rows S1, U5, U6, H4.
- M6 (weapon identity): CLOSED. D:529-531 and row C3.
- M7 (error texts): PARTIAL. The texts are given, but they drift from today's form (L3).
- M8 (reason and cursor keys): CLOSED in the design. Rows H1, H2, T2. The packer still accepts `queue_ful` (run p5).
- M9 (asset brief): PARTIAL. The caps wait for CEO decision D-M (D:140). Orientation is advice only (D:1410).
- L10 (placement gap): CLOSED. D:291, D:428-430, row K2.
- L11 (D28 when absent): CLOSED. D:1368 and D:774.
- L12 (second-mod limits): CLOSED. D:1444-1446; R1 allows free repair.

## Guesses for the sample mod (`tide`: diver, lancer, skimmer, dock, spire, a match file, an interface file)
1. Does `CombatDef` have an `id` or a `name`? C1-C4 list four fields only.
2. Are fields inside new structs required? This applies to `constructor.progress`, `constructor.when_short`, `placement.gap`, `node_clearance`, `in_build_area`, and the hint fields `starts` and `done`. D:1272 covers only new fields of existing structs.
3. Is `AllSeatedReady` written `AllSeatedReady(3)` or `AllSeatedReady(countdown_secs: 3)`?
4. Is `Rate` written `Rate((numerator:..,denominator:..))`?
5. What does `site_health` mean? D:1334 is its only mention.
6. In `Construct{content}`, is the content the production id or the unit id? Does `BuildMenu` list the constructor entries?
7. Which key shows in the build menu: the order `hotkey` or the production `hotkey`?
8. A site has `queued` and `output_blocked` refunds that mean nothing, but the fields are required.
9. What is the RON form of `FocusAlert`? Is `Input("leave_match")` an engine action, or must the input map declare it?
10. How does a mod build the confirm dialog for Surrender (D:1655)? I found no widget or predicate for it.
11. The full list of reason keys and the `activity.<name>` state keys are defined only by engine source (D:1379, D:1381).
12. What are the top-level shape of the effects file and the types of `fire` and `hit` (V3, V4)?
13. Can a `Supply` resource be seeded in `slot_economy`?
14. Is a `period_ms` of 1050 legal? Its legality depends on the tick rate (M5 below).

## New findings

**H1 (High). The engine has two outputs in frontier words, with no mod field.**
- The "Place" column of the alerts names frontier units: "an offline turret, else the Command Centre", and "the Command Centre" for `SupplyShort`, `SupplyRestored` and `InsufficientFunds` (D:1714-1716). `AlertDef` has no place field (6.10 rows H4-H6, H17).
- The statistic "credits gathered" (D:1001, D:1650) names no resource. `MatchStats` has no argument (H7).
- This contradicts D:360 ("No private path") and D:497. My Dock mod has no public way to say where its alert points or which balance counts.
- Repair: add `alerts[].focus: Option<[ContentRef]>`, defined as "the first own unit of these types; else no camera move". Add "gathered: one count for each `Balance` resource; `MatchStats(resource)`".

**M1. RAW detection cannot find `Button.font` and `Button.size` (row H13).**
- Probe: `ron::Value` drops variant names, and it gives struct fields and map keys the same `Map(String)` form.
- `Text` already writes `font` and `size` in each hud.ron (forge: 23 `font:` keys, muster: 6). A key walk either refuses forge and muster, or misses the field.
- Repair: "A RAW row names a path from the root of a file of one export group. A field inside an enum variant is detected on the parsed value (H13: a `Button` with `font` or `size` Some)."

**M2. `kind: State` excludes forge's triggers.**
- D:1704-1705 gives forge's 1 s rules as the reason for State alerts. Row H6 (D:1384) allows State only for frontier's triggers.
- forge's triggers `OutOfAmmo`, `OutputBlocked`, `InserterWaiting`, `ResearchReady` (`mods/forge/interface/hud.ron:130-134`) cannot use State.
- Repair: allow State for `InserterWaiting`, `OutOfAmmo`, `OutputBlocked` and `ExtractorFull`.

**M3. No public path for the result reason lines of D:1644-1647.**
- Row H10 has only `OutcomeIs`. `OutcomeReason` (H7) gives a code, not text. There is no `{slot}` value outside alerts.
- Repair: add `Predicate::OutcomeReasonIs(reason)` and a `{slot}` placeholder for the other slot in `Format`.

**M4. Key lists point at engine source.**
- `REASON_KEYS` points at `messages.rs as_str`. `STATE_KEYS` points at "each wire `Activity`" (D:1379, D:1381).
- Repair: list every key in 6.10.

**M5. `income.period_ms` must be "a whole number of ticks" (U7), but the packer cannot check this.**
- The packer uses `DEFAULT_TICK_HZ` (`crates/turts_pack/src/lib.rs:25-27`). The server reads `tick_hz` from server.ron (`crates/turts_server/src/config.rs:77`).
- `try_ms_to_ticks` rounds up (`crates/turts_sim/src/boot_m3.rs:591-599`).
- Repair: "`period_ms` above 0; converted by `try_ms_to_ticks`; the packer makes no tick check."

**L1.** `cancel_refunds.destroyed` is also added to `TechnologyDef` (`crates/turts_mod_api/src/schema.rs:312`), and K8 uses it for research. Row K5 lists production only.

**L2.** Guesses 1-10 and 12-13 above. Repair: one default and one RON example for each new struct.

**L3. Error texts drift from today's form.**
- Row V1 has no package and no `models[i]` field path. Row V3 has no list and no hint.
- 6.10 drops the backticks of today (runs p1, p7, p8) and of 6.7.4.
- O8 says "unknown action". Today the text is "unknown input action".
- The examples leave out the `(allowed: ...)` that D:1268 requires.
- A version-row error names `manifest.ron: engine_api`, not the file of the field.

**L4. Policy that the engine holds, recorded and not blocking.**
- `HintCondition` has no balance condition and no selection condition.
- Retaliation comes with `acquire` and cannot be turned off.
- Fixed constants: the 1/2 threshold of `hit_reduced`, the 15 ring radius, Shift queues 5, the 1 s hold.

## Existing packages and the version table
- No 6.10 row renames an existing field. Each new field is optional. All repo packages are `^0.2.3` (`mods/*/manifest.ron`), so the ED rows do not change them.
- forge and muster pack with exit 0 today.
- The only break risk is M1.
- The version rows fit `VersionTableRow{name, since_api}` (`crates/turts_mod_runtime/src/version_table.rs:7-57`). The bump of `RELEASED_ENGINE_APIS` to 0.2.4 is NOT TRACED in the design.

## Error format, 10 rows checked
- Package, file and field are named: A1, A2, C3 (first text), U1, U4, K1, K2, M3, H1.
- V1 does not name them.
- Present form: `muster/maps/field.ron: placements[0].unit: unknown unit `muster:troper``.

## What I ran
| Command | Claim tested | Would refute | Observed |
| --- | --- | --- | --- |
| `cargo build -p turts_pack --locked -j 6` (separate target dir) | none | none | exit 0 |
| turts-pack on copies of forge and muster | existing packages pack today | exit 2 | exit 0, exit 0 |
| 9 probes on muster | today's error form (D:1266) and M7/M8 | other form | p1, p2, p3, p6, p7, p8, p9: exit 2 with the form `pkg/file: field: msg`; p4 (damage 0) and p5 (reason typo): exit 0 |
| `ron::Value` probe, ron 0.12.2 | RAW finds fields (D:1280-1282) | variant name or path is kept | names dropped; struct fields and map keys look the same; exit 0 |
| `xref.py` | sections 5 and 7 names are defined in 6.10 | 0 missing | `FocusAlert`, `leave_match`, the outcome reason codes and `MatchResult` are missing; exit 0 |
| RON syntax check of the 18 `tide` files | the sample is well-formed RON | a parse failure | 18 ok, exit 0 |
| greps of hud.ron, the work-capability code and the design | M1, M2, H1 | no hit | as cited above |

NOT TRACED: client behaviour, the asset checks, the plan.

## What I changed
- Repository: none. Another agent's file `docs/research/mp1/regate-player.md` is untracked in the tree; it is not mine.
- My memory: `.claude/agent-memory/mod-builder/mp1-design-gate1.md` and `MEMORY.md`.

The sample mod `tide` and the probe tools (`xref.py`, `ronprobe/`, `runs/`) are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-mod/
