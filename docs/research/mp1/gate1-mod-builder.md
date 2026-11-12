# MP1 gate 1: mod-builder lens

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=FAIL FINDINGS=12 HIGH=3 RUNS=9 NOT_CHECKED=5

Verdict for the mod-builder lens: a modder cannot write `frontier` from design rev 3 alone, and cannot write a second mod either. The design names mechanisms but gives no fields. The only field-level text is in the agent inputs, and those inputs conflict with the design. Spec status: D15 Baseline, D25/D28/D44/D55 Decided, S:335 Decided (readiness is not mandatory).

## High

**H1. The design has no public API for 0.2.4.** It gives no field names, types, defaults, export groups, validation or error texts. Lane 0 "adds every schema field ... first" (design:195), so the plan would freeze a shape that no gate has seen.
- I wrote `frontier` (scratch `mb/gen.py`). Today's packer refused 15 features, one at a time (`mb/stageB.txt`).
- I had to invent about 22 fields (`mb/invented-fields.md`).
- The inputs contradict the design:
    - Damage formula: GD:68 `max(1, floor(d×percent/100))` against design:149 `max(min, floor(d×ratio) - flat)`.
    - Where acquire goes: the weapon (design:150) or the unit (GD:277).
    - Power: `Network` plus `UnitDef.power` (GD:32, :270) against `Capacity`/`scope: Slot` with a shortage effect on each capability (design:142-143).
    - `Construct` (design:71) against `Place` (GD:273, UX:126).
    - `lanes` (GD:272) against named queues (design:250).
    - `ConsumeOverTime` (GD:271) against `ChargeOverTime` (design:157).
    - Relations: Enemy/Ally/Own (GD:278) against Ally/Enemy (design:173).
    - The home of the victory, disconnect and start-policy data is not stated. Today the map refuses `match`.
- Fix: add "6.10 Public mod API 0.2.4". Give one row per item: export group or file, field, type, default when absent, validation, error text, version-table row. Lane 0 builds that table. Gate 1 checks it.

**H2. Construction has two homes for the job.** design:132 says "Progress inside the producer's queue is refused". design:250, PRD1:378 and GD:141 say the structures queue of the command centre holds one site job. design:137 says construction is a second work capability. So I cannot write the command centre. Today `ProductionDef` refuses a building output ("output must be mobile and cannot be a producer").
- Fix text: "A constructor declares `constructor: Some((buildings: [ids], concurrent_sites: n))`. It is a work capability, exempt beside `producer`. The site holds progress, charge and job. The constructor counts only its active sites. MP1 progress is `Autonomous`. `NeedsBuilder(reach)` is the variant for a later worker mod."

**H3. NC1 cannot go red on the new content kinds, and the scan has a sharp edge for repo mods.**
- The scan derives tokens from a fixed list of kinds (`crates/turts_checks/tests/support/mod.rs:86-100` and the loops after it). A plant `"frontier:cannon"` or the queue id `"structures"` stays green.
- Bare local ids also match engine words. Probe: a repo package with the resource unit_name `"units"` gave 781 hits in 10 crates and test exit 101. design:263 names the engine source `Supply` "for the supply cap of a later mod". A later repo mod whose resource is named "Supply" will make the engine red.
- The scan is correct as a rule for engine source. Mods outside `mods/` are never scanned. For repo mods (MB) it is an undocumented limit on ids.
- Fix text for NC1: "The scan derives every 0.2.4 content kind. NC1 plants one id of each new kind, and each plant is red. A bare-id collision names the definition path. The repo package renames. The rule is written in the modder guide."

## Medium

**M4. The version-table presence test reads parsed values, not written fields.**
- Probe: `^0.2.2` with `orders: []` packs (exit 0). The 0.2.2 `Exports` is `deny_unknown_fields` and has no `orders` (git `740564c^`), so a 0.2.2 engine refuses that file. Same class in MP1: `armour: None`, `acquire: None`.
- Other features that are hard to detect:
    - Variants deep in trees: `Supply` inside `Format` arguments, `SupplyShort` in `visible`, the alert triggers, `Capacity`, `ChargeOverTime`, `Construct`/`SetRally`/`Repair`, `OwnDamaged`, `Ally`/`Enemy`.
    - The lifted rules and the work-capability exemption. These need combination tests, like the "weapon with movement" row.
    - Changed defaults with no field to detect: TGT2, idle acquisition, the OBS1 approach point.
- Fix text: "Presence means the field is written, read from the raw RON. VAL1 has one red package per row, including an explicit `None`/`[]`. A package with no 0.2.4 field keeps 0.2.3 behaviour, except the listed re-freeze items."

