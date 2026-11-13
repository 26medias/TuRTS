# MP1 design revision 5 re-check: network and host

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=0

Revision 5 (12bca5b) closes all 13 findings, and the stage-1 start under `AllSeated(0)` holds end to end. Four Low text gaps remain. None blocks gate 1 for this lens; they can go into the plan. Point 14 is now answered: yes (question 4 below).

**1. Closure (all CLOSED; lines are in revision 5)**
- N1: CLOSED. ORDW1 case A (no drop) can now go red on plant (1). With change-only deltas and no `orders_cleared`, the stale record stays in the replica's order table. The delta-path replacement (replica.rs:523-525) was the only thing that cleared it, and plant (1) removes it. Case B and plant (3) test the Snapshot rebuild (replica.rs:403-418, :563). Case C tests the on-change rule and the 8-B `Fired` (row at 2452).
- N2: CLOSED. See 6.7.5:896, 6.7.7:1002-1010, 6.7.11:1183-1185, and LEAVE1 (`end_snapshot_tick`, `deltas_after_end`, the drop case).
- N3: CLOSED (6.7.10:1141-1147, 6.7.14:1262-1265, LEAVE1 rejoin case). N4: CLOSED (6.7.5:903-907, LOB1 run 0 red).
- N5: CLOSED (6.7.12:1190-1193, rows 4:249-250, lanes 10). One gap remains; see F3. N6: CLOSED (RES1, byte-equal export test). N7: CLOSED (6.7.8:1083-1088); the validator part is point 18.
- N8: CLOSED (6.7.7:1011-1013, LEAVE1). N9: CLOSED (ORDW1 C). N10: CLOSED (6.7.7:1024-1026). N11: CLOSED. My grep found 1 row hit each for `colour_unknown`, `lobby_changed`, `not_in_lobby` and `all_disconnected`. N12: CLOSED (6.8:1314-1317). N13: CLOSED (1:80, 6.7.10:1160).

**2. Stage-1 trace under `AllSeated(0)`**
- A connects. A gets seat 0 and `LobbyState{you:0, Waiting}` and enters `Seated`. The sim is gated, so the tick stays 0 (6.7.5:882-885, :893).
- B connects. In the same frame B gets seat 1, then the start step: claim seat i to slot i (`Slots::claim` gives the first open slot, slots.rs:46-57), `Welcome`, the tick-0 Snapshot built inside the step, and `MatchConfig`. Defined.
- A reloads before B connects. A's old connection is still live, so the reload's `Hello` is refused until L. At L the seat is released, and the retry takes seat 0 again. Defined (6.7.5:923-929, LOB1 run 4).
  - Not written for stage 1: B connects within L. The table counts A's dead seat as filled, so the start step claims slot 0 for A's dead connection. A joins about 10 s late through the Running reconnect path. GOAL1 measures slot 0 from a start tick that A missed. 6.7.5:930-934 writes this case only for the stage-2 countdown. See F1.
- B's connection drops in the same frame as the start step. Defined: the receive systems run first and release the seat (6.7.5:910-913). If the drop is noticed one frame late, the claim stays (D-B), and B rejoins through the Running path. NOT TRACED: which schedule detects the end of a connection. No handler exists today; the only mention is a comment at session.rs:710.
- A third client arrives when both seats are taken. It gets `no open slot` in Running (6.7.5:964). In the same frame as B, judgement in order gives the same answer (6.7.3:800-801).
- A stored id from an earlier process. The new process does not know the id, so it takes the "new id" path: a lobby seat, or `no open slot` in Running. Defined by the table.
- Stage 1 needs nothing that the design puts in stage 2: no `SlotConnection`, Countdown, `LobbyAction` or ready. The only gap is F3.

**3. End-tick order.** The Snapshot and `MatchResult` both ride `ReliableChannel`, which is `OrderedReliable` (protocol_plugin.rs:15-19; 6.8:1316). So the wire order is guaranteed. The client reads each message type from its own receiver, so it can still process `MatchResult` first within one frame. The rule "show the outcome only at replica tick >= `end_tick`" (6.7.7:1008) covers that case.

**4. Point 14: yes, the replica accepts a Snapshot that it did not request** (traced by reading; not run, because the client does not build here).
- `on_snapshot` holds a Snapshot back only when a request is outstanding and the hold hook is set. Otherwise it applies it at once (ingest.rs:422-436).
- `apply_snapshot` refuses only `tick <= last_applied` (`IgnoredStale`, replica.rs:217-221).
- When the end delta is lost, `last_applied` is t-1, so the end Snapshot commits (replica.rs:258-275).
- When the end delta arrived, the end Snapshot is `IgnoredStale` and changes nothing. See F2.

**5. `Immediate` tests.** The claim at `Hello` does not change. The sim gate changes nothing if the sim `MatchConfig` exists before the first fixed step. Revision 5 does not say who inserts it under `Immediate` or `--start-now`, or when. See F4.
- IMM1 names its red builds: a lobby as the default policy, `--start-now` ignored, a missing version-table row.
- I found no process test that depends on refusals skipping the bucket. The only bucket tests are unit tests in session.rs:1320-1556.

**New findings (all Low)**
- F1. Repair text for 6.7.5: "Stage 1: if a seated player reloads and the other seat fills before the server notices the old connection's end, the start step claims the slot (D-B), and the reload rejoins as a reconnect in Running."
- F2. Repair text for 6.7.12:1224: "`end_snapshot_tick` is recorded on receipt, also when the replica ignores the Snapshot as stale (replica.rs:217-221)." Otherwise LEAVE1 is red on a correct build whenever the end delta arrives.
- F3. The X12 stage-1 rows (4:250 and the section 10 stage table) leave out "the rejoin controller counts `Seated` as joined". 6.7.7:1036-1037 puts that change in stage 1, and LOB1 run 4 needs it. Add it to both rows.
- F4. Repair text for 6.7.5: "Under `Immediate` and `--start-now`, the boot inserts the sim `MatchConfig {start_tick: 1, team = slot}` before the first fixed step."

**What I ran** (claim | result that would refute it | observed)
- `git diff 5158f96 12bca5b -- crates scripts` | my code citations still hold | any code change | empty, exit 0. `git rev-parse HEAD` gave 12bca5b. `git status --porcelain` is empty, so I left nothing in the tree.
- `git show 12bca5b:<design>` to the scratchpad, plus the diff against faf70c5 | — | — | the closures above.
- `rtk proxy grep -cF` of 7 terms in section 12 | N11 and N2 rows exist | 0 hits | 1 or more hits each.
- Read ingest.rs:410-436, :753-810 and replica.rs:216-275 | point 14 | a Snapshot accepted only as a resync answer | accepted unless stale.
- Read protocol_plugin.rs:15-43 | the end order is guaranteed on the wire | two channels | one ordered channel.
- `pkg-config --exists alsa` | the client can build here | — | missing, so I could not run a replica test.
- Earlier this session: `turts_server` with `muster` and no client gave `ticks` 60 and `hellos` 0 after 3 s. That is the IMM1 green on today's code.

**What I changed:** none. Scratch copy: /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-net/r5.md (and r5.diff).
