VERDICT=fail FINDINGS=5 HIGH=2 RUNS=5 NOT_CHECKED=2

## Items

- K19 (a), D:103,544-557: fixed with new defects H1, M2. EL1 red: outside write; old validator.
- N2, D:620-630: fixed (v7: 0 overlaps). Residual M3.
- N3, D:592-600: fixed with new defects H2, M1. 2,048 units: 3.2 ms.
- N5, D:1169: fixed.
- N6, D:589: fixed.
- REG1, D:1189: fixed.

## New findings

- H1 High. `AuditState` and D:551 omit the movement inputs: stall timers, pass mode, claim counts, group id. EL1 can be red on a correct build. The `blocked` failure comes from the movement step, outside `capture_operation`. Fix: put every input in `AuditState`; `Orders` writes the failure.
- H2 High. D:594: a 4,096-unit envelope (K4) never fits the 2,048 cap, so it never spreads. Fix: always spread the head envelope.
- M1 Medium. 8,192 orders per tick per slot come in; 2,048 per tick go out. The pending list has no limit; one FIFO lets one slot delay all others.
- M2 Medium. The stream test counts operations; it does not compare bytes (`m3_m2_stream.rs:31-36,119`). A captured start or completion of the `acme` move (`:258-279`) adds operations. Fix: no operation for a plain move.
- M3 Medium. F and I have 7 and 9 `Blocked` units (targets reachable); F penetrates 0.92. GMA (D:1167) passes both.

## What I ran

| run | claim | refute | observed |
|---|---|---|---|
| gm40v7.py | v7 passes | overlap | 0 |
| diff | reproduces | differs | order only |
| band2.rs | cap is cheap | >10 ms | 3.2 ms |
| read stream test | bytes matter | hashed | counts |
| read D:592-600 | big envelopes spread | rule | none |

## Not checked

DET1 Rust mutant (NOT TRACED); ECS cost.

## What I changed

none
