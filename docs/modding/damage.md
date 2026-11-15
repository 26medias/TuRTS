# Damage and armour

API 0.2.12. The server decides damage. Models and effects do not change the result.
A game exports at most one combat file through `exports.combat`.
A package with no combat file keeps its old damage rule and old range modes.

| Game rule | Fields |
| --- | --- |
| Command & Conquer: warhead against armour type | `damage_types`, `armour_classes`, and `cells` |
| StarCraft: type multiplier and flat upgrades | `cells`, `armour.flat`, and `ArmourModifier(Flat)` |
| Age of Empires II: melee and pierce armour, additive class bonuses | `armour.flat_by_type`, `armour.tags`, and `weapon.bonus` |
| Warcraft III: attack types and an armour curve | `cells`, `curve: Ratio`, and `armour.points` |
| Capped percentage reduction | `curve: Percent` and `armour.points` |

The package tests generate small examples of these five styles.
Run `TURTS_DAMAGE_FIXTURE_DIR=OUTPUT cargo test -p turts_mod_runtime --test damage damage_four_game_styles`.
Each output folder contains a complete package that the packer can check.

## One order of operations

1. Add the weapon modifiers to weapon damage. Reject overflow.
2. Add every bonus row that matches the target's class or a tag. Reject overflow.
3. Multiply by the type cell and the armour curve. Divide once. Round down once.
4. Subtract flat armour from this wide value. Stop at zero. Then limit the result to `u64`.
5. Apply `minimum`, then limit the applied damage to the target's current health.

For example, base 10, cell 1/2, and flat armour 3 give `max(minimum, 5 - 3) = 2` when minimum is 1.
A base of 15, cell 1/2, curve K 2, and one armour point give 5.
Two separate rounding steps would incorrectly give 4.

For an example with all steps, weapon 20 and modifier 5 give base 25.
Matching bonuses 3 and 7 give 35. Use cell 3/4, K 50/3, and five armour points.
The curve factor is 50/65. The combined product rounds down to 20.
Flat armour 2 gives a hit of 18. A target with 12 health loses 12.

Each table row below is an executable worked example. The documentation test reads these same inputs and expected numbers.
`Input` means weapon damage plus its modifier. `Bonus` contains only matching rows.
The ratios in this table use short mathematical notation.

<!-- damage-examples:start -->
| Case | Input | Bonus | Cell | Curve | Points | Flat | Minimum | Health | Hit | Applied |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| unchanged | 10+0 | 0 | 1/1 | None | 0 | 0 | 0 | 100 | 10 | 10 |
| modifier | 10+2 | 0 | 1/1 | None | 0 | 0 | 0 | 100 | 12 | 12 |
| bonuses | 10+0 | 3+7 | 1/1 | None | 0 | 0 | 0 | 100 | 20 | 20 |
| table | 10+0 | 0 | 1/2 | None | 0 | 0 | 0 | 100 | 5 | 5 |
| ratio | 100+0 | 0 | 1/1 | Ratio(100/1) | 100 | 0 | 1 | 1000 | 50 | 50 |
| percent | 100+0 | 0 | 1/1 | Percent(1/20,3/4) | 5 | 0 | 1 | 1000 | 75 | 75 |
| cap | 100+0 | 0 | 1/1 | Percent(1/20,3/4) | 100 | 0 | 1 | 1000 | 25 | 25 |
| single floor | 15+0 | 0 | 1/2 | Ratio(2/1) | 1 | 0 | 0 | 100 | 5 | 5 |
| flat | 10+0 | 0 | 1/1 | None | 0 | 3 | 0 | 100 | 7 | 7 |
| table then flat | 10+0 | 0 | 1/2 | None | 0 | 3 | 1 | 100 | 2 | 2 |
| minimum | 1+0 | 0 | 0/1 | None | 0 | 99 | 1 | 100 | 1 | 1 |
| immunity | 10+0 | 0 | 0/1 | None | 0 | 0 | 0 | 100 | 0 | 0 |
| all steps | 20+5 | 3+7 | 3/4 | Ratio(50/3) | 5 | 2 | 1 | 12 | 18 | 12 |
| checked base | 18446744073709551615+1 | 0 | 1/1 | None | 0 | 0 | 0 | 100 | overflow | overflow |
<!-- damage-examples:end -->

A missing cell uses 1/1. A target with no armour has no class, points, flat value, or tags.
An untyped weapon uses 1/1 and the target's general flat value.
Classes and tags share a name space. A bonus row matches once, even if its name matches both.
Different matching rows add together. A construction site uses its finished building's armour.

## Armour curves

`None` applies no curve. `Ratio` multiplies by `k / (k + points)`.
For example, write `curve: Ratio(k: (numerator: 50, denominator: 3))`.
`Percent` multiplies by `1 - min(cap, per_point * points)`.
Write both ratios. For example:

```ron
curve: Percent(
    per_point: (numerator: 1, denominator: 20),
    cap: (numerator: 3, denominator: 4),
),
```

K must be positive. Every ratio denominator must be positive. Cap must be below one.
The packer refuses points and Points upgrades when there is no curve. It names the fix.
The calculation checks bonus sums, armour sums, and curve denominators for overflow.
An overflowing hit consumes no ammunition and removes no health.

Whole-number damage makes the ratio curve approximate. For life 1,000 and K 100:

<!-- damage-kills:start -->
| Hit before armour | Armour points | Hits to kill |
| --- | --- | --- |
| 100 | 0 | 10 |
| 100 | 100 | 20 |
| 100 | 200 | 31 |
| 100 | 300 | 40 |
| 10 | 200 | 334 |
<!-- damage-kills:end -->

Use damage values of 100 or more when a curve needs a close approximation.
`minimum` prevents armour from making a unit immune. For immunity, use cell 0/1 and minimum 0.

## Upgrades and display

`ArmourModifier` affects existing and future units of its owning slot.
`Flat` changes general armour. `Points` changes curve points.
`FlatByType("game:pierce")` changes only that type's row.
A new row starts from the general flat value at that moment. Later Flat upgrades leave that row unchanged.
Technologies apply in completion order. Effects inside one technology apply in their declared order.

The owner-only research record carries this armour order. The HUD and server share one armour calculation.
The HUD shows a flat value, or points and the curve reduction, plus differing type values.
It hides another player's armour because that player's upgrades are private.
`SelectionPrimary(WeaponDetails)` shows own effective weapon damage, its type, and its bonus rows.
A hit below the modified base damage uses `hit_reduced` when the visual declares it.
Protocol 7 adds the reduced-hit flag and the owner-only armour order. Older peers cannot join.

## Range and automatic engagement

`Centre` keeps the old centre-distance rule. `Surface` includes both footprints.
`TargetSurface` adds only the target footprint to the authored range.
Choose a mode in the package. Loading an older package does not change its mode.

`acquire.range` sets the idle acquisition distance, using the weapon's range mode.
`acquire.leash` limits the unit's travel from the position where it starts an automatic engagement.
Zero permits firing in place. A unit returns when its target leaves the allowed chase area.
An explicit player order takes priority. Static weapons never chase.
A blocked automatic chase selects another enemy already in reach.
Coordinates resolve target ties and sequential firing priority. Unit IDs do not grant priority.
