VERDICT=pass-with-changes FINDINGS=5 HIGH=1 RUNS=13 NOT_CHECKED=4

D = the MC design, revision 2.

Original findings
- T1: fixed (D:431, D:946, D:1128).
- T2: fixed. (b) is honest; hashes are computed, not pinned. See N3.
- T3: fixed (D:278).
- T4: fixed (D:1098-1108); my probe re-run matches.
- T5: fixed (D:463-499). The version table runs after the catalog (archive.rs:134).
- T6: fixed (K24-K26, D:565).
- T7: fixed with a new defect (N4).
- T8: fixed (D:126, D:429, D:278).

New findings
1. N1, D:1157, D:545. High. Today movement writes no operation (movement.rs has no `capture_operation`). K19(b) adds one for each moving unit. The frozen M2 stream moves an acme unit (m3_m2_stream.rs:263) and counts every operation (:54-58). REG1 therefore fails, and the pin at refutations.py:18 goes red. Fix: filter movement out of the comparison, or re-freeze with the CEO.
2. N2, D:916, K27. Medium. Liveness is server state. Keep `Slots::claim` (slots.rs:20, :46-57). Reuse the slot in `judge_hello` (session.rs:93-109). The timeout is 10 s (join.rs:278), so a quick reload can get the refusal. WebSocket close detection: NOT TRACED. SLOT1 must reload in under 10 s.
3. N3, D:1.6. Medium. "M2 browser proof" is red on main (run 40943729334). No task owns it. Fix: record it and give it to task 18 or 21.
4. N4, D:1206. Medium. The named path is seven tasks and 36 runs, not "six" and 30. The longest is 2, 8, 9, 12, 15, 16, 21, 23: 43 runs.
5. N5, D:1209. Low. No row for `turts_client/src/lib.rs` (tasks 9, 10, 12, 16).

What I ran (claim | refute | observed)
- boundary.sh, revised rows | D:278 | red | green, 4 lines
- T1X copy | D:1101 | green | red
- real crate, wasm32 | D:1104 | green | red
- wasm-only bevy_ecs | D:950 | green | red
- direct winit | D:280 | green | red
- grep capture_operation | N1 | captured | absent
- read stream test | N1 | no move | :263
- gh run view | N3 | green | red
- path script | D:1206 | 30 | 43
- slots, session, join | K27 | no timeout | 10 s
- archive.rs:125-137 | D:497 | before catalog | after
- grep ORD1, D18 | T1, T8 | absent | present
- grep 17.1 | mapping | gap | 8/8

Not checked: the forge M3 stream; WebSocket close; the bucket clock; K20.

What I changed: none in the repository. I updated my memory.
