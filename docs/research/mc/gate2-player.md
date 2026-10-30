VERDICT=fail FINDINGS=8 HIGH=3 RUNS=9 NOT_CHECKED=4

# Gate 2, player lens: MC plan `docs/superpowers/plans/2026-10-29-mc-player-control.md` (f69badf)

Mode: review-plan. Status: plan DRAFT (plan:5). Design revision 6 passed gate 1 (Decided by CEO, K1-K27).
This file is in the scratchpad. My standing rule forbids writes to the repository in review mode,
so I did not write `docs/research/mc/gate2-player.md`.

## What a player can do today (baseline, f69badf)

- Window A (slot 0, forge): I see one engine panel top left with "Select Bore Drill" buttons. Every unit is the same red-orange colour. My machines and the enemy "Target Drone" look the same; only the name labels differ (shots/shot-2-after-input.png). The page has scroll bars. "Ore Vein" shows a box glyph.
- Slot 0 has no unit that moves. `--move 20,8` for slot 0 sent nothing (client-a.json: command_results []).
- Slot 1 moves both drones to (20,8). Both stand on the same point (c2b.json: units 9 and 10 at [20.0, 8.0]).
- A reload with the same id is refused: "player already in match" (client-a2.log, exit 1). A third player gets "no open slot" (c2c.json).
- The near drone dies at tick 544 (27.2 s at 20 Hz) with no orders (tick.log). So window B must open after about 27 s, or the tester sees two drones.

## Findings

1. HIGH. plan:641, plan:649, D:1639. PT1 is not blind. The CEO decided K1-K27 and K22, and read the Player path. The player agent has Read and Grep on the repository and can read D:1637-1679. Needs decision: (a) the CEO plays and the play log says "not blind"; (b) the CEO names a person who has not read the design; (c) the agent gets only the build URL and the goals, with no repository access.
2. HIGH. plan:647, plan:626, D:1146-1148. The PT1 instrument red is not shown. The pt1 driver holds keys and layout as literals. Remove guide page 5, and the driver still presses Ctrl+1. No step says the oracle checks the guide pages. The plant can stay green. I want a plant that makes the scripted run fail, or no claim of a red.
3. HIGH. plan:136, plan:275, plan:51, plan:714. Every screen task waits for task 9, and task 9 needs task 1 (P1). P1 needs a headed Firefox. Firefox did not answer in the plan run (120 s) or in my run (snap: "sanity timeout expired", 30 s). The stop rule covers "P1 is red on Firefox", not "Firefox does not start". Needs decision: (a) fix Firefox on the dev machine first; (b) P1 on Chrome only, Firefox NOT TRACED, the CEO picks K11; (c) another machine.
4. Medium. plan:342. Task 3 step 8 can be skipped when task 1 has not merged, and no later task adds the P1 rows. Then I can bind Stop to a key that the browser takes.
5. Medium. Player path steps 2, 3 and 18 have no named task. No step builds "the guide opens at the first join" or the F1 toggle (D:1063, D:342). No step builds the minimap ping (D:728, D:1676-1677): task 14 MM1 has no ping (plan:552), task 15 step 1 has no ping (plan:566), and task 15 does not need task 14 (plan:138). Step 9 text "Shell Turret: Cannot move" has no row before PT1.
6. Medium. plan:578, plan:581, plan:582. Page 5 comes from `Bindings`. Pages 1-4 are free text; the text rule forbids ticks and coordinates, not key names. "Press R to research" goes stale after a rebind. No row checks that page 5 changes after a rebind (plan:614, plan:615).
7. Medium. D:928-930, plan:614. No row tests the swap-or-cancel prompt or "Reset". Step 14 of the Player path asks me to change Stop; most letters near S are in use (D:344-346).
8. Medium. plan:649-651, plan:51. When PT1 or PT2 fails, the plan has no next step and no rule for a second blind tester. Task 23 gives the person no start command (`mc-browser.sh` runs a driver, plan:353) and no rule to open window B after the near drone dies.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| cargo build -p turts_server, -p turts_client, -p turts_pack (target/gate2-player) | baseline can run | build error | exit 0 |
| turts-pack scratch copy of forge | forge packs today | exit 2 | exit 0, sha256:2743f9b1... |
| server --run-ticks 600; client A (id 11), B (id 12), A again (id 11) | plan:441, D:957 reload refused today | Welcome for id 11 | A exit 0 slot 0; B exit 0 slot 1; A again exit 1 "player already in match"; server exit 0 |
| server --run-ticks 500; A; B --move 20,8; third id | D:957-958 "no open slot"; overlap today | third welcomed; drones apart | B applied tick 32, units 9 and 10 both at (20,8); third exit 1 "no open slot" |
| awk on tick.log, unit 9 | D:109 K22 near drone dies | unit 9 alive at tick 600 | last row tick 544 |
| timeout 60 firefox --version | plan:275 P1 can use Firefox | version printed | "sanity timeout expired: Interrupted system call" after 30 s |
| lspci | VIS2 headed local needs a GPU (plan:539) | no GPU | RTX 3080 present |
| trunk build --release --dist scratch; headed Chrome under xvfb-run on forge | today's player view | coloured slots, no panel | exit 0; one colour; engine panel; scroll bars |
| rtk proxy grep plan for ping, first join, swap, Reset, Bindings | each Player path step has a task | a task step names it | ping 0, first join 0, swap 0, Reset 0, Bindings 1 (plan:578 only) |

Paper walk: paper-walk.md in this directory.

## Not checked

- Firefox outside the agent sandbox (snap can fail only in the sandbox): NOT TRACED.
- `bash scripts/t7.sh` and `scripts/acceptance.sh`: not run (cost).
- Design re-review: out of scope.
- The per-unit refusal text with two reasons in one reply: no browser path makes one; I did not probe further.

## What I changed

none (repository). Scratch files only.

## Open questions for the orchestrator

- Findings 1 and 3 need a CEO decision.
- PT2 (plan:650) is fine as a feel test: the plant run gives the agent a red. The person run has no red build; that is normal for a person.
