# Swordsman and surface range

Status: implementation in progress. This is a small foundation change plus skirmish content.

## Player result

Barracks offer Crossbowmen and Swordsmen. Swordsmen have a sword and shield, more health, and a short attack reach. They can hit a castle from outside its footprint. Mixed armies keep their separate weapon reaches. Existing weapons keep their current behavior.

Swordsman starting balance: 300 health, speed 4.5, damage 25, cooldown 800 ms, footprint radius 0.6, attack gap 0.6, cost 80 credits, training 6 seconds. Extra health represents protection. There is no new damage reduction rule.

## Engine contract

API 0.2.8 adds `weapon.range_mode: Centre | Surface`. Omission means Centre. Writing either value requires a manifest that excludes engines before 0.2.8. Resolution retains the mode. Existing RON files and Centre weapons retain their center-distance rule.

Surface reach in center coordinates is `range + source_radius + target_radius`, in that order. The sum uses f32, capped at the largest finite f32. Radii come from the authoritative live unit types. This same reach controls firing, attack approach, and attack-move engagement. Broad-phase lookup includes the largest live target radius. Exact filtering uses each candidate's radius. Target priority remains unchanged.

The attack goal uses the existing bounded rounding correction, so the goal passes the firing rule. Movement collision rules and first-arrival destination ownership stay unchanged.

A selected Surface weapon draws its ring at `range + source_radius`. A target footprint can intersect that ring to be in reach.

The audit catalog includes an optional range mode, omitted for Centre. Old audit records default to Centre. Audit attack approach and engagement use the same geometry. The audit also reconstructs the existing bounded approach rounding correction; its old ring formula could reject a legal goal. Game command and snapshot wire messages do not change. High and Low effects cannot change combat state.

## Acceptance

| Row | Check |
| --- | --- |
| SW1 | Old weapon parses as Centre. Explicit mode requires API 0.2.8. Both variants resolve. |
| SW2 | Surface weapon fires at a large building outside both footprints. It cannot fire beyond the authored gap. Centre behavior stays unchanged. |
| SW3 | Attack and attack-move reach a large building and deal damage. Movement has no overlap. |
| SW4 | A mixed swordsman and crossbow group attacks. Both deal damage at their separate reaches. |
| SW5 | Barracks show both named costs. Training charges 80 credits and creates a swordsman after 6 seconds. Rally works. |
| SW6 | Browser shows sword, shield, team color, walking, attack, hit, and death. Sword attacks create no bolt. |
| SW7 | Audit replay accepts Surface attack and attack-move. Repeated identical commands give identical state. |
| SW8 | Detailed High rendering at 400 mixed fighting units stays at least 30 FPS. Record raw evidence and settings. Fix performance failures before delivery. |
| SW9 | Scripted cold player probes pass on the final build. The launcher starts that build with sound off. |

One independent reviewer must run the foundation code after implementation. Fix findings before delivery. Record evidence and unverified limits in the delivery report. Do not merge or push main.
