# The damage system: armour, damage types, and two new units

Date: 2026-11-14. Status: gate 1 passed with changes (simulation-core-engineer, 3 High, all
applied in this text). Ready to build after stages P2 and P3.
Track: Foundation (server damage rule, mod API contract).
Process: this spec, one gate, the build, one independent review that runs code. No plan document.
Base: the branch that holds P2 and P3 when they are complete.

## 1. Source and decision

- The CEO showed the team a video on armour formulas (Wyatt Cheng, "The Simple Math Hiding in
  Your Favorite Games"): flat armour (StarCraft), simple percent, and the ratio
  `Armour / (Armour + K)` (Warcraft III, World of Warcraft, League of Legends, Dota 2, Diablo).
- Three seats gave their view (game-designer, simulation-core-engineer, mod-builder).
- CEO, 2026-11-14: "I agree with the experts, go with what they recommend."

Thus:

1. The engine gets the damage system as one piece: a short, fixed list of steps that a mod
   switches on in data. The ratio is one of the steps. (mod-builder)
2. The default game uses the type table as its main rule, plus small flat armour. It does not
   use the ratio. (game-designer)
3. The work comes with the next combat units, after animation and effects, and before the
   mounted unit and other later units. (all three)

## 2. Facts, traced (branch `codex/models-stage1`)

- A damage rule exists as a library: `hit = max(minimum, floor(base x cell) - flat)`
  (`crates/turts_rules/src/damage.rs:1-50`). It has no production caller.
- The server applies plain damage: `health.min(damage + modifier)`
  (`crates/turts_sim/src/economy/combat.rs:485-490`).
- The packer refuses `exports.combat`, `armour` and `weapon.damage_type` (lane `L-DMG`,
  `crates/turts_mod_runtime/src/unbuilt.rs:64-66`). `acquire` is refused too (`L-TGT`, `:68`).
- Schema that exists: `ArmourDef { class, flat }` (`crates/turts_mod_api/src/schema.rs:446-450`),
  `CombatDef { damage_types, armour_classes, cells, minimum }` and `DamageCell`
  (`schema.rs:884-897`).
- Research can change weapon damage only: `EffectDef::WeaponModifier` with `WeaponStat::Damage`
  (`schema.rs:333-345`).
- The HUD has an armour field (`PrimaryField::Armour`), and its source returns nothing today
  (`crates/turts_client/src/widgets/sources/unit_hud.rs:17`).
- `Effect::Fired` carries no damage value (`crates/turts_net_protocol/src/records.rs:117-121`).
- No package under `mods/` has a combat file.
- NOT TRACED: how an idle unit selects a target today, and how far it chases. Codex traces it
  before section 6.3.

## 3. What a mod can build after this change

| Game rule | How |
| --- | --- |
| Command & Conquer: a percent table of warhead against armour type | step 2 only |
| StarCraft: flat armour for each hit, +1 for each upgrade, a type multiplier | steps 2 and 4, with an armour modifier |
| Age of Empires II: melee armour and pierce armour, bonus damage against classes that adds | steps 1 and 4 (`flat_by_type`), tags |
| Warcraft III: attack type against armour type, and the ratio on the armour value | steps 2 and 3 (`Ratio`) |
| Simple percent with a cap | step 3 (`Percent`) |

Out of scope (section 9): penetration chance, directional armour, negative armour.

## 4. The damage rule

One rule for all damage sources. Each step is optional. A package that sets none gets the
result of today (`hit = base`), bit-equal.

```
base   = weapon damage + weapon modifiers                              (checked add)
step 1 = base + sum of the weapon `bonus` rows that match the target   (checked add)
step 2, 3 = floor( step1 x cell_num x curve_num / (cell_den x curve_den) )   ONE floor
step 4 = step 2,3 - flat armour for this damage type                   (saturating)
step 5 = max(minimum, step 4)
applied = min(step 5, target health)
```

### 4.1 Step 1: bonus damage

- A weapon has `bonus: [(against: <class or tag>, add: u64)]`, default empty.
- A row matches when the target's armour `class` or one of its `tags` equals `against`.
- All rows that match are added.
- A unit has `armour.tags: [String]`, default empty. The combat file lists the valid tags.

### 4.2 Step 2: the type table

- As the schema of today: the cell of (weapon `damage_type`, target armour `class`) is a ratio.
- A pair with no cell uses 1/1.
- A target with no `armour` has no class: all cells 1/1, flat 0, points 0, no tag. `none` is
  not a reserved name.
- A weapon with no `damage_type` uses cell 1/1 against every class, and no `flat_by_type` row
  matches it; `flat` applies.
- Classes and tags share one name space in the combat file. A name is a class, a tag, or both.
- A construction site has the armour of the finished building.

### 4.3 Step 3: the curve

`CombatDef.curve`, default `None`. It uses the target's `armour.points` (new field, default 0).

| Curve | Factor on damage | Use |
| --- | --- | --- |
| `None` | 1 | the default game |
| `Ratio(k: Ratio)` | `k / (k + points)` | Warcraft III form: `k = 50/3` gives 6 % for each point at low armour |
| `Percent(per_point: Ratio, cap: Ratio)` | `1 - min(cap, per_point x points)` | simple percent; `cap` is required and is below 1 |

- `k` is a `Ratio`, because 1 / 0.06 is not an integer.
- Steps 2 and 3 are one multiplication and one division with one floor.
  - `Ratio`: `curve_num = k_num`, `curve_den = k_num + k_den x points`.
  - `Percent`: `curve_num / curve_den = (pd - pn x points) / pd`, or `(cd - cn) / cd` when
    `pn x points x cd >= cn x pd` (`per_point = pn/pd`, `cap = cn/cd`). This is exact.
- Overflow (gate 1, H2): `curve_den` and `cell_den x curve_den` use checked `u128`. Each sum
  (bonus, points, flat) is checked. On overflow the shot is rejected with `ArithmeticOverflow`,
  as today (`crates/turts_sim/src/economy/combat.rs`). The flat armour is subtracted from the
  wide value, and only then the result saturates to `u64` (`damage.rs:31-36`).
- Rounding is floor. It favours the target by less than 1 point for each hit.

Documented limit of whole numbers (simulation-core-engineer, probe): with `Ratio`, the
"hits to kill grow linearly" property holds only approximately. Life 1,000, hit 100, K 100,
armour 0, 100, 200, 300 gives 10, 20, 31, 40 hits (the ideal is 10, 20, 30, 40). A hit of 10
against armour 200 gives 334 hits (ideal 300). The documentation tells a mod author to use
damage values of 100 or more when the curve is in use.

### 4.4 Step 4: flat armour

- `armour.flat` (exists) applies to all damage types.
- `armour.flat_by_type: [(damage_type, u64)]` (new, default empty) replaces `flat` for the
  types that it lists.
- Modifiers: `Flat` adds to `flat` only; it does not change a type that `flat_by_type` lists.
  `FlatByType(t)` adds to the row of `t`; if the unit has no row for `t`, the row starts from
  the unit's `flat` at that moment and is then independent of `flat`.

### 4.5 Step 5: minimum

- `CombatDef.minimum` (exists) is the last floor. It applies with every curve.
- Thus at very high armour each hit does `minimum`. That is intended: no unit is immune by
  armour alone. A mod that wants immunity uses a cell of 0/1 and `minimum: 0`.

### 4.6 Armour modifiers

`EffectDef` gets a second variant:

```
ArmourModifier { unit: <unit id>, stat: Flat | Points | FlatByType(<damage_type>), add: u64 }
```

- It applies to all units of that type of the slot that owns the research, the units that
  exist and the units that come later. Same rule as `WeaponModifier`.
- The pack-time check refuses `Points` when the combat file has no curve, with a text that
  names the fix:
  `<package> units/knight.ron: armour.points is 5, but combat.ron has no curve. Add curve: Ratio(k: (50, 3)), or use armour.flat for a subtraction for each hit.`
- The same form of refusal for: a `damage_type`, class or tag that the combat file does not
  list (with a `did you mean` hint); a `Percent` curve with no `cap` or a `cap` of 1 or more;
  `k` of 0.

### 4.7 Where the rule runs

- The server calls the one rule in `turts_rules` for every hit. No second copy.
- All arithmetic is integer. The same world gives the same result on every run.
- The older packages (no combat file) keep their results and their frozen operation streams.
  If a stream changes, record the old value, the new value and the reason.
- Remove the `L-DMG` lines from the unbuilt list. Add version-table rows for each new field.
- The server catalog has no combat table today (gate 1, M4): add it, so that the rule can read
  the cells, the curve and the minimum at hit time.

### 4.9 Weapon range is measured to the edge of the target (gate 1, H1)

- Today the range is centre to centre, and a range below the sum of the two radii never fires
  (`crates/turts_sim/src/economy/combat.rs:306-312`, and the test at `:1028` that pins it, open
  item X2). A melee unit could then never hit a building: the Barracks radius is 1.5.
- New rule for all packages: a weapon can fire when
  `distance between centres <= range + target footprint radius`. This closes X2.
- A player sees: a melee unit walks up to the wall of a building and hits it; a ranged unit
  reaches a large building from a little further away than a small unit.
- The frozen operation streams of the older packages can change. Record each change with the
  old value, the new value and this reason.

### 4.8 What the player sees of it

- HUD: a selected unit shows its armour. With flat armour only: the number. With a curve: the
  points and the percent for a plain hit, for example `5 (23 %)`. With `flat_by_type`: one
  number for each type that differs.
- The value is correct for the units that the player owns, with their upgrades. For a unit of
  another player the value is correct, or it is absent. It is never wrong. Use the smallest
  wire change; state it and any protocol version change in the report.
- A hit that the rule reduced below `base` uses the `hit_reduced` effect of stage P3 when the
  visual has one. If this needs a wire flag on `Fired`, add it with the same report rule.
- The tooltip of a weapon shows its damage, its damage type and its bonus rows.

## 5. Documentation for mod authors

One page `docs/modding/damage.md`:

- The table of section 3 ("game rule -> fields").
- One worked hit with numbers for each step, and one with all steps.
- The rounding note of 4.3 and the meaning of `minimum`.
- The order of the steps, stated one time, with the example: base 10, cell 1/2, flat 3 gives
  `max(minimum, 5 - 3) = 2`.

## 6. Default game content (Proposed values; the game-designer tunes them)

Theme: medieval fantasy. The rule: the type table and small flat armour. No curve.

### 6.1 Two new units

| Unit | Role | Built at | Notes |
| --- | --- | --- | --- |
| Warrior | Armoured melee infantry. Slower to kill, must close the distance. Good against buildings and in a close fight. | Barracks | Weapon range about 1.2 map units. |
| Guard tower | A static defence that shoots arrows. Holds a place against light units. | Castle build menu, in the build area | Does not move. Needs no garrison. |

Art, CC0, same style as the units of today: the orchestrator supplies `warrior.glb`,
`warrior_anim.glb` and `tower.glb` in `target/mc-evidence/art/skirmish-fantasy/` with the
README and CREDITS rows.

### 6.2 Types, classes and the table

Damage types: `pierce` (bolts, tower arrows), `blade` (the Warrior).
Armour classes: `light` (Villager, Crossbowman), `heavy` (Warrior), `structure` (Castle,
Barracks, Guard tower). `minimum: 1`.

| Damage / armour | `light` | `heavy` | `structure` |
| --- | --- | --- | --- |
| `pierce` | 1/1 | 3/4 | 1/4 |
| `blade` | 1/1 | 1/1 | 3/4 |

Flat armour: Warrior 2. All others 0. A bolt (15) on a Warrior does floor(15 x 3/4) - 2 = 9.

Gate 1 (H3): with `pierce`/`heavy` at 1/2 and Warrior health 200, a simple model gave the
Warriors every duel with all of their cost left. The values above are the corrected start. The
model had no body collision and no leash; the real duel table of 6.4 decides.

What the player learns from it: Crossbowmen are good against Villagers and other Crossbowmen,
and poor against Warriors and buildings. Warriors beat Crossbowmen when they reach them and
are the unit that takes down buildings. A Guard tower stops light raids.

### 6.3 Statistics (starting values)

| Unit | Health | Damage | Type | Cooldown ms | Range | Speed | Cost | Train or build time |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Crossbowman (today) | 120 | 15 | pierce | 700 | 6.0 | 4.0 | 40 | 4 s |
| Warrior | 160 | 20 | blade | 900 | 1.2 | 3.6 | 60 | 6 s |
| Guard tower | 400 | 18 | pierce | 800 | 7.5 | 0 | 100 | 15 s |

Behaviour that the player expects (trace how targeting works today first; build what is
missing with the `acquire` field, lane `L-TGT`):

- Traced at gate 1: today an idle unit acquires a target only inside its weapon range, and the
  acquisition code of lane `L-TGT` is empty. Among targets at equal distance the lowest unit id
  wins (`combat.rs:450`). That is a tie-break for an exact tie only; keep it.
- An idle combat unit attacks an enemy that comes into its acquisition range.
- A Warrior walks to its target until it is in range. It stops the chase beyond a leash
  distance and returns. A Guard tower never moves.
- A melee unit that cannot reach its target because other units are in the way selects another
  target in reach. It does not stand still behind its friends.
- The computer opponent trains both unit types and builds one Guard tower at its Castle.

### 6.4 Balance check (recorded, with a pass rule)

A scripted, headless duel table, no player input, run 5 times with different placements (the
first row has equal total cost on each side; the other rows state their counts):

| Duel | Expected |
| --- | --- |
| 6 Crossbowmen against 4 Warriors, start 12 map units apart | neither side wins all 5 runs with more than half of its cost left |
| 3 Crossbowmen against 1 Guard tower | the tower wins |
| 5 Warriors against 1 Guard tower | the Warriors win |
| Time to destroy 1 Barracks: 8 Crossbowmen, and 8 Warriors | the Crossbowmen need at least 2 times the time of the Warriors |

If a row fails, the game-designer seat changes the values of 6.2 and 6.3, not the rule. The
final table and values go in the report.

## 7. Acceptance

Each row must be able to fail on the build before the change.

| Id | Check | Level |
| --- | --- | --- |
| DM1 | The rule, step by step: a table of at least 40 cases (each step alone, all steps, overflow edges, health below the hit) gives the expected whole numbers. The cases come from an independent calculation (not from the Rust code). It includes: base 15, cell 1/2, `Ratio` k 2/1, points 1 gives 5 (two floors would give 4); and an overflow case that is rejected. | rules test |
| DM2 | The probe values of 4.3: 10, 20, 31, 40 hits; hit 10 against armour 200 gives 334 hits. | rules test |
| DM3 | The server uses the rule: with a test table (`pierce`/`heavy` 1/2), a `pierce` hit of 15 on a `heavy` target with flat 2 removes 5 health (floor(15/2) = 7, minus 2). Before the change it removes 15. Also: one hit of each weapon of the default game against each class removes the value that the rule gives. | sim test |
| DM3b | Range to the edge (4.9): a weapon of range 1.2 hits a target of radius 1.5 from the distance 2.6 and not from 2.8. Before the change it never fires. | sim test |
| DM4 | Guard row (green before and after): a package with no combat file gives the damage of today for each hit. The frozen operation streams of the older packages change only through the range rule of 4.9; each changed stream is listed with its reason, and a stream with no weapon in it is equal. | existing tests |
| DM5 | `ArmourModifier`: after the research, a unit that exists and a unit trained later both have the new armour; a unit of another slot does not. | sim test |
| DM6 | Bonus damage: two rows that match add; a row that does not match adds 0. | sim test |
| DM7 | Pack: each refusal of 4.6 gives its text. The four example packages of section 3 (small fixtures) pack. | pack test |
| DM8 | Guard row (green before and after): the same battle, with the table, a curve and modifiers in use, run two times gives the same health of every unit at every tick. | sim test |
| DM9 | HUD: a selected Warrior shows armour 2; after a `+1` armour research fixture it shows 3. A unit with no armour shows no armour row or 0, as the HUD design says. | client test or browser |
| DM10 | Browser: the player trains a Warrior and builds a Guard tower with the mouse; the Warrior walks to an enemy and attacks it; the tower shoots an enemy in range. | browser |
| DM11 | The duel table of 6.4 passes and is recorded. | sim test, document |
| DM12 | The scripted player probes of `docs/qa/cold-play.md` pass, and one probe wins a match that has Warriors and a tower on the enemy side. | browser |
| DM13 | `docs/modding/damage.md` exists, and each worked number in it is checked by a test that reads the same numbers. | document, test |

Every test browser runs with `--alsa-output-device=null` and a fresh profile.

## 8. Review

One reviewer, after the build. The reviewer runs DM1 to DM12 and plants four faults: (a) the
flat step before the table step, (b) two floors in steps 2 and 3, (c) the server path that
skips the rule for one weapon, (d) the armour modifier applied to all slots. Each must turn a
row red.

## 9. Out of scope

Penetration chance and directional armour (Company of Heroes), negative armour, damage over
time and area damage (they will use the same rule with a `damage_type` when they arrive),
shields, healing, veterancy, an armour research building in the default game (the engine
supports the modifier; the default game gets its first upgrades in a later spec).

## Current-code corrections before the build (Codex, 2026-11-14)

The supplied facts describe the older models-stage1 branch. The implementation starts from the tested presentation integration.
API 0.2.11 already has explicit `WeaponRangeMode::{Centre, Surface}`. Surface includes both footprints and lets the existing Swordsman hit buildings.
Changing every older package to a different range would break this explicit contract.
Add `TargetSurface` for the target-edge rule of 4.9. Keep Centre and Surface unchanged.
Use TargetSurface for the new Warrior and the DM3b fixture. Existing frozen streams must remain equal.
The default game already has Swordsmen. Keep them as a separate unit. Include them in the damage table and browser checks.
The damage table is optional. Missing class, damage type, or cell uses the default stated in section 4.2. Remove old pack checks that demand a full table or armour on every health-bearing unit.
The CEO forbids a unit ID from deciding a visible result. For equal-distance target selection, use target coordinates before any internal identity.
One independent review runs the acceptance checks after the build. No planted-fault registry, receipt ledger, new gate, or second review pass is required.
