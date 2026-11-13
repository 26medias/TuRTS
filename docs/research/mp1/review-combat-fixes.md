# Review: mp1/combat-fixes at 315d5c74 (performance-architect, review mode)

VERDICT=PASS (HIGH=0, MEDIUM=0, LOW=3)

Probe sources: standoff.rs (all inputs), standoff_inmap.rs (in-map, non-degenerate), q3tests.rs, q3c.rs, in this folder.
Run logs: /home/julien/Projects/TuRTS/target/mc-evidence/combat-rev-*/command.log.

## Error bound (u = 2^-24, M = largest |component| of ring and target, r = range)
- Ring offset from target: |o| <= r(1+5u) + sqrt(2)uM (direction |d| = 1 +- 4u; product u; sum u per coordinate).
- Check passes when |o| <= r(1 - 2.5u) (subtraction u, squares+sum 2u on D, r*r rounding u).
- Pulled point shift = 2k u (r+M). Sufficient: k >= k_min = (8.5 + 1.414 M/r) / (2 (1 + M/r)).
- In map (coordinates >= 0, so M >= r/sqrt(2)): k_min <= 2.78. k = 4: worst margin 1.44 (near the origin corner); M >> r: margin 5.66.
- Coordinates < 0 (not reachable, clamp): k_min 3.32, margin 1.2.
- Degenerate: shift >= 2r when r < ~9.5e-7 M (k=4). Pulled point crosses the target and fails.
- Largest inward shift: 8u(r+M) = 4.77e-7 (r+M). Muster r=4, M=64: 3.2e-5. M=16384, r=8: 7.8e-3. Probe max 0.54 at M=2^20, r=1000.

## Search (27.8 M cases all inputs, 18.5 M in map)
k:        0.25     0.5    0.75  1  1.5  2  4
in-map:   1158070  7689   1     0  0    0  0
all:      k=4 fails 260559, all in the degenerate regime (r=0.001, M>=~2000), e.g.
t=(3713.1167,3960.168) from=(3713.114,3960.1707) r=0.001 -> p=(3713.1174,3960.1672) dsq=1.0728836e-6 > 1.0000001e-6.

## Findings
- LOW combat.rs:337-342: the comment says "four bounds keep a margin of four" (experiment). The worst-case margin by derivation is 1.44 in map. The rule holds by construction for k=4; the margin text is wrong. Fix: write the derivation in the comment; add debug_assert!(within_weapon_range(p, target, range)) after the pull-in.
- LOW combat.rs:323-342 + content_m3_validation.rs:33: a range below ~9.5e-7 x M (e.g. 1e-3 on a 4096 map) makes the pull-in cross the target; the unit never fires. Unreachable with the shipped packages (ranges 4.0, 8.0; maps <= 64). Option: clamp the shift to r/2, or the exact bounded form: step each non-zero offset component one ulp toward the target until the rule holds (the check is monotone in |offset|, so it ends in a few steps).
- LOW (pre-existing, trace only, NOT RUN) attack.rs:31-39, frame.rs:775-783: a weapon range below target radius + actor radius puts the ring point inside the target body; the arrival test finds it occupied. Not reachable today (4.0 > 2.0 + 0.5). Needs decision for X2 (centre-to-centre or edge-to-edge range).

## Question 3 runs (temporary tests, removed, cmp clean)
- Two attackers, one bearing: the first is pushed aside to d=3.9999998; the second stops at d=4.0. Both in range.
- Target radius 0.25 at x=0.3, attacker radius 0.5 along the edge (clamp active): fired at tick 43, d=3.9999886. The goal is recomputed each tick (attack.rs:87, 124-135) and converges on the clamp line.
- Static friendly unit on the ring point: fired at tick 56, attacker slid to d=4.
- Moving target: chase by recompute each tick, attack.rs:50-55, 87, 124-135 (trace).
