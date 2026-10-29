# Gate 1 rigour evidence (MC design)

Probes: deps.py (R4 closure of done/later rows), gm40.py (GM40 geometry), ste.txt (long and passive sentences), copy/ (scratch repo copy for the planted run).

## Planted run (done -> later for PF01, CB03, MP09), scratch copy, completeness.py --enforce
RED R4 CA13 (MP09); OR01 (PF01); OR04 (CB03); OR08 (CB03); PF04 (PF01). exit=1. Baseline copy exit=0.

## Done rows vs cited files
- PF01 Sensible path: movement.rs tests are straight line only (unit_moves_speed_per_second_and_arrives); yardstick dep MP01 = later:M4. NOT PROVEN.
- CB03 Auto-target: forge_cases c13/c14/c16 turret only; gap-game-designer.md:84 says "partial"; dep FV03 = later:M5. NOT PROVEN for mobile units (K5 adds them).
- MP09 Playable bounds: command.rs:243 move_to_nan_and_outside_map_rejected asserts refusal; K6(a) and GMU reverse it. Dep MP01 later:M4.
- MD02 Weapon and projectile definition: no "projectile" in schema.rs or schema_m3.rs. Narrow.
- MD02, MD05, EC01 done while dep MD01 is `in` at MC; EC02 done while dep OR17 is `in` at MC.
- Plausible: PR01, PR02, PR03, PR07 (forge_cases.rs:1259 PrerequisitesMissing), PR08, PR15, EC04, EC10, CB01, CB02, CB21, UN13, MD18, AN08.
- Tool gap: rule4 (completeness.py:136-153) checks deps of `in` rows only; a `done:` row with a later dep passes.

## Later rows sampled
GM07, GM11 (M4b, S:725), OR26, CB05 (MP1, design 383), AU03, AU07 (MP1, 716), MF01 (M8, S:732), SV01 (M10, S:734), CA15 (M4): consistent. PC16 later:M5 but built and tested by SEL1 (322, 924, K16). UI05/CA08/CA09/UI08: author F-G2.

## GM40 geometry (r=0.5, s=1.05)
Separation range r_i+r_j+0.1 = 1.1 > slot spacing 1.05. Worst centre gap with 0.05 arrival tolerance: -0.05 (overlap). Axis-aligned screen rects of neighbours at exact slots overlap: 32 to 65 pairs (pitch 45/60, yaw 0/15/30). Max slot distance 3.637 < R_near 4.28 (OK).

## Budget arithmetic
Task rows sum 97 over 22 tasks; 97+14+15 = 126. Matches 990-993.
