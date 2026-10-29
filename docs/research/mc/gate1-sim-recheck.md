VERDICT=pass-with-changes FINDINGS=6 HIGH=1 RUNS=8 NOT_CHECKED=4

Rev 2. Probes: /tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/recheck/

## Original findings

1. Fixed with new defects N1, N4 (design:533-550).
2. Fixed (design:563-608). Author model reproduces. My E, G, H (10-30 Hz): 0, 0.
3. Fixed with new defect N2 (design:568-569).
4. Fixed (design:572-575). 8,192 units: 7.9 ms, was 68 ms. New: N3, N6.
5. Fixed (design:499,611).
6. Fixed (design:401-412,501-511).
7. Fixed with a gap: REG1 (design:1157) omits `crates/turts_sim/tests/command.rs:322`.
8. Fixed (design:513-519).

## New findings

- N1 High. K19 (b), EL1: the hard projection moves a unit more than speed x dt (ratio 1.016). EL1 is red on a correct build. Fix: bound includes the projection.
- N2 Medium. design:590-592: a unit that passes its own group can stop Blocked inside a resting group unit. F (second group while the first moves): 15 overlaps, 13 Blocked. I (passing a resting group): 7, 9. Fix: leave every overlap before rest; add F and I to GMA.
- N3 Medium. K25 does not cap tick work: 8,192 orders per tick per slot; 8 slots, about 63 ms of spreading at 20 Hz. Fix: cap the work per tick, with a pending state (S:13.6).
- N4 Medium, needs decision. K19 (b) writes two full `AuditState` for each unit and tick (`transaction.rs:55-60`, `economylog.rs:22`), O(N·E). The design does not remove the `movement_pending` exception (`economylog.rs:326,553-556`). Options: (b); (c) one movement operation per tick, O(E).
- N5 Medium. GM40 determinism: the server takes no command log (`config.rs:38-51`). Live timing makes false reds. Fix: a headless replay at fixed ticks.
- N6 Low. 6.3 step 2 has no index for reserved cells. A scan: 34 ms. Fix: the 6.4 grid.

## What I ran

| run | claim | refute | observed |
|---|---|---|---|
| gm40v6.py | GM40 passes | overlap | as the author's |
| extra.py ×3 Hz | more cases | overlap | F, I overlap |
| diag.py | cause | cross-group | same group, Blocked |
| disp.py | K19 bound | ratio ≤ 1 | 1.016 |
| band.rs | O(N log N) | >20 ms | 7.9 ms |
| grep tests | REG1 complete | none | :322 |
| grep config | replay hook | a hook | none |
| read economylog | log cost | small | full states |

## Not checked

HashMap mutant (NOT TRACED); log size; mixed radii.

## What I changed

none