**M5. `Supply` is shaped to `frontier`.**
- PWR1 counts completed buildings only. The shortage effects are a rate or weapon off. A population cap needs units and queued jobs counted, and an effect "cannot start".
- The alert triggers `LowPower` and `NoCredits` (UX:123) use `frontier` words. A mod with two resources cannot tell which resource ran out.
- Fix: `counts: Completed | Queued` (MP1 builds `Completed`); shortage effect `Rate(ratio) | Disable`, with `BlockStart` reserved; triggers `SupplyShort(res)`, `SupplyRestored(res)`, `InsufficientFunds(res)`.

**M6. Weapon identity.** `Fired{weapon}` (design:191) names a weapon, but `WeaponDef` has no id and a unit has one `weapon` (`schema.rs:364`). A leash on each weapon makes no sense for a unit with two weapons.
- Fix: `acquire` goes on the unit; `Fired.weapon` is the weapon index; a missing table cell is a pack error.

**M7. The design gives no error text** (grep: 0 hits for "did you mean" or "ModError"). What today's packer does:
- `placements[0].unit: unknown unit` has no allowed list. A cost reference does list them.
- Weapon damage 0 packs.

Proposed messages:
- (a) `combat/damage.ron: cells: no cell for damage `frontier:rocket` and armour `frontier:structure``
- (b) existing: `missing field `footprint``
- (c) `units/x.ron: placement: no point of map `twin_fields` satisfies extends_area 10 and gap 2.0`
- (d) `victory.key_units[0]: unknown unit `frontier:comand_center`; did you mean `frontier:command_center`?`
- (e) `assets/assets.ron: models[2]: 9,412 triangles exceed the 3,000 vehicle cap`

**M8. Reason keys and cursor keys are not checked.** The typos `queue_ful` and `plcae` pack with exit 0. GHO1, SMT1 and OR22 depend on these keys.
- Fix: refuse an unknown key, with a "did you mean" hint, and add a version-table row for each new key.

**M9. The asset brief is not in the design.** design:285 says "the named nodes present" but does not name them. These facts are only in RE:118-128: forward +Z, the pivot, scale 0.8–1.2 × footprint (advice or check?), the triangle caps per class, and `turret`/`muzzle`. The shape of the `model` field is only at RE:17. The caps need a decision (RE:24).
- Fix: copy RE part 6 into section 8.1 as checks, each with its error text.

## Low

**L10. Placement gap: data or derived?** design:217 says "largest mobile diameter + 0.2", but GD says 0.5. A later mod with a larger unit gets silent traps. It is also not stated whether units count in the gap.
- Fix: a `placement.gap` number; the packer refuses a gap below the largest diameter + 0.2.

**L11. D28 is Decided.** The disconnect field (design:44) must mean no defeat when it is absent. State this.

**L12. The second mod hits limits that MP1 can accept, if the design records them as additive:**
- square footprints;
- batches;
- hold a point;
- two resources: "a tick charges all costs or none";
- free repair: a cost of 0 must be legal, and the cost for each HP is an integer ratio;
- whether `repairer` counts as a work capability.

## What I ran
| command | claim tested | refute | observed |
| --- | --- | --- | --- |
| `cargo build -p turts_pack` (scratch target) | – | – | exit 0 |
| turts-pack on today-schema `frontier` | GD: the content fits today's schema | refusal | exit 2 on Attack `fire`; after the fix, exit 0 (command centre without health) |
| 15 runs, one feature each | design:53: none of these exist | exit 0 | 15 × exit 2 |
| ^0.2.2 + `orders: []`; control | design:194: one row per feature | exit 2 | exit 0; control exit 2 |
| reason and cursor key typos | keys are checked | exit 2 | exit 0 |
| 10 mistakes | messages name file, field, hint | no file | file and field yes; hint only on field typos |
| scan + `frontier` stage A | design:99: ids are clean | fail | pass |
| scan + unit_name "units" | the scan constrains repo mod ids | pass | exit 101, 781 hits |
| `git show 740564c^` schema | 0.2.2 refuses `orders` | field present | absent, `deny_unknown_fields` |

## Not checked
The client and wasm model loading, the sound formats, the lobby protocol, the balance numbers, and the runtime behaviour of the probes (packer only).

## What I changed
Repository: none. `git status --porcelain` is empty. While my scan probe ran, I saw a foreign file `crates/turts_sim/tests/zz_pa_sim.rs`. It is not mine, and it was gone at the end. My memory: `.claude/agent-memory/mod-builder/mp1-design-gate1.md` and `MEMORY.md`.

## Open questions for the orchestrator
1. Must the model caps be decided before Lane 0? (needs decision; RE:24)
2. Does the NC1 bare-id rule stay for repo mods, or does it match qualified ids only?

Scratch files: /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mb/invented-fields.md, /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mb/stageB.txt, /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mb/scan_units.txt, /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mb/gen.py
