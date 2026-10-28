# Forge final local names

Task T5 ran E1 against the SPEC working names in a scratch copy. The scanner found exact engine
source token collisions for `inserter` and `range`. These two local ids use the SPEC suggested
names below. All other working ids remain unchanged.

| Content kind | SPEC working name | Final local id | E1 result |
| --- | --- | --- | --- |
| Item | `ore` | `ore` | No collision |
| Item | `shell` | `shell` | No collision |
| Node | `ore_vein` | `ore_vein` | No collision |
| Unit | `drill` | `drill` | No collision |
| Unit | `belt` | `belt` | No collision |
| Unit | `inserter` | `arm_loader` | Collision: `inserter` is an engine source identifier |
| Unit | `press` | `press` | No collision |
| Unit | `turret` | `turret` | No collision |
| Unit | `lab` | `lab` | No collision |
| Unit | `drone` | `drone` | No collision |
| Recipe | `make_shells` | `make_shells` | No collision |
| Technology | `hardened_shells` | `hardened_shells` | No collision |
| Map | `range` | `proving_ground` | Collision: `range` is an engine source identifier |
