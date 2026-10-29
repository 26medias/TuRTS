VERDICT=pass-with-changes FINDINGS=4 HIGH=0 RUNS=4 NOT_CHECKED=1

## Items

- H1, D:558: fixed with new defect M1. EL1 (D:1190) goes red when an input is missing in a stalling case.
- H2 and M1, D:604: fixed.
- M2, D:546: fixed. Both traces have no `transact` and no `capture` (`command.rs:236-252`, `gather.rs:88-175`). REG1 goes red on an extra operation: `m3_m2_stream.rs:399` compares the counts.
- M3, D:1187: fixed.

## New findings

- M1 Medium. D:558 writes the movement inputs only for a unit with a queue or in a group. A plain mover or a gatherer can stall too (6.5); the validator then lacks its timer. Fix: write each field when it is not the default.
- L1 Low. D:552 says "the movement step never writes an order". D:550 says it completes a plain move.
- L2 Low. D:604: the worst case is now 8 x 16,384 / 2,048 = 64 ticks, not 32.
- L3 Low. D:547: a plain move does write the command operation (`capture_command`, `command.rs:166-170`). It adds no other operation.

## What I ran

| run | claim | refute | observed |
|---|---|---|---|
| grep `command.rs:236-252` | no operation | `transact` | none |
| grep `gather.rs:88-175` | no operation | `transact` | none |
| read `m3_m2_stream.rs:242-346,399` | REG1 can go red | no compare | compares counts |
| read D:540-615 | the inputs are complete | an input missing | written by class |

## Not checked

EL1 in Rust (NOT TRACED).

## What I changed

none
