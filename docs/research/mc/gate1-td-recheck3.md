VERDICT=pass-with-changes FINDINGS=1 HIGH=0 RUNS=5 NOT_CHECKED=1

Items
- N1b: fixed (D:546-551). `apply_move_to` (command.rs:236-252) and the gather order write no operation. The gather worker path has no other unit within 1.05, so 6.4 adds no push (haul and cedar maps, `interaction_range` 0.5).
- D:1259: fixed. 29 runs is correct.

New findings
1. D:1209, D:557-561. Medium. REG1 says only that the count stays the same. The test also compares `event_kinds` and `op.before == op.after` for each operation (m3_m2_stream.rs:116-120). Each gatherer tick is one captured operation (gather.rs:226). If `process_gatherer` writes a new `AuditState` field inside it (activity, stall timer), an operation that has before equal to after today flips, and the stream goes red. Fix: derive the activity, or write it outside the operation. Name all three compared fields in REG1. Also keep the float step of movement.rs:44-48 for a unit with no neighbour.

What I ran
- read m3_m2_stream.rs:110-125, :300-346 | REG1 | counts only | three fields
- read command.rs:230-252 | D:547 | an operation | none
- grep gather.rs transact/capture | D:549 | capture in assign | :226 only per tick
- read haul, cedar, base maps and units | 6.4 push | a unit within 1.05 | none
- read 6.4 | push rule | the depot overlaps | 3.0 > 2.5

Not checked: whether activity is stored or derived.

What I changed: none in the repository.
