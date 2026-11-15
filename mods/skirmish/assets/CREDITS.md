# Skirmish art: sources and licences

All files are CC0 1.0 (public domain, https://creativecommons.org/publicdomain/zero/1.0/).
Credit is not required. We give it.

| File | Source | Author | Licence |
| --- | --- | --- | --- |
| `soldier.glb`, `soldier_anim.glb` | "Knight" body, KayKit Adventurers Character Pack 1.0, https://github.com/KayKit-Game-Assets/KayKit-Character-Pack-Adventures-1.0 ; crossbow from "Rogue" of the same pack | Kay Lousberg, www.kaylousberg.com | CC0 1.0 (`source/KayKit-LICENSE.txt`) |
| `worker.glb`, `worker_anim.glb` | "Rogue", the same pack | Kay Lousberg | CC0 1.0 |
| `barracks.glb` | "Barracks", https://poly.pizza/m/jjbNh1vCbj (from the pack "Ultimate Fantasy RTS") | Quaternius, https://quaternius.com | CC0 1.0 |
| `base.glb` | "Castle Fortress", https://poly.pizza/m/3r6gq3ow6O | Quaternius | CC0 1.0 |
| `trees.glb` | "Pine Trees", https://poly.pizza/m/oYtDty0fR6 | Quaternius | CC0 1.0 |
| `ground_grass.png` | Made for this project by a script (tiling noise) | TuRTS | CC0 1.0 |

Changes made to the models (script: `source/prep.py`, run with Blender in background mode):
scale to the game footprint, pivot at ground level, parts removed (unused weapons and shields),
one material renamed or split to `team` for the team colour, the people baked to one pose for
the static files. Downloaded 2026-11-13.

Stage P2 adds `soldier_anim.glb` and `worker_anim.glb` from the same supplied CC0 set.
The files retain their skeletons and authored clips. No model was regenerated.

The animated Crossbowman crossbow prop uses half its supplied local scale.
This keeps the carried weapon above ground in the Idle clip. The body scale remains 0.88.

## Swordsman (2026-11-14)

`swordsman.glb` and `swordsman_anim.glb` use the Knight body, 1H Sword, and Round Shield from KayKit Adventurers 1.0 by Kay Lousberg (CC0 1.0).
Source: https://github.com/KayKit-Game-Assets/KayKit-Character-Pack-Adventures-1.0/tree/672074b73ba276876a19e8816ecdc5241817ab47
Source file: `addons/kaykit_character_pack_adventures/Characters/gltf/Knight.glb`.
Source SHA-256: `60428e3abc09ba83e595d256e3af8c5c976b46cdae599f0802fc82b4a3445168`.
Changes: remove unused accessories, keep seven clips, scale to 1.7 units, shorten the sword prop to 60 percent, add a neutral team material, and export static and animated GLB files.
Reproduce with `blender -b -t 4 -P scripts/qa/animation/prepare_swordsman.py -- SOURCE_DIRECTORY mods/skirmish/assets/models`.
The source directory must contain `Knight.glb`. The original license is in `target/mc-evidence/swordsman/source/LICENSE.txt`.

## Selection portraits (2026-11-14)

`portraits/{base,barracks,worker,soldier,swordsman}.png` are static renders of the existing CC0 models listed above.
No source mesh or texture changes. The blue material identifies the portrait only; world models retain slot colours.
Render with `blender -b -t 4 -P scripts/art/render_skirmish_portraits.py -- mods/skirmish`.
The script uses an orthographic camera, transparent background, area lights, and 24 Cycles samples.
