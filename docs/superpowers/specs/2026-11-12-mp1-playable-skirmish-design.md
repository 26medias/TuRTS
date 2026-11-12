# MP1: Playable skirmish for two people: design

Date: 2026-11-12. Revision 4. Status: DRAFT, for the gate-1 re-check. Gate 1 ran on revision 3 with six lenses: 23 High findings (docs/research/mp1/gate1-*.md).

Revision history:

- Revision 2: section 6.9 from probe MP1-P1.
- Revision 3: the Yardstick coverage and Player path sections (qa-lead; the completeness gate passes), 12 new
  acceptance rows, section 16.
- Revision 4, the gate-1 repair in five parts (inputs `docs/research/mp1/repair-*.md`):
    - part 1 (backend-engineer): section 6.7 and the rows LOB1, LOB2, MAT1, RES1, SUR1, IMM1, LEAVE1;
    - part 2 (technical-director): sections 3, 6.1 to 6.6, 6.8, 6.9, the new 6.10, 9, 10 and row ORDW1;
    - part 3 (ux-designer): sections 5, 7, 8, 13, 16, the Player path and 17 rows;
    - part 4 (qa-lead): sections 1, 2, 4, 11, 12, 14, 15 and the Yardstick coverage section;
    - part 5, the reconcile pass (technical-director): one term list (1.1); one list of placement reason keys
      (6.1, 7.2); the site caps make an order wait (6.1, CON1, CON3, PRD1); one match-file shape (6.7.4, 6.10);
      one wire record list (6.7.13, 6.8); the interface rows of 6.10 aligned with section 7; the plants of STO1;
      the stage of TGT2 and of the statistics (10, 12); the gate-1 findings that no part repaired, repaired or
      moved to section 15.
Author: the orchestrator, from five seat inputs under `docs/research/mp1/` (producer, game-designer,
technical-director, ux-designer, bevy-rendering-engineer). The inputs are agent output. This design takes
their conclusions; gate 1 checks each claim that the design depends on.

Short references: `S:n` is `RTS-Engine-Project-Specifications.md` line n. `GD`, `TD`, `UX`, `RE`, `PR` are the
five input files (`input-game-designer.md`, `input-technical-director.md`, `input-ux-designer.md`,
`input-bevy-rendering-engineer.md`, `input-producer.md`). `MCD` is the MC design
(`2026-10-29-mc-player-control-design.md`, revision 16).

## 1. Goal and exit

MP1 is the first real game on the engine. Roadmap row (S:723, Proposed until this gate, S:701): two people
finish a match with mouse and keyboard; both build, power and fight; both see the result screen. The mod is
the first playable mod of specification section 16.2 (Decided, S:782): one faction, three infantry types, one
resource and a harvester, one tank, one turret, a command centre, a power station (S:782-805). The ground is
flat (G15). There is no bot, no fog of war, and no terrain.

MP1 has two stages (CEO order of 2026-11-11, as the orchestrator gives it; the vault entry is NOT TRACED).
Section 4 marks each item with its stage, and section 12 lists the rows of each stage.

- Stage 1, the thin playable loop: build power, harvest, train, fight, win, and see the result screen, with
  simple shapes. Stage 1 is done when its rows of section 12 are green, each with its recorded red; REG2 is
  green; and the `player` agent plays the loop from the goals text of section 13 to the result screen. Then a
  person plays the loop at checkpoint C1 (section 10), and stage 2 starts only after C1 passes.
- Stage 2, the breadth: the lobby, cancel, repair, rally, surrender and disconnect, models, sound, the MC
  items, and the person play test.

MP1 is done when:

1. Every row of section 12 except VIS3 is green, and each row has its recorded red: the output of its plant
   (the red build of the row) and its green output, each with the commit [rigour 7]. VIS3 is recorded, not
   judged. REG2 names its plant: on the MP1 tree, `clamp_to_map` returns at once
   (`crates/turts_interaction/src/camera.rs:358`), and `bounds_hold`
   (`crates/turts_interaction/tests/camera.rs:322`) fails with "ArrowRight must reach and stop at the authored
   map margin: Vec2(78.50007, 32.0)" (qa-lead mutation run on d729a97; the code of 1b15402 is the same: green, red, green after the restore).
   PT3 names its plant in row PT3 (ux-designer seat).
2. Two people play one full match to the result screen, and one person signs off (section 13; decision P7;
   G12, S:703).
3. The technical-director audit of the whole MP1 change finds no structural defect that is open.
4. The gates and the hosted CI are green in the form that the CEO decides for MC (decision note
   `vault/Decisions/2026-11-11 MC hosted CI without a GPU.md` on the MC integration branch, status open). The hosted runner has no GPU.
   Until the CEO decides, the MP1 pixel rows run on the local GPU gate with receipts (option b of that note).
5. `docs/measurements/mp1.md` records these numbers, each with the machine configuration and the package
   versions (spec 13.2). They are recorded, not judged (S 15) [perf measurements]:
    - server tick p50 and p99 over 200 or more ticks for an MP1 mixed scene at 2 x 200 units, at 1,000 units
      with 4 slots, and at 5,000 units with 8 slots, and the idle tick apart;
    - postcard bytes per tick per client, split by record kind, and the export time for all clients;
    - native `apply_delta` p50 and p99; browser frame p50 and p95 and the wasm memory high-water after 10
      minutes at 400 units, with the debug state off and on (DBG1, VIS3);
    - the boot-to-ready time of the server with `frontier` (backend L10);
    - the result of each balance duel of DMG1 and the end time of the scripted headless match.
   The D57 and D58 hosting benchmarks are not part of MP1.

### 1.1 Terms

Each term has one meaning in the whole design [rigour 11]. Sections 6.7.1 and 7.0 point to this list and add no
term.

- Player slot (slot): the D25 slot, one entry of `Slots` (`crates/turts_sim/src/slots.rs:22-25`). The start step
  claims it (decision D-B); under the start policy `Immediate` the claim happens at the accepted `Hello`, as today.
  A claimed slot is never released or reassigned in the match. The lobby screen words "Slot 1" and "Slot 2" (7.6)
  name the seat with that index.
- Lobby seat (seat): the reservation of one player-slot index in the lobby, before the start. The room can release
  it. A seat is not a claim. After the start, this design says "player slot" only.
- Claim: a player id recorded as the owner of a player slot in `Slots`. A claim is terminal for the match.
- Team: a set of player slots. MP1 has two teams of one slot (decision P2 a).
- Connection: one transport link (`ClientOf`) that sent an accepted `Hello`. Disconnected: the server ended the
  connection (a transport timeout, a kick or a clean close); the server knows nothing else about a player who is
  gone (6.7.8).
- Room: the lobby and lifecycle state machine in the new file `crates/turts_server/src/room.rs`. In MP1 each
  process has one room and one match (decision D-C).
- Start step, start tick, end tick: 6.7.5 and 6.7.8. The start tick is 1. Tick 0 is the world before the start
  tick (the `Snapshot` of tick 0); "from tick 0" in sections 4, 5, 7, 12 and the Player path means "from the start
  tick, with no input".
- Stage 1, stage 2: the two delivery stages of MP1 (this section). "Stage" has no other meaning in this design. A
  `stage` in a quoted MC oracle name (for example `sel2-500-slot0`) is the name of a test step.
- Job state: the condition of a production job or a site: running, slowed, `WaitingForFunds(resource)`, output
  blocked, or held. The words on the screen are the `states` texts of the mod (6.10 row H3).
- Constructor queue: the queue of a unit with `constructor`. It holds `Construct` orders only. Producer queue: the
  queue of a unit with `producer`. It holds jobs. Neither queue has an id (decision Q1). A lane (section 10) is a
  worktree and its task chain, not a queue.
- Held order: a `Construct` order, with its content and its point, in a constructor queue, that waits for an open
  site (6.1). It has charged nothing and holds no ground.
- Site: the server entity of a building under construction. It owns its progress, its charge and its job, it has
  health and a footprint, and it can be attacked (6.1, decision D-E). An open site is a site that is not complete.
- Pending site: the client marker from the left click until the server result. It is not a site and charges
  nothing.
- Engine order: an order that the engine gives through `apply_order` with the source `Engine` (6.5): the follow
  order of a rally, the start order of a map placement, and the placement of a held order.
- Edition: the lowest released engine API that the `engine_api` requirement of the root package admits (6.10).
- Team material: the material that a model names in `team_material` (8.1). The client replaces it with the slot
  material. (Revision 3 called it a "material slot".)
- Alert: a line in the alert list that an alert rule makes (7.9). It can have a sound, a minimap ping and a place.
  Notice: the text of a refused order, at the pointer, at a site, or the grouped failure line of ruling O14. A
  notice is not an alert.
- Hint: the one hint line of 7.8. A hint is done in the first client update in which its done condition holds. A
  done that holds only through a pending site ends when the server refuses that order (7.8) [rigour 10].
- Click to play: the button that ends the start screen. The clock of the 60 s rule (7.1) starts when the player
  presses it, and it ends at the first client update in which an own Power Station site or a complete own Power
  Station exists (a pending site does not end it) [rigour 10].
- `done:`: the Yardstick disposition only. "MP1 is done" is the exit of this section.
- Ids. Some ids occur in two lists, so the text always names the list. A row of the API table is written
  "6.10 row X" (for example 6.10 row P1, the payment row; 6.10 row H1, the reason keys). Alone, P1 to P9, Q1, S1,
  N1 and Y1 are decisions of section 2; X0 to X26 are scope items of section 4; H1 to H7, F and X1 to X3 in the
  form "hint H1" or in the Player path are hints.

## 2. Decisions for the CEO

Each line quotes the specification line that it touches and the status of that line. "Choice" is the
recommendation of the team or the call of the orchestrator after gate 1; the CEO can overrule each one. The
design below is written on the choice. "Changes" names, for each other option, every section, row of section
12, Yardstick coverage item and Player path step (numbered as in revision 3) that changes [rigour 9]. Every
line is "CEO to confirm": the design applies its Choice (or, where the Choice cell says "needs decision", the
recommendation) as the working text, and a different CEO answer changes the places that the last column
names. Y1 is the one exception: QA0 K6 lets only the CEO change a dependency edge, so the working text keeps
option (a) until the CEO answers.

| Id | Question | Specification line and status | Options | Choice | Changes for each other option |
| --- | --- | --- | --- | --- | --- |
| P1 | Real models in MP1? | S:247 (Baseline): "glTF is the preferred interchange candidate; exact supported features" stay to be selected; S:750 (Open): importer features | (a) simple shapes; (b) one static model and one texture set for each unit and building, from the package, with the shape as fallback; (c) wait for M13 | (b), with a cap of 9 models, in stage 2; stage 1 uses shapes (RE part 1) | (a) or (c): sections 4 (X16), 6.10 (rows V1, V2), 8.1, 8.5, 11 (minus 8 to 16 runs); rows MDL1 out, VIS3 primitives only; coverage MD17 and AN07 to `later:M13`; Player path: no step |
| D-M | Model caps (RE:24; mod M9) | S:247 (Baseline): the supported features stay to be selected | (a) the caps of section 8.1 now: 8,000 triangles for one model, 4 MiB for one model, 16 MiB for all models, at most 2 materials; (b) the X16 task sets the caps after the measurement of section 8.5 | needs decision (CEO; section 8.1 names it D-M); the working text applies (a). qa-lead recommendation: (a) as an upper bound that the 8.5 measurement can only lower, because the MDL1 red packages need fixed numbers | (b): MDL1 gets its cap cases in stage 2; section 8.1 numbers become provisional. No coverage item; no Player path step |
| P2 | Lobby | S:723 (Proposed until gate 1): "a minimal two-slot lobby (team, colour, ready)"; S:335 (Decided): readiness is a suggestion only | (a) two slots, ready and colour choice, team = slot; (b) two slots, ready only, team = slot; (c) team and colour choice | (a). Revision 3 wrote option (a) as "ready only", but its recommendation kept colour choice (LOB2, L4, TM03); the option text now says what the design builds, so every "P2 (a)" in the design keeps its meaning [rigour 9]. Stage 2 | (b): row LOB2 out; Player path L3 (default colours stay), L4 out; coverage TM03 to `done:` with the MC VIS1 slot colours. (c): sections 6.6, 6.7, 7.6; rows LOB1, MAT1 team cases; coverage TM01, TM02, MF10 stay `later` by R4 (MF02 needs MD12, `later:M9`) |
| P3 | Scope cuts if the schedule slips | S:798 (Decided, 16.2 rules): "Engineer: Repairs buildings and vehicles"; S:723 (Proposed): repair, rally, charge-over-time payment, cancel | (a) keep all; (b) cut rally; (c) also cut cancel of construction; (d) also cut repair: this changes the Decided S:798 and needs the CEO [rigour 2]. Charge over time is not a cut: it is decision P4 | (a). If the schedule slips: (b), then (c). (d) only by a CEO decision | (b): sections 4 (X9), 6.5; rows RLY1 out, SMT1 rally part out; coverage OR15, PR06 to `later:MP2`; Player path step 8 out. (c): X4b; row CON3 cancel part out (the destroyed-site part stays); coverage CO08 to `later:MP2`. (d): X8; rows REP1 out, SMT1 repair part out; Player path step 16 out; the specification line S:798 changes |
| P4 | Payment | S:801 (Decided, 16.2 rules): "Construction: Pay credits, select a valid site, then construction progresses over time"; S:434 (Baseline): "consume over time" is a valid policy | (a) pay at the order; (b) pay over time while the job runs | (b) (D-A, orchestrator). This changes the Decided order of S:801, so the CEO decides, and the specification line changes to "Select a valid site; construction progresses over time and charges credits as it progresses" [rigour 2] | (a): sections 4 (X7), 6.4, 7.3; rows PAY1, PAY2 (refund base becomes the paid cost), CON1 (charge clause), CON3, HUD5 ("On hold: no credits"), HUD6 ("paid while it builds"), ALR2 (no-credits state); coverage PR03, EC10, PR02, CO08 notes; Player path steps 3, 4, 9, 11 |
| D-A | Charge model of P4 (b) | S:434 (Baseline): "Cancellation, refunds, interrupted production ... must be explicit" | (a) each tick charges the difference of `floor(cost x progress)`, with a u128 intermediate; ledger events coalesced: one event when the job completes, is cancelled or stalls; (b) charge in progress steps of a mod-data size (TD:199-200); (c) as (a) with one ledger event for each tick | (a) (orchestrator) [perf M3] | (b): rows PAY1 (totals at step bounds), PAY2, HUD5 (the credits fall in steps); Player path step 3. (c): row PAY1 event-count case out; the ledger grows to about 400 events for a 20 s tank (perf M3). No coverage item |
| P5 | Sound | D69 (Proposed): audio | none; a minimal set (fire, hit, death, built, low power, ready, error); voices | the minimal set, in stage 2, with the engine part of X14 (an effect names a sound; the engine plays it at the place, with a cap) | none: sections 4 (X14 sound part, X17), 6.10 (row H16, the `sound` of V3), 8.4; row AUD2 out; coverage AU03, AU07 to `later:M13`; Player path steps 3, 13, 15 lose their sound clauses. Voices: X17 grows; no row change |
| P6 | Names, tone, art direction | S:807 (16.2, Decided section): "Names/art ... remain to be specified during implementation" | as revision 3 | Faction "Lantern Company" (working name), a mirror match over a claim; grounded near future, no gore; low-poly, clear top-down silhouettes, team colour on large surfaces; plain function names for units | Another choice: section 5 names; the words of Player path steps 2, 6, 7 and of section 13. No row; no coverage item |
| P7 | The two people of the play test | S:703 (Decided, G12): a person signs off | two people; or the CEO and one more person | two people, one of them not the CEO, so one tester is blind | The CEO as a tester: section 13 and row PT3 (one tester is not blind); exit item 2 |
| P8 | A lost connection | D28 (Decided): "Do not impose ... disconnect gameplay through the engine"; S:327 (Baseline): everyone disconnects is server policy | (a) the match waits for ever; (b) a slot that stays disconnected for N seconds is defeated, by a victory clause that the mod declares; with no clause, nothing happens [mod L11] | (b), N = 120 s in the `frontier` victory data, counted in ticks (D-D), in stage 2 | (a): sections 6.7, 6.10 (row M1, `DisconnectedFor`), 4 (X11); row MAT1 disconnect cases out; coverage MF14 to `later:M10`; Player path step 19 out |
| P9 | Units and buildings in the way | none (MC ruling O14 records the limit) | (a) a free approach point in the engine and a placement gap as mod data; (b) the M4 route step in MP1 | (a), in stage 1 | (b): sections 6.9, 11 (400 to 700 lines, 5 to 7 files, estimate of the probe); row OBS1 becomes a route row; coverage PF04 stays `in`. No Player path step |
| D-B | Claim boundary | D25 (Decided): "A claimed player slot ... cannot be reassigned"; S:308 (Baseline): "the exact claim boundary" needs design; S:350 (Baseline): open to claimed only by atomic admission | (1) a lobby seat is a reservation that the room can release; the match start claims every seated slot in one step, and D25 holds from that tick; a package with no start policy starts at once (`Immediate`), and the claim stays at Hello, as today (`crates/turts_server/src/session.rs:152`); (2) the claim stays at Hello, and a leave closes the slot | (1) (orchestrator) [rigour 3, backend H2, net 1] | (2): sections 6.7, 7.6; rows LOB1 (the leave closes the room), LOB2; coverage MF07 note; Player path L2 to L5 (a leave ends the lobby) |
| D-C | One match for each process | S:296 (recommendation): "Separate worker processes per active match are a recommendation" | (a) the server runs Boot, Lobby, Countdown, Running, Ended, then exits with 0; a supervisor starts the next process; (b) the process resets the world in place (about 30 resources, `crates/turts_sim/src/plugin.rs:60-90`) | (a) (orchestrator); no world reset in place [backend H1, net 12] | (b): sections 4 (X11b), 6.7; a new reset row; rows MAT1, RES1; Player path steps 17, 18, 19 (a next match with no reload) |
| D-D | Disconnect, reconnect, surrender | S:267 and S:392 (Decided, G9): replays are a command log; D28 (Decided) | (a) the room writes each as a sim input into the command log at a tick; the sim counts the 120 s in ticks; the rule is a clause of the mod victory data; (b) a server timer on the wall clock | (a) (orchestrator). (b) breaks invariant 15 (S:581), so it is not a valid option [net 2, backend H4] | (b): row MAT1 loses its replay case. No coverage item; no Player path step |
| D-E | Where construction progress lives | S:801 (Decided); S:434 (Baseline) | (a) the constructor queue holds the `Construct` order, with its point, until the engine places its site; then the site entity owns the progress, the charge and the job; `constructor` is a work capability beside `producer`; (b) the progress stays in the producer queue, and the site is a marker | (a) (orchestrator) [rigour 1, mod H2] | (b): sections 6.1, 7.3; rows CON1 (its red becomes the green), PRD1; coverage OR16, CO04, CO07 notes; Player path step 7 |
| D-E2 | A cap on open sites for each slot | none (net 5: a site at progress 0 is free) | (a) a cap as mod data; (b) no cap; (c) an engine constant | (a) (orchestrator). The `frontier` value is content (GD:141 proposes 1; S:807 keeps numbers open) | (b): row CON1 cap case out; the free-site grief of net 5 stays. (c): a private engine number. No coverage item; no Player path step |
| D-F | The harvester at the start | S:805 (Decided): "One command center, one harvester, and enough credits to establish power" | (a) a map placement can carry a start order (an engine mechanism, 6.10 row M3); the `frontier` harvester gathers from tick 0; the first hint names the power station; (b) no mechanism; the first hint is "Send your Ore Hauler to the ore" | (a) (orchestrator) [player H1] | (b): section 4 (X24); row STO1 out; GOAL1 first hint; coverage MF06 note; Player path steps 2 and 6 change order |
| D-G | A unit that is attacked | S:803 (Decided): "Move, attack, attack-move, stop, hold, and automatic targeting" | (a) an idle unit targets its attacker inside its leash; a holding unit fires back only inside its weapon range and never moves; (b) a holding unit chases inside its leash (revision 3) | (a) (orchestrator) [rigour 5] | (b): rows TGT1 (hold never moves) and TGT2 contradict; coverage OR08, OR26 notes; Player path step 12 |
| D-H | Where the browser keeps the player id | D52 (Decided): one controlling session for each claimed slot | (a) sessionStorage, as today (`crates/turts_client_web/src/lib.rs:239-251`); step 19 says "B reloads the page in the same tab"; (b) localStorage; a second tab becomes a D52 session replacement | (a) (orchestrator) [net 3] | (b): MC row SLOT1 (clause copied-tab-terminal-refusal) changes; Player path step 19 can say "closes its window". No coverage item |
| D-I | The public mod API 0.2.4 | D15 (Baseline): a versioned mod API | (a) a full table in the design: for each item the export group or file, field, type, default when absent, validation, error text and version-table row; presence means "written in the source", read from the raw file; (b) lane 0 defines the fields | (a) (orchestrator) [mod H1, mod M4] | (b): row VAL1 has no list of red packages before lane 0. Coverage MD01 note |
| D-J | Performance rules | S 13 (Baseline); budgets Open (S 15) | (a) the UnitId index and the debug-state gate in lane 0; acquisition from a grid for each relation, only when the weapon is ready and has no valid target, at most every K ticks, staggered by unit id; wire records with a start tick and a rate, sent on change; slice part A3 before content with a footprint above the mobile radius; (b) leave them to later milestones | (a) (orchestrator) [perf H1 to H4] | (b): sections 4 (lane 0), 10, 11 (minus 4 to 7 runs); rows TGT1 (query counter), DBG1, DES1 (index), OBS1 (A3); coverage CB03 note |
| D-K | A rematch | S:296: "A room may exist ... afterward for results or rematches" | (a) no rematch button in MP1; the result screen names the reason of the result; "Leave" says how to start the next match; (b) a rematch button (the room outlives the match, against D-C) | (a) (orchestrator) [player 9] | (b): sections 4 (X13), 6.7; rows RES1, LOB1; coverage MF12 note; Player path step 17 |
| D-L | A free income of the Command Centre | S:799 (Decided, 16.2 rules): "Harvester collects ore and delivers it to the command center for credits"; S:807: these policies are mod-owned | (a) an engine capability `income` as mod data; `frontier` gives 3 credits each second while the Command Centre is complete (section 5, row INC1); (b) no income; the floor comes only from a cheaper hauler | (a) (part-3 repair; the call is NOT TRACED in the orchestrator's brief). qa-lead: it adds a second credit source beside the Decided harvesting line; it does not contradict that line, but the CEO should see it [player 5] | (b): section 5 table, 6.10 row U7, row INC1 out; Player path steps that count credits (3, 6, 11) lose the income; the 166 s floor of section 5 goes. No coverage item |
| D-N | The VIS3 floor before the play test | S 15 (Open): numeric budgets | (a) p95 at or below 50 ms and p50 at or below 33 ms at 400 units on the play-test machine, as a start condition of PT3 (Proposed, not a budget); (b) no floor | needs decision (section 8.5 names it D-N); the working text applies (a), the recommendation of the performance and UX seats [perf L2] | (b): row VIS3 and PT3 lose the start condition. No coverage item; no Player path step |
| Q1 | Named producer queues | none (6.1; GD:272 asked for `lanes`) | (a) no named producer queues in 0.2.4: the command centre has the producer queue and the constructor queue (D-E); (b) named queues for each producer | (a) (part-2 repair, 6.1) | (b): sections 6.1, 6.10, 7.3; rows PRD1, VAL1; coverage PR04 note |
| S1 | Specification text edits after MP1 | S:723 and S:809 ("three" M3 rules; MC lifted one); S:801 (D-A); S:727 ("Needs teams (MP1)") | edit the lines; or keep them | edit: S:723 and S:809 say "two"; S:801 as in P4; S:727 says what MP1 gives (two teams of one, the `Ally` and `Enemy` relations) | No row; no coverage item; no Player path step |
| 6.7.14 | Four server policy points of section 6.7 | see 6.7.14 | see 6.7.14 (the `all_disconnected` default; `AllSeated`; when Ended exits; the disconnect notice timeout) | the recommendations of 6.7.14 | the changes that 6.7.14 names |
| N1 | The bare-id rule of the no-content scan | none (support code: `crates/turts_checks/tests/support/mod.rs:449-456`) | (a) match the qualified id and the bare local id for every kind, as today; a collision with an engine word names the definition path; the repository package renames, or the scan gets an exception row with a reason and `identifiers_only: true`; (b) match only qualified ids for the new 0.2.4 kinds | (a) (qa-lead): a leak in engine code is mostly a bare word in a string literal (for example `"power"`), and (b) cannot see it. An exception with `identifiers_only: true` keeps the literal red [mod H3] | (b): row NC1 bare-id plants go green. No coverage item; no Player path step |
| Y1 | The yardstick edge MP07 to MP01 | QA0 K6 (CEO): only the CEO changes a dependency edge (`docs/research/review-2026-10/inventory-engine.md:5-9`) | (a) keep the edge: MP07 and, by R4, MF07 are `later:M4`; (b) remove the edge, as K20 did for MP09: the client downloads the locked package with its maps since M1 | needs decision; the design applies (a); recommendation (b) [rigour 8] | (b): coverage MP07 to `done:crates/turts_server/tests/acceptance.rs` (test `t2a_package_parity`), MF07 to `in` LOB1; the inventory gets a K line. No row; no Player path step |

## 3. What exists and what MP1 adds

MC gave the engine: orders as content ids in an open envelope, group movement with slot pools, the HUD
widget set with mod layout, the client shell, control groups, key bindings with a settings menu, alerts,
and the pixels-only test harness.

The engine has none of the items below today. The technical-director checked each one on the MC tree
(branch `mp1-design`, code equal to the MC integration branch) on 2026-11-11:

| Item | Evidence |
| --- | --- |
| Construction | `UnitDef` has no constructor or placement field (`crates/turts_mod_api/src/schema.rs:333-367`); a production output must be mobile (`crates/turts_mod_runtime/src/content.rs:557`) |
| Power | `ResourceKind` is `Balance` or `Item` only (`schema.rs`, `pub enum ResourceKind`) |
| Armour and damage types | `WeaponDef` has `damage: u64` only (`schema.rs`, `pub struct WeaponDef`) |
| Teams | `SlotRelation` is `Other`, `Same`, `Any` (`schema.rs:245-249`); `SLOT_COUNT` is the constant 2 (`crates/turts_sim/src/slots.rs:7`) |
| A match end from mod data | the only stop reason is `TickExhausted` (`crates/turts_sim/src/economy/events.rs:183-198`) |
| A lobby | the server claims a slot at `Hello` (`crates/turts_server/src/session.rs:152`) |
| A result screen, rally, repair | no `SetRally` or `Repair` behaviour (`crates/turts_mod_api/src/orders.rs`, `pub enum Behaviour`) |
| A fire event on the wire | the sim stages `Fired` for the shooter's slot only (`crates/turts_sim/src/economy/combat.rs:393-402`); `turts_net_protocol/src` has no `Fired` (grep) |
| A unit heading on the wire | `UnitDelta` carries the position only (RE part 3) |
| One key for each build entry | the build menu uses the key of the producer's Produce order for every row (`crates/turts_client/src/widgets/sources.rs:391-393`) |
| An action scope for keys | `ActionDef` has `name` and `default` only (`crates/turts_mod_api/src/input.rs:10-14`); MCD:312 says actions have a scope |
| A start order on a map placement | `Placement` has `unit`, `owner_slot`, `pos`, `contents`, `facing` (`schema.rs:440-447`) |
| Idle acquisition | an idle unit fires only inside its weapon range (`crates/turts_sim/src/economy/combat.rs:297-339`) |
| A UnitId-to-Entity index | 78 `iter_entities()` calls in `crates/turts_sim/src` (grep) |
| Order records as changes | each delta carries the full owner order table (`crates/turts_server/src/export.rs:190-195`) |
| Version-table presence from the written file | presence comes from parsed values (`crates/turts_mod_runtime/src/content_mc_validation.rs:65-68`) |
| Checked reason and cursor keys | the typos `queue_ful` and `plcae` pack with exit 0 (mod-builder gate run) |

The M3 validation rules. S:723 and S:809 both say that MP1 lifts "three" M3 rules. MC already lifted one of
them, "weapons must be static", with the version-table row `weapon with movement`, since 0.2.3
(`crates/turts_mod_runtime/src/version_table.rs:52-56`). Two rules remain:
`crates/turts_mod_runtime/src/content_m3_validation.rs:374-376` ("a unit with health requires removal") and
`:420-424` ("a unit with a queue cannot be removed"). MP1 lifts them (X1, section 6.4). The text "three" in
S:723 and S:809 is stale after MC. The specification edit that corrects both lines is in the CEO list of
section 2 [rigour 12].

MP1 adds one engine API step (0.2.3 to 0.2.4, section 6.10) and one protocol step (4 to 5, section 6.8). Each
mechanism of section 6 is exposed to all mods through that API (S 4.3, S 5.2).

## 4. Scope

Types: E engine mechanism for all mods; C mod content; UI client; I test instrument. Stage: 1 is the thin
playable loop (build power, harvest, train, fight, win, result screen, with simple shapes); 2 is the breadth
(section 1). An item with two stages has its stage-1 part first.

| Id | Item | Type | Stage |
| --- | --- | --- | --- |
| X0 | Lane 0 (section 10): the 0.2.4 schema table of D-I with stub refusals and version-table rows; the wire freeze (protocol 4 to 5) with `unit_orders` as changes only plus `orders_cleared` (ORDW1); the start policy with `Immediate` as the default (D-B, IMM1); the UnitId index used at the lookup sites of perf H2 (D-J); the client debug state built only when the harness asks (DBG1, D-J); `turts_rules`; the behaviour handler table | E + I | 1 |
| X0e | Entry condition: slice part A3 (the neighbour grid) is merged before any MP1 content with a footprint above the mobile radius (D-J) [perf H4] | E | 1 |
| X1 | Lift the two M3 rules; destruction of a unit with a queue through the one death path, with an explicit `on_destroy` policy for its jobs | E | 1 |
| X2 | Damage types and armour classes as mod data; automatic targeting with an acquire range and a leash; retaliation: an idle unit targets its attacker inside its leash, a holding unit fires back only inside its weapon range and never moves (D-G, TGT2); acquisition from a grid for each relation, at most every K ticks, staggered by unit id (D-J) | E | 1 |
| X4 | Construction (D-E): `constructor` is a work capability beside `producer`; a `Construct` order with content and a point; the constructor queue holds the order, with its point, until the engine places its site; a site check; a site entity that owns its progress, charge and job and can be attacked; the limits `concurrent_sites` for each constructor and `open_sites_per_slot` for each slot as mod data; no named producer queues (Q1); a footprint that blocks | E | 1 |
| X4b | Cancel of construction with refund of what the site charged | E | 2 |
| X5 | Placement mode and ghost | UI + E (interaction core) | 1 |
| X6 | Power: supply and demand for each slot, a shortage rule for each capability | E + C + UI | 1 |
| X7 | Charge over time (D-A): each tick charges the difference of `floor(cost x progress)` with a u128 intermediate; the job state `WaitingForFunds`; coalesced ledger events | E | 1 |
| X8 | Repair: a `repairer` capability and a `Repair` behaviour, charged over time | E + C | 2 |
| X9 | Rally: `SetRally` on a producer; the engine sends the follow order through `apply_order` with the source `Engine`: no seq, no token, no `OrderResult` (net 8) | E + UI | 2 |
| X10 | Relations: `Ally` and `Enemy` beside the present slot filter; MP1 uses two teams of one | E | 1 |
| X11 | Match lifecycle: a declarative victory rule as mod data and an outcome for each slot | E + C | 1 |
| X11c | Surrender, disconnect and reconnect as sim inputs in the command log; the disconnect rule as a clause of the mod victory data, counted in ticks (D-D, P8) | E + C | 2 |
| X11b | One match for each server process (D-C): Boot, Running, Ended, exit 0 (stage 1); Lobby and Countdown (stage 2, with X12); a client that connects after the end: the owner gets the result, a new id gets "The match is over" (LEAVE1) | E | 1, 2 |
| X12 | Lobby for two slots (D-B): lobby seats are reservations; the match start claims every seated slot in one step; the lobby states Lobby and Countdown; ready and colour (P2 a); the browser keeps the player id in sessionStorage from the lobby seat (D-H) | UI + E (server room) | 2 |
| X13 | Result screen, owned by the mod, from engine sources; it names the reason of the result; no rematch; "Leave" says how to start the next match (D-K) (stage 1); the statistics (stage 2) | UI | 1, 2 |
| X14 | Effects: `Fired` and `Destroyed` on the wire (stage 1); flash and death effect (stage 1); the other effects and the sound engine part: an effect names a sound, and the engine plays it at the place of the effect with a cap (stage 2, P5) | UI + E (wire event) | 1, 2 |
| X15 | The mod `frontier`: the 8 types as shapes, ore, the map "Twin Fields", the HUD layout, the numbers of section 5 (stage 1); the guide and the tuned numbers (stage 2) | C | 1, 2 |
| X26 | Free income of the Command Centre: an engine capability `income` as mod data (D-L, INC1) | E + C | 1 |
| X24 | A starting order on a map placement: the engine applies it through `apply_order` with the source `Engine` at the start tick; the `frontier` harvester gathers from tick 0 (D-F, STO1) | E + C | 1 |
| X25 | Hint lines: an ordered hint list in mod data; each line names the event that starts it and the event that ends it; the first hint names the power station (D-F) | E + C + UI | 1 |
| X16 | Models (decision P1) | E + C | 2 |
| X17 | Sound set (decision P5) | C | 2 |
| X18 | MC items handed to MP1: Button `font`, "Unbound", Controls groups and keyboard navigation, the alert list defects, a value predicate for labels, the grouped failure notice (O14), the no-GPU page | UI | 2 |
| X19 | One protocol step (4 to 5) and one mod API step (0.2.3 to 0.2.4) for all of MP1, both frozen in lane 0 (X0) | E | 1 |
| X20 | Instruments: the headless two-client match and the stage-1 rows (stage 1); the stage-2 rows, the fault plants and the pixels-only two-player driver (stage 2) | I | 1, 2 |
| X21 | The play test: the `player` agent on the stage-1 loop and the person of checkpoint C1 (stage 1); two people (stage 2) | I | 1, 2 |
| X22 | Final regression (REG2) and CI; REG2 also runs at the end of stage 1 | I | 1, 2 |
| X23 | Architecture audit before the merge to main | I | 2 |

The state alerts of ALR2 are in stage 1, though the stage order does not name them: the CEO play of MC failed
on an alert that repeats (`mods/forge/interface/hud.ron:130-136`), and stage 1 has the low-power and
no-credits states.

Not in MP1: a bot (MP2), fog of war (M5), terrain (M4), routes round obstacles beyond section 6.9,
skeletal animation (M13), more than two slots, rooms (M9), scripting (MS), a rematch (D-K), a world reset in
place (D-C), team choice (P2), a worker that walks to its site (D-E).

## 5. The mod `frontier` (content)

The content is in a new package `frontier`, not in `base`, because the frozen M2 stream test uses `base`
(`crates/turts_pack/tests/m3_m2_stream.rs:350-366`). All ids passed the game-designer's scan against engine
identifiers (`light` and `shell` collide and are not used).

Display names: Rifleman, Launcher, Engineer, Ore Hauler, Tank, Guard Turret, Command Centre, Power Station. Resources:
ore (an item), credits (a balance), power (a `Supply` resource, section 6.2). The screen, the hints, the alerts and the
play-test texts use these names only. The word "harvester" does not show to a player [player 8].

Status: the roster and the rules of S:782-805 are Decided. Names, statistics, timings, costs, repair accounting,
construction restrictions, the power multiplier and the simultaneous-destruction case are Open (S:807). Every number
in this section is a proposal to measure in a headless match. The full stat tables stay in GD parts 1 to 6. The table
below gives only the values that the gate-1 repair changed, with the reason. A balance ratio that fails in a headless
duel is a retune of these numbers, recorded in `docs/measurements/mp1.md`; it is not a red of DMG1. DMG1 is red only on
the formula and the table cells [rigour 6].

Numbers after gate 1 (game-designer pen, repair pass; the scripts are listed after the results):

| Item | Gate-1 value (GD) | Repaired value | Why |
| --- | --- | --- | --- |
| Starting credits | 1,200 | 1,200 | S:805 ("enough credits to establish power"); the other changes remove the holds |
| Ore Hauler cost, build time | 800, 20 s | 500, 15 s | a lost hauler can be replaced [player 5] |
| Ore Hauler load, gather, unload, speed | 200, 10/s, 40/s, 2.0 | 300, 15/s, 60/s, 2.4 | income on the main field 332 to 519 credits a minute [player H3] |
| Ore Hauler at the start | waits for an order | gathers from tick 0: the map placement carries a start order (decision D-F; engine mechanism: a placement `start_order`, 6.10 row M3, sent through `apply_order` with the source `Engine`, row STO1). Today a placement has no order field (`crates/turts_mod_api/src/schema.rs:440-447`) | the first "+300" at about 30 s with no input [player H1] |
| Free income of the Command Centre | none | 3 credits each second while it is complete (decision D-L, engine capability `income`, row INC1) | the floor: a slot with 0 credits and no hauler gets a hauler in 166 s [player 5] |
| Rifleman build time | 6 s | 7 s | the spend rate of one unit queue (14.3 credits/s) is near one hauler plus the free income |
| Launcher build time | 9 s | 10 s | the same |
| Engineer cost | 300 | 250 | the same |
| Tank build time | 18 s | 20 s | the same (35 credits/s) |
| Guard Turret cost, range (health 600 unchanged) | 500, 8 | 600, 7 | defence is not twice as cost-efficient [player H3]; 7 still outranges the tank (6) and equals the Launcher |
| Damage cells rocket to structure, cannon to structure | 50 %, 100 % | 60 %, 125 % | tanks and launchers break buildings; formula unchanged (GD part 1) |
| Command Centre health | 2,000 | 3,000 | a broken base has time to answer |
| Build area | 10 from the edge of an own Command Centre or Power Station | 10 from the edge of the own Command Centre only (radius 12.5 from its centre) | no turret creep to the side and centre ore: those fields stay open ground, so careful players must meet there [player 10] |
| Ore nodes | main 3 x 1,500; side 2 x 1,500; centre 8 x 2,000 | main 3 x 5,000; side 2 x 3,000; centre 8 x 3,000 | the defended main field lasts 9.6 min with 3 haulers, 14.5 min with 2 |
| Placement gap | 0.5 (GD) or 2.0 (6.9) | `placement.gap` 2.0 as data; the packer refuses a gap below the largest mobile diameter + 0.2 (1.8 + 0.2 for the Tank and the Ore Hauler); units count for overlap, not for the gap | [mod L10] |
| Ore clearance | 2.0 from the node edge | 2.0, data `placement.node_clearance` | unchanged |
| Open sites for each slot | GD: 1 | 1, mod data `open_sites_per_slot` (decision D-E2); a further `Construct` order waits in the constructor queue of the Command Centre, which holds up to 5 held orders (6.1) | [net 5] |
| Unit queue capacity | 5 | 5 | unchanged |
| Power | station makes 100; Command Centre uses 40; turret uses 30 | unchanged; build time of the Power Station 12 s (24 s at the start, because the start is in low power) | the tooltip shows the time at the current power (7.10) [player 7] |

Results of the scripts (first 180 s unless the row says otherwise; "hold" is a span in which a job cannot pay, with
gaps below 2 s merged, because free income lets a starved job pay in some ticks):

| Run | Gate-1 numbers | Repaired numbers |
| --- | --- | --- |
| S1, the player seat's ordinary script (power, 3 riflemen, turret, tank, 2 riflemen, tank, 2 launchers), hauler from tick 0 | longest hold 31 s, 3 holds | no hold; 445 credits at 180 s |
| S4, power and a second hauler first | longest hold 24 s | no hold |
| S7, the step order of the hint list (Player path) | longest hold 16 s, 7 holds | one hold of 4 s at 95 s |
| S5, tank rush (overspends on purpose) | 31 s | 25 s |
| S8, greedy (overspends on purpose), 300 s | holds up to 33 s; "Not enough credits" sounds with the rev-3 rule: 8 | holds up to 30 s; rev-3 rule: 89 sounds (free income ends and starts the state in most ticks); the rule of 7.9: 1 sound |
| L1b, the only hauler dies at 90 s; at 95 s the player cancels the tank job and trains a hauler | no hauler in 420 s | a new hauler at 110 s |
| L1, the same without the cancel (the hauler waits behind a starved tank job) | no hauler in 420 s | a new hauler at 333 s; hint X1 tells the player to cancel first |
| L2, the floor: 0 credits, no hauler, a hauler queued | never | at 166 s |
| One turret with its power share (690; gate-1: 590) against the cheapest winning attack of riflemen, launchers, tanks and engineers | 800 / 590 = 1.36 | 750 / 690 = 1.09 |
| Tanks only against one turret | 2 tanks: 1,400 / 590 = 2.37 | 2 tanks: 2.03; one tank and two riflemen win at 1.30; one tank alone loses (the turret keeps 100 of 600) |
| Tanks only against two turrets | 3 tanks: 1.78 | 3 tanks: 1.52 |
| match.py, careful against careful with 10 % less fight value | A wins at 11.8 to 14.0 min | A wins at 11.4 min |
| match.py, careful against turtle (6 turrets) | careful wins at 16.9 min | careful wins at 20.2 min |
| match.py, aggressor (2 tanks first) against careful | careful wins at 16.9 min | careful wins at 20.2 min |
| match.py, aggressor against turtle | aggressor wins at 2.9 min | aggressor wins at 3.5 min |
| match.py, two equal careful players | no end in 40 min; ore gone at 27.6 min | no end in 40 min; ore gone at 30.4 min |

What the results support, and what they do not:

- The first 3 minutes: no hold above 4 s for the ordinary scripts S1, S4 and S7. A hold comes only when the player
  overspends (S5, S8), and then the alert sounds one time (7.9) [player H3].
- The loss of the only hauler is not a silent loss: the alert "Ore Hauler destroyed" and hint X1 name the action, and
  the free income is a floor (L1b, L2) [player 5].
- Defence: a mixed attack needs 1.09 times the cost of one turret; a tank-only attack needs 2.03. The role lines (7.10)
  tell the player to bring a mix [player H3, player 6].
- Match length: unequal pairs end in 11 to 20 min with the repaired numbers. The macro model is crude: it has no
  tactics, so two exactly equal careful players stall in it with both number sets. The repaired build area forces both
  players onto open ground after 9.6 to 14.5 min. The headless match (section 14) and the play test measure the real
  length. This is a proposal, not a measurement.

Scripts (Python 3 standard library; run with `python3 -I`; the gate-1 player scripts are the control, and
`pacing.py old` gives the same holds as `docs/research/mp1/probes/player/play.out` S2):
`pacing.py [old|new]` (first minutes, the floor, the alert rule), `fights2.py [old|new]` (battles and the defence
ratio), `match.py` (macro match), all with their output in `results.txt`, now in
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-pacing/`. The plan
copies them to `docs/research/mp1/probes/pacing/`.

The ratios that carry the balance (fights2.py, new):

- Fights last seconds and are readable: a rifleman kills a rifleman in 9 s; a tank kills a rifleman in 6 s.
- At equal cost, 7 riflemen lose to a tank and 8 win; 3 launchers kill a tank in 7.5 s; 5 riflemen kill 2 launchers in
  3 s.
- One turret beats one tank; a tank with two riflemen beats one turret; two tanks kill a turret in 10 s.
- One Ore Hauler earns about 519 credits a minute on the main field and pays for itself in about 1 min.
- One Power Station supplies the Command Centre and two turrets; the third turret needs a second station.

Power, in the words the player gets: "When your base uses more power than it makes, everything builds at half speed
and your turrets switch off."

Victory: at the end of a tick a team is alive if one of its slots has a Command Centre with health above 0. One team
alive: it wins. No team alive: a draw for every team that was alive at the start of the tick. The disconnect clause
is mod data in the victory rule: a slot that stays disconnected for 120 s (counted in ticks) is defeated (decision
D-D). A package with no clause gets no disconnect defeat (D28).

Interface content (in `mods/frontier/interface/`): the goal line and the hint list of 7.8 (texts in the Player path),
the alert rules of 7.9, the reason texts of 7.2, the role lines of 7.10, the key defaults of 7.4, the lobby palette
and the result screen of 7.6.

No private path: every rule above is data in the package. No content id of `frontier` appears in engine code; the
no-content scan checks it (row NC1).

## 6. Engine mechanisms

Each mechanism is exposed to all mods through the public mod API (S 4.3, 5.2). The owner crate and the
decision that keeps later milestones cheap are from TD part B.

### 6.1 Construction (X4, X4b, X5)

Status: S:178 (4.3, Decided principle) puts placement, footprints, progress and occupancy in the engine, and
"worker-built versus instant placement" and costs in the mod. The design below follows decision D-E.

**One home for each job** [rigour 1] [mod H2]:

- A constructor is a unit with the `constructor` capability (6.10 row K1): `buildings` (the production entries
  that it can build), `concurrent_sites` (the number of open sites that it can have at one time),
  `queue_capacity` (the number of held orders), `progress` and `when_short`.
- The player places a building with a `Construct` order (content and point). The constructor queue holds each
  `Construct` order until the engine places its site. The queue holds orders only. It never holds progress or
  charge. There is no order without a point, so there is no "ready to place" state: the left click of 7.2 gives
  the point.
- The engine places a site at once when the constructor has fewer than `concurrent_sites` open sites and the
  slot has fewer than `open_sites_per_slot` open sites (6.10 row K6). Otherwise the order waits in the queue,
  in order, as a held order. The engine checks the site when the order arrives (a refusal goes to the player in
  the `OrderResult`, per unit, D65) and again when it places the site (a refusal then is an engine-order
  failure, section 6.5).
- **The two caps make an order wait. They never refuse it.** An order that arrives when the constructor is at
  `concurrent_sites`, or when the slot is at `open_sites_per_slot`, waits in the constructor queue. The engine
  refuses an order at arrival only when the constructor queue already holds `queue_capacity` held orders
  (`queue_full`). When a site completes, is cancelled or is destroyed, the engine places held orders in the next
  tick while both caps allow, in ascending constructor unit id, then in queue order. There is no `site_cap`
  reason. Reason: `frontier` has one constructor and the slot cap 1 (section 5), and 7.2, 7.3, row PRD1 and
  Player path step 8 show a second order that waits ("Waiting to start"); a refusal at the slot cap makes the
  queue of 5 useless. A held order has charged 0 and holds no ground, so the wait keeps the closure of net 5.
- The site is an entity. It owns its progress, its charge and its job. Its footprint blocks from the tick it
  appears. It can be attacked. It neither provides nor consumes supply (6.2). Its weapon, producer and
  constructor capabilities are off until it is complete.
- The constructor counts only its open sites. When a site completes, is cancelled or is destroyed, the
  constructor can place its next held order.
- A site does not need its constructor after it appears. A site whose constructor is destroyed continues
  (`Autonomous` progress).
- `constructor` is a work capability. The rule "at most one work capability"
  (`content_m3_validation.rs:418`) gets one exemption: `constructor` beside `producer`. A constructor beside any
  other work capability is still refused. So the command centre has two queues that run at the same time: the
  producer queue for units and the constructor queue for structures. MP1 adds no named producer queues
  (decision in the CEO list).

**Progress.** `progress: Autonomous` is the only MP1 form: the site progresses by itself. `NeedsBuilder(reach)`
is the named later variant (a worker must stand within `reach` of the site). The schema has it in 0.2.4, and the
packer refuses it with the text "reserved for a later engine API" (6.10 row K1). A worker mod (MB) then needs no
schema change.

**Free sites are a grief path** [net 5]. Under charge over time (6.4), a site at progress 0 has charged 0, and
it holds ground. Two limits close this: `concurrent_sites` for each constructor and `open_sites_per_slot` for
each slot. Both are mod data. An order above a limit waits as a held order, which holds no ground (above). `open_sites_per_slot` is required when a package has a constructor, so no
engine default hides it (6.10 row K6).

**Placement rules are data** (6.10 rows K2 and K3) [mod L10]:

- The site footprint is fully inside the map.
- If `placement.in_build_area` is true: the site centre is within `build_area` of the footprint edge of a
  completed own unit that declares `build_area`.
- No overlap with any footprint, mobile units included.
- Between two static footprints, the edge distance is at least `max(gap of the new building, gap of the other
  building)`. A mobile unit counts by overlap only, not in the gap: the gap exists so that mobiles can pass
  between buildings (6.9), and a unit that walks past must not block a site.
- The edge distance to a resource node is at least `placement.node_clearance`.
- `placement.gap` is a number in the package. The packer refuses a gap below the largest diameter of a unit
  with movement in the resolved package set, plus 0.2. For the proposal of GD part 1 (tank radius 0.9) the
  smallest legal gap is 2.0, so the GD value 0.5 is refused.
- Footprints stay circles (`schema.rs:28-30`).

**Refusal causes** (reason keys, validated, 6.10 row H1) [player H4]. This is the one list of the design; 7.2,
6.8, 6.10 and the rows CON1 and GHO1 use it. The checks run in this order, and the first failure is the cause:

1. `site_outside_map`: a part of the footprint is outside the map ("Outside the map").
2. `site_out_of_area`: `placement.in_build_area` is true, and the centre is farther than `build_area` from the
   footprint edge of every completed own unit that declares `build_area` ("Too far from your base").
3. `site_blocked`: the footprint overlaps a footprint, a unit included ("Something is in the way").
4. `site_near_building`: no overlap, but the edge distance to a static footprint is below the gap ("Too close to
   a building").
5. `site_near_resource`: the edge distance to a resource node is below `placement.node_clearance` ("Too close to
   ore").
6. `queue_full`: the constructor queue holds `queue_capacity` held orders (an existing wire key,
   `crates/turts_net_protocol/src/messages.rs:241`; "Construction queue is full (5/5)").

The texts in brackets are the `frontier` texts; the texts are mod content. Why these names: revision 4 part 2
wrote `site_too_close`, which does not say "close to what" (the gap and the node clearance are both "too
close"); part 3 wrote `site_near_ore`, but `ore` is a `frontier` id, and no engine name uses a `frontier` word
(6.2); `site_out_of_area` names the `build_area` field that the check reads. The placement of a held order (an
engine order) runs checks 1 to 5 again; a failure there shows in `last_failure` (6.5).

**One pure function, with visibility** [net 6]. `turts_rules::placement::check(site, view, visible)` gives the
verdict and the cause. `visible` is the viewer's visibility predicate. In MP1 it is always true. When a hidden
unit blocks a site (M5), the verdict is still a refusal, but the cause is `site_blocked` with no unit id, so a
refusal never names a hidden blocker. The server calls the function on the authoritative world. The
interaction core calls the same function on the replica for the ghost colour, the reason at the pointer, and the
gap and build-area rings (UX 7.2). The client holds no second copy of a rule.

**What the client verdict can promise** [net 9]. The client sees positions that are one tick plus the latency
old. For a scene with no motion and no new site between the client frame and the server tick, the client
verdict equals the server verdict. When they differ, the player sees the pending site, then the server refusal
with its reason at the pointer and the error sound, and the credits do not change.

**Cancel** (X4b). `CancelConstruction` on a site cancels it. A cancel of a held order removes it and refunds 0,
because it charged 0. Refunds: 6.4.

### 6.2 Power (X6)

Power is one use of a generic supply mechanism. A second mod can use the same mechanism for a population cap
[mod M5]. Status: S:426 (7.1) names "network quantity" resources, distributed "according to the selected network
model"; S:802 (16.2, Decided, D55) is the frontier rule; S:177 puts population rules in the mod.

- **Resource kind.** `Supply(scope: Slot, counts: Completed)` (6.10 row S1). `scope: Slot` is one network for
  each slot; M6 adds `Network` scope for the utility networks of S:549, with no change to the unit fields.
- **Units.** `provides` and `consumes`: lists of (supply resource, amount) (6.10 row U5). The field name is not
  `power`, because `power` is the frontier resource id and the no-content scan would match it.
- **Counts.** `Completed`: a unit counts when it exists and is not a site. `Queued` (also count the outputs of
  queued jobs) is named in 0.2.4 and refused as reserved. MP1 builds `Completed` only.
- **Computation.** Once for each tick, at the start of the economy phase, the engine sums `provides` and
  `consumes` for each slot and supply resource. A slot is short of a resource when consumed is greater than
  provided. All effects in that tick use that one result. A removal in tick t takes effect at the computation of
  tick t+1.
- **Shortage effect for each capability.** Each capability states its own effect in `when_short` (6.10 row U6):
  `Rate(ratio)` (progress of a producer job or a site runs at the ratio), `Disable` (the weapon neither fires
  nor acquires), and `BlockStart` (no new job starts), which is named and refused as reserved. A capability
  with no `when_short` entry is not affected. So frontier writes `Rate(1/2)` on the producer and the
  constructor, and `Disable` on the turret weapon; the rifleman weapon has no entry and is not affected; the
  engineer's repair has no entry, so repair is not slowed (GD part 4).
- **No hard-coded power.** No "power" check exists in production, combat or construction code. The code reads
  the slot supply result and the capability's `when_short` rules. NC1 checks the content ids.
- **Shortage disable is not SetEnabled.** A weapon that is disabled by a shortage keeps its player `enabled`
  state, so recovery restores the state that the player set.
- **On the wire and in the HUD.** A `SupplyRecord{slot, resource, provided, consumed}` goes to the owner on change
  (6.8). The HUD source is `Supply(resource, Provided | Consumed)` and the predicate `SupplyShort(resource)` (6.10
  rows H7, H10). The alert triggers are `SupplyShort(resource)` and `SupplyRestored(resource)`, and the credits
  trigger is `InsufficientFunds(resource)` (6.10 row H4). No engine name uses a frontier word such as "low power"
  or "credits".
- **The population cap of a later mod.** It needs `counts: Queued` and `BlockStart`. Both have their names in
  0.2.4, so adding them later is additive and changes no field.

### 6.3 Damage, armour, targeting (X2)

Status: S:179 (4.3) puts target queries and damage events in the engine, and weapons, armour, damage types and
targeting restrictions in the mod. Decisions D-G and D-J apply.

**One damage formula.** The design (`max(min, floor(d x ratio) - flat)`) and GD:68 (`max(1, floor(d x percent /
100))`) disagree [mod H1]. This design takes the design form, because it holds the GD form as a case (flat 0,
minimum 1, ratio = percent/100) and also covers flat armour. In 6.3 and 6.4, `combat.rs` is
`crates/turts_sim/src/economy/combat.rs` [rigour 12, the path]:

    base = weapon.damage + modifier          (the research modifier of today, combat.rs:371-374, checked add)
    raw  = floor(base x num / den)           (num/den: the cell of (weapon damage type, target armour class))
    hit  = max(minimum, raw saturating_sub armour.flat)
    applied = min(hit, health)               (as today, combat.rs:376)

- All arithmetic is integer, with a u128 intermediate for `base x num`. No float takes part.
- The reason is exact authoring, not an invariant: health and damage are already `u64` (`HealthDef.max`,
  `WeaponDef.damage`), and a float ratio gives a different value for the same data. Probe (this repair):
  `floor(100 x 0.29)` in floating point is 28; the integer form `100 x 29 / 100` is 29. Invariant 15 (S:581) is
  the same-build replay rule; any deterministic arithmetic meets it, so it is not cited for integers here
  [rigour 12].
- A package with a damage table must give one cell for each pair of damage type and armour class. A missing
  cell is a pack error, not a default (6.10 row C3) [mod M6].
- A package with no damage table keeps the damage of today (`hit = base`).
- `minimum` is on the damage table. `flat` is on the unit armour (6.10 rows C4, U1).
- The formula is one function in `turts_rules`, used by the sim and by the client tooltip.

**Acquire is on the unit** [mod M6]. `acquire: (range, leash)` belongs to the unit, not to the weapon: a leash
is a property of the unit's movement, and a later unit with two weapons has one leash. `Fired.weapon` is the
weapon index on the unit (0 in MP1, where a unit has one weapon) (6.8).

**Idle acquisition.** A unit with `acquire` that is idle, with a weapon that is ready and not disabled and has
no valid target, takes the nearest enemy (distance, then unit id) inside `acquire.range` that the visibility
predicate allows. It chases while its distance from its idle point is at most `leash`, and then it returns to the
idle point. An ordered target comes first (`combat.rs:323-326`). A unit without `acquire` keeps the behaviour of
today: it fires only inside its weapon range (`crates/turts_sim/src/economy/combat.rs:297-339`) [rigour 12, the
path].

**Retaliation and hold (decision D-G)** [rigour 5]:

- The engine records the attacker of each unit that takes damage in a tick (the lowest attacker unit id when
  several hit it in one tick).
- An idle unit with `acquire` targets its attacker if the attacker is inside its leash (the same chase rule as
  idle acquisition).
- A holding unit fires back at its attacker only when the attacker is inside its weapon range. A holding unit
  never moves.
- A unit with no weapon, with a disabled weapon, or with an order that is not Hold does not react.
- Retaliation applies only to units with `acquire`, so no 0.2.3 package changes behaviour (6.10 row U4).

**Acquisition cost (decision D-J)** [perf H3]:

- Candidates come from one grid for each relation: one grid for each team, built once for each tick, with the
  units of that team. A unit queries only the grids of the teams that are `Enemy` to it. Friends are never
  candidates.
- A query runs only when the weapon is ready and has no valid target, and at most once every
  `ACQUIRE_PERIOD_TICKS` ticks for each unit, staggered by unit id (a unit queries in tick t when
  `(t + unit_id) mod K = 0`). K is an engine constant, 4 at the MP1 tick rate (Baseline, recorded in
  `docs/measurements/mp1.md`; the tick rate stays Open, S 15).
- The grid cell is the smallest weapon range in the catalog (slice part A3).
- Today `choose_target` runs before the cooldown check (`combat.rs:201`, `:229`). The new path runs the query
  after the readiness check. Units without `acquire` keep the order of today, because the attack-move state needs
  acquisition during the cooldown (`combat.rs:224-228`).
- Row TGT1 asserts acquisition within K ticks of an enemy entering the range.

**Visibility predicate.** Every target query (acquisition, retaliation, attack validation, repair targets, the
placement check of 6.1) takes the viewer's visibility predicate. It is always true in MP1. M5 (fog) and MP2
(bots) use the same queries.

**Damage types and armour on the wire.** No new record: health is public (`export.rs:180`), and the client derives
a hit from the health drop. The tooltip shows the cell value from the catalog through `turts_rules`.

### 6.4 Payment, cancel, destruction (X7, X4b, X1)

**Payment over time (decision D-A)** [rigour 2]. S:801 (16.2, Decided, D55) says: "Pay credits, select a valid
site, then construction progresses over time". Decision D-A changes this order: the player selects a valid
site, and the construction progresses over time while it pays. This is a change of a Decided specification line.
It is in the CEO list of section 2, with the specification edit, and the CEO can overrule it. S:434 (7.2) names
"consume over time" as a valid production policy, so the mechanism itself is inside the specification.

`PaymentPolicy::ChargeOverTime` (6.10 row P1) applies to production jobs (units) and to sites (buildings):

- **Progress in integers.** A job has `done` and `total = duration_ticks x Q`. `duration_ticks` comes from the
  conversion of today (`try_ms_to_ticks`, `crates/turts_sim/src/boot_m3.rs:591`). `Q` is the least common
  multiple of every `Rate` denominator in the catalog, so each tick adds the whole number `Q x num / den` (`Q` at
  full rate). The packer refuses `Q` above 65,536 and a `total` that overflows u64.
- **Charge.** For each cost entry, the total charged at `done` is `floor(cost x done / total)`, computed with a
  u128 intermediate. `cost x done` is a product of two u64 values, so it always fits in u128, and the result is at
  most `cost`. Each tick charges the difference to the previous total [perf M3].
- **All costs or none.** For a job with more than one cost entry, a tick charges every difference or none. If
  any source cannot pay its difference, the tick charges nothing and the job does not advance. Its job state is
  `WaitingForFunds(resource)`, with the first resource in cost order that cannot pay. It resumes on its own.
  The balance never goes below 0.
- **Order inside a tick.** Jobs pay in ascending unit id of their owner (producer or site), then in queue order.
- **Proof run (this repair).** A model of these rules (scratch `td-repair/charge.py`, 20,000 cases, costs up
  to u64::MAX, five rates, random stalls) gave a charged sum equal to the cost in every case, and no negative
  difference.
- **Ledger events are coalesced.** A job writes one `Charged{job, resource, amount}` ledger event when it
  completes, when it is cancelled, and when it enters `WaitingForFunds`, with the sum since its previous event; it
  never writes one for each tick (400 events for one 20-second tank at 20 Hz is the cost this avoids) [perf M3].
  The job record holds its `charged` amount for each cost entry. The ledger rule of `turts_audit` uses it: in
  each record, the change of a balance equals the ledger events plus the negative change of `charged` over the
  jobs that pay from that balance. No MP1 behaviour adds a branch to a validator (TD part E1).

**Refunds.** A refund is `floor(charged x numerator / denominator)` for each cost entry, with a u128
intermediate, from the job's `cancel_refunds`:

| Case | Ratio | Note |
| --- | --- | --- |
| Cancel of a held order or a queued job | any | it charged 0, so the refund is 0 |
| Cancel of a running job or a site | `cancel_refunds.running` | |
| Cancel of a complete job that waits for a free exit | `cancel_refunds.output_blocked` | it charged the full cost |
| Destruction of the producer or of the site | `cancel_refunds.destroyed` (new, 6.10 row K5) | 0/1 when absent: a destroyed site refunds nothing (CON3) |

Each refund is one `Refunded{job, resource, amount}` ledger event.

**Destruction (X1).** It goes through the one death path (`crates/turts_sim/src/economy/transaction_m3.rs:354`,
called from `crates/turts_sim/src/economy/combat.rs:405`). In one captured operation it:

1. cancels the running and queued jobs of a destroyed producer and refunds each by `cancel_refunds.destroyed`;
2. refunds a destroyed site by the `cancel_refunds.destroyed` of its production entry;
3. drops the held `Construct` orders of a destroyed constructor (refund 0); its open sites continue (6.1);
4. releases every claim of the unit;
5. removes the footprint in the same tick (DES1).

The two lifted M3 rules (6.10 rows K7, K8): a unit with health and no `removal` is destroyed with its contents
(`ContentsPolicy::Destroy`); a unit with a queue (producer, machine, research or constructor) can be removed, with
the explicit job policy above (S:434: destruction policies are explicit, not side effects).

### 6.5 Repair and rally (X8, X9)

**Repair.** S:798 (16.2, Decided): the engineer repairs buildings and vehicles. Fields: 6.10 row R1.

- A `repairer` unit has `rate_per_second`, `reach`, `cost` (a list of (resource, amount, per_health)), `targets`
  (a slot filter), `classes` (armour classes; empty means all) and `auto_range`.
- Cost is an integer ratio for each health point: `amount` for each `per_health` points. A cost of 0 is legal:
  `amount: 0`, or an empty `cost` list, gives free repair [mod L12].
- Heal for each tick: `floor(rate x t / tick_hz)` minus the same value at `t - 1`, where `t` counts the ticks of
  this repair order. Applied heal is at most `max - health`. Charge is by applied health: the total charged after
  `H` applied points is `floor(H x amount / per_health)`, and each tick charges the difference. A tick that cannot
  pay heals nothing, and `t` does not advance (job state `WaitingForFunds(resource)`). Several repairers on one
  target are independent, and the health cap stops all of them.
- Repair writes health only inside a transaction (`stage_health`) and charges as coalesced ledger events (6.4).
- The repairer approaches to `reach` of the target footprint edge through the one movement model; it adds no
  movement path (TD part E2).
- `repairer` is not a work capability: it has no queue, and a unit runs one order at a time, so a unit can have
  a weapon and a repairer. A later combat engineer then needs no rule change [mod L12].
- An idle repairer with `auto_range` repairs the nearest damaged target that its filter accepts inside that
  range, with the cadence and grid rule of 6.3.

**Rally.**

- `SetRally` on a producer stores a rally target: a point, a unit or a node. A `SetRally` on the producer itself
  clears it. The rally goes to the owner in the producer's order record (`UnitOrderRecord.rally`, 6.8).
- When the producer spawns a unit, the engine gives the new unit a follow order: the smart rule that the new unit
  would get for a right click on the rally target. The engine reads `orders/rules.ron` `rally` when it is present,
  else `smart` (6.10 row O7). So frontier can map the ground to attack-move for combat units and a node to gather
  for haulers, with no hauler special case.

**Engine orders** [net 8]. The follow order of a rally, the start order of a map placement (decision D-F, 6.10
row M3) and the placement of a held `Construct` order are engine orders:

- They go through `apply_order` (`crates/turts_sim/src/orders.rs:2610`) with the source `Engine`. They are not
  player envelopes; a fake player envelope is refused (TD part B5).
- They have no sequence number, cost no rate-limit token, and give no `OrderResult`. Today `OrderResult` is
  routed by (slot, seq) (`crates/turts_server/src/outbound.rs:26-31`); an engine order writes nothing into that
  list.
- A refusal shows in the unit's `last_failure` (`UnitOrderRecord.last_failure`,
  `crates/turts_net_protocol/src/messages.rs:136`). The client needs no order id: the follow order shows in
  `unit_orders.current`.
- The order log (`crates/turts_server/src/orderlog.rs`) records the source `Engine`.
- Engine orders are not inputs of the command log. A replay derives them again from the inputs (G9,
  invariant 15).

### 6.6 Relations and teams (X10)

Status: S:183 (4.3) puts players, teams and lifecycle in the engine; decision P2 (a): team = slot in MP1.

- `SlotRelation` (`schema.rs:245-249`) gains `Ally` and `Enemy` (6.10 row L1). Meanings:
    - `Same`: the own slot (it is the "Own" of GD:278, so no third new variant is needed).
    - `Ally`: a slot of the own team, the own slot included.
    - `Enemy`: a slot of another team.
    - `Other`: any other slot. It keeps its meaning. Code must not use `Other` to mean enemy (TD trap B8).
    - `Any`: every slot.
- In MP1 each slot is its own team, so `Ally` equals `Same` and `Enemy` equals `Other`. The frontier package
  writes `Enemy` on its weapons, so that the same package works when M5 adds teams of more than one slot.
- The team of each slot comes from the match configuration (6.7). Under the `Immediate` start policy, each slot
  is its own team.
- One function `turts_rules::relation(a, b, teams)` gives the relation. The target filters, the acquisition grids
  (6.3), retaliation, repair targets, the build area (6.1) and the victory check (6.7) use it.
- Export takes a viewer group (a team or a slot), so M5 can share vision inside a team. In MP1 the group is the
  slot.
- `Welcome` and `MatchStart` carry the team of each slot (6.8), so the client uses the same relation function.

### 6.7 Match lifecycle, lobby, result (X11, X12, X13)

This section is the server side of the match: the room, the lobby, the claim, the start, connectivity, the
outcome, the result and the end of the process. The screen words are in section 7.6 (UX seat). The wire
records are in the table of section 6.8 (technical-director); section 6.7.13 names each record that this
section needs, with its fields. This section applies the decisions D-B, D-C, D-D, D-H and D-K of the gate.

Spec status that applies here: D23, D24, D25, D26, D28 and D56 are Decided (S:52-57, S:85). The worker
process for each match is a recommendation (S:296). Readiness and a countdown are suggestions only (S:335).
The claim boundary needs design (S:308). D70 (who owns the result screen) is Proposed (S:99).

#### 6.7.1 Terms

The terms of this section (player slot, seat, claim, team, connection, disconnected, room, start step, start
tick, end tick) are in the one list of section 1.1 [rigour 11]. The wire records of this section are the
authority for their rows in the table of 6.8, and the match-file fields of 6.7.4 are the authority for rows A2,
K6, M1, M2 and O9 of 6.10.

#### 6.7.2 One match for each process (decision D-C)

The server runs one match for each process: Boot, Lobby, Countdown, Running, Ended, then exit with code 0.
A supervisor starts the next process (section 6.7.10). There is no world reset in place [backend H1].

Reasons from the code: the world is built one time, at boot (`crates/turts_server/src/main.rs:72`,
`boot.rs:38`). The match id is made one time (`main.rs:78`). `Slots` has no release (`slots.rs:46-57`).
About 30 resources make the sim state (`crates/turts_sim/src/plugin.rs:60-90`); a reset in place would have
to clear each of them. A new process gives a new match id and an empty `Slots` for no code. This is the S:296
recommendation; S:296 also asks for separate room and simulation lifecycles, and section 6.7.5 keeps them
separate inside the one process.

#### 6.7.3 The claim boundary (decision D-B)

The rule, in one sentence that M10 keeps: a lobby seat is a reservation that the room can release; the start
step claims every seated slot in `Slots` in one step; from the start tick, D25 holds, and no leave,
disconnect, defeat, surrender or administration action releases or reassigns a claim; under the start policy
`Immediate`, the claim happens at the accepted `Hello`, as today (late admission, S:337) [backend H2]
[rigour 3] [net 1].

Facts that the rule depends on:

- Today the claim happens at `Hello` (`crates/turts_server/src/session.rs:152-159`), and the economy log
  records it (`session.rs:261-263`). Under `Immediate` this does not change.
- The start step is one exclusive system. It calls `Slots::claim` for the seats in index order. All slots are
  open at that time, so seat i gets slot i; the step asserts this and exits with code 1 on a difference (it
  cannot happen in a lobby process).
- Two `Hello` messages in one frame get two different seats, or one refusal, because one system judges them in
  order (`session.rs:162-176`, `:215-237`). The seat table has the same rule.
- The row LOB1 checks the boundary: after the start, a new id gets `no open slot`; in the lobby, a leave
  releases the seat.

#### 6.7.4 The start policy (mod data)

The mod selects the start policy (S:333, D23). The engine supplies the variants. Readiness is not mandatory
(S:335), because `Immediate` exists.

| Variant | Behaviour | MP1 |
| --- | --- | --- |
| `Immediate` (default when the field is absent) | Boot goes to Running. Claims happen at `Hello`, first open slot. This is the behaviour of every package today | built |
| `AllSeatedReady(countdown_secs)` | The countdown starts when every seat has a human and every seat is ready. `countdown_secs` is 0 to 30 | built; `frontier` uses 3 |
| `AllSeated(countdown_secs)` | The D23 automatic start: the countdown starts when every seat has a human | reserved, not built (decision 2 below) |
| a creator start | Needs a room creator (M9) | not in MP1 |

`Immediate` keeps the 31 ignored process tests of `crates/turts_server/tests` (31 lines with `#[ignore`),
hd1, T7 and the MC rows working with no change to their commands: their packages write no match data, so
they boot to Running and claim at `Hello` [backend H3]. Row IMM1 checks this.

The server test hook `--start-now` starts the match at boot as if the policy were `Immediate`, for any
package. Every `frontier` headless row that uses `--command-script` with no client (DMG1, CON1, PWR1, PAY1,
OBS1, MAT1 script cases) uses it [backend H3].

The match data. This list is the authority for the rows A2, K6, M1, M2 and O9 of the 0.2.4 table of section
6.10, which copy its names (the reconcile pass replaced the earlier 6.10 forms `start_policy`, `AllReady`,
`DisconnectDefeat` and `match/rules.ron`) [mod H1, the match part]:

- Export group `exports.match`, at most one file, for example `match/match.ron`. Version-table row
  `exports.match`, since 0.2.4. Presence: the key `match` is written in `exports` in the raw manifest (D-I).
  A `^0.2.3` package that writes it gets the existing text (`crates/turts_mod_runtime/src/version_table.rs:101-108`):
  "`exports.match` needs engine api 0.2.4; `^0.2.3` admits 0.2.3".
- `MatchDef { start: StartPolicy, victory: Option<VictoryRule>, open_sites_per_slot: Option<u16> }`. Absent
  `start` gives `Immediate`. Absent `victory` gives no outcome, ever (D28) [mod L11]. `open_sites_per_slot` is
  the site cap of 6.1; it is required when a unit of the package set has `constructor` (6.10 row K6).
- `VictoryRule::LastTeamStanding { defeat: Vec<DefeatClause> }`. The trait behind it lets MS add a script
  form later.
- `DefeatClause::KeyUnitsLost { units: Vec<ContentRef> }` and `DefeatClause::DisconnectedFor { secs: u32 }`.
- The order behaviour `Surrender` (a new `Behaviour` variant, `crates/turts_mod_api/src/orders.rs:32-42`):
  version-table row "behaviour Surrender", since 0.2.4, detected from the parsed variant (VAR, 6.10 row O9;
  `ron::Value` drops variant names, so the raw file cannot show it). Orders exist since 0.2.3, so
  `exports.match` does not cover this row.
- The colour palette is the existing `slot_colours` list of the interface (`crates/turts_mod_api/src/interface.rs:14`).
  A colour on the wire and in the room is an index into that list. The server reads only the length of the
  list, at boot. NOT TRACED: whether `turts_pack::load_package` parses the interface file today.

Pack-time checks and their texts (the form of 6.10, with the package prefix `frontier/` left out):

- `exports.match: at most one file, found 2`
- `match/match.ron: start: countdown_secs must be 0 to 30, found 45`
- `match/match.ron: start: AllSeatedReady needs 2 different slot colours in the interface, found 1` (one
  different colour for each player slot; a package with a lobby policy and fewer different colours is refused;
  6.10 row H15)
- `match/match.ron: start: AllSeated is reserved for a later engine API`
- `match/match.ron: open_sites_per_slot: a package with a constructor needs open_sites_per_slot` (6.10 row K6)
- `match/match.ron: victory.defeat[0]: KeyUnitsLost needs at least one unit`
- `match/match.ron: victory.defeat[0].units[0]: unknown unit `frontier:comand_center`; did you mean `frontier:command_center`?`
- `match/match.ron: victory.defeat[1]: DisconnectedFor secs must be above 0`
- `match/match.ron: victory.defeat: DisconnectedFor is written 2 times; at most 1`

#### 6.7.5 The server states

The room lifecycle and the simulation lifecycle are separate (S:296). The room owns the state. The sim gets
one input from the room before the start: the sim `MatchConfig` (section 6.7.11). Every sim system runs in a
`SimSet` (`plugin.rs:31-44`); each set gets the run condition "the sim `MatchConfig` exists". Without it, the
tick does not advance and no sim system runs. The server fixed systems that read the tick also wait for it:
the economy log capture (`main.rs:117`, `:120`), `exit_after_ticks` (`main.rs:139-142`), the tick log and the
tick count of `--stats`. So `--run-ticks N` counts sim ticks only; in the lobby it does not count.

Today the sim ticks with no client: 39 ticks before any `Hello`, and two clients got `Welcome` at ticks 76 and
171 (gate 1 probe, `scratchpad/be-probe/probe.out`) [backend H1].

| State | What runs | Leaves the state when |
| --- | --- | --- |
| Boot | `server.ron` read; package loaded; world built (`boot.rs:38`, `main.rs:104`); match id made (`main.rs:78`); HTTP and WebSocket bound | the boot ends: to Lobby if the policy is a lobby policy; to Running if the policy is `Immediate` or `--start-now` is set. A boot error exits with code 1 (`main.rs:36-38`) |
| Lobby | the world exists and does not tick (tick 0); seats; lobby actions; HTTP | the start policy holds: to Countdown |
| Countdown | as Lobby; the room counts `countdown_secs x tick_hz` fixed steps of the server clock. The sim still does not tick | any accepted seat change (a leave, a colour change, `unready`): to Lobby. The count reaches 0: the start step, then Running |
| Running | the sim ticks from the start tick; orders; connectivity inputs; the outcome check once for each tick | the outcome check gives the end (every slot has an outcome, section 6.7.8): to Ended |
| Ended | the sim is stopped (`MatchStopped`, reason `MatchOver`); the result is served | see section 6.7.10: exit with code 0 |

The start step (Countdown to Running), in one exclusive system, in this order:

1. Build the `MatchConfig` from the seats (section 6.7.11) and freeze it.
2. Claim each seat in `Slots` (section 6.7.3). Write each claim to the economy log at tick 0.
3. Put a `Session { slot }` on each seated connection and send `Welcome`, then the `Snapshot` of tick 0
   (section 6.7.7).
4. Insert the sim `MatchConfig`. The next fixed step advances the sim to tick 1. The start tick is 1.

The receive systems (`Hello`, connection ends, lobby actions) run before the room transition, and the room
transition runs before the start step, in one frame (`session.rs:123-133` is the present receive chain). So a
seat whose connection ended in that frame is released first, and the room goes back to Lobby before the start
step can claim it.

Each event of the lens list, defined:

- The second player never comes. The room stays in Lobby with no time limit. The waiting player sees the
  status "Waiting for a second player" (7.6). The engine has no minimum-player rule, no auto-fill and no
  deadline (S:333, S:337, D23). The operator stops the process.
- A player leaves in the lobby. The seat goes to `Open`, the revision goes up, and the other seats keep their
  ready flags. The other player sees the leave text of 7.6 ("Player 2 left. Waiting for a second player.").
  The text comes when the server notices the leave, at most about 10 s after the loss (section 6.7.8).
- A player leaves, or clears ready, during the countdown. Countdown to Lobby. The countdown starts again from
  its full length when the policy holds again.
- A seat change during the countdown. Every accepted change of a seat (its occupant, its colour, its ready
  flag) ends the countdown, because a colour change clears the ready flag of that seat (L5). MP1 has no action
  that moves a player to another seat.
- All players disconnect during the match. The new `server.ron` field `all_disconnected` applies (S:327):
  `Continue` (default) or `Shutdown(after_secs)`. `Continue`: the sim keeps ticking; the mod clause
  `DisconnectedFor`, if any, gives the outcome. `Shutdown(after_secs)`: when no claimed slot has a connection
  for `after_secs` seconds of wall time, the server logs `turts_server: all players disconnected; shutdown`
  and exits with code 0, with no outcome. This timer ends the process; it changes no sim state, so a replay
  is not affected. Pause and save need M10 and are not MP1 values. Slot ownership never changes (S:327)
  [backend H1].
- The match ended, and a client connects. The owner of a claimed slot that did not send `LeaveMatch` gets
  `Welcome`, the `Snapshot` of the end tick and the `MatchResult`. Any other id, and an owner that sent
  `LeaveMatch`, gets `JoinRejected` with the reason `The match is over` [backend H1] [net 12].
- Both slots reach the disconnect limit in one tick: a draw for both. At different ticks: the first slot is
  defeated at its tick, and the other slot wins at that tick (section 6.7.8) [backend H4].

What a connecting client gets in each state. The protocol and lockfile checks come first in every state, as
today (`session.rs:146-151`).

| State | New id | Id with a seat or claim and no live connection | Id with a live connection |
| --- | --- | --- | --- |
| Lobby | the lowest open seat, then `LobbyState`; no open seat: `no open slot` | not possible: a seat is released when its connection ends | `This game is open in another window` (`session.rs:157`) |
| Countdown | `no open slot` (every seat is full under `AllSeatedReady`) | not possible, as above | as above |
| Running, after a lobby start | `no open slot` | `Welcome`, `Snapshot`, then deltas; `SlotConnection { connected: true }` at the next tick | as above |
| Running, `Immediate` | the first open slot is claimed: `Welcome` (late admission, as today); none open: `no open slot` | as above | as above |
| Ended | `The match is over` | owner that did not leave: `Welcome`, `Snapshot`, `MatchResult`; owner that left: `The match is over` | as above |

#### 6.7.6 Lobby rules

- The seat count is the player-slot count (2 in MP1, `slots.rs:7`).
- A new seat gets the lowest palette index that no seat holds. So the two seats start with different colours
  (L3).
- The seat label is `Player N`, with N = seat index + 1. The server makes it. MP1 has no name input. A label is
  not an identity (S:410).
- Lobby actions (client to server): `SetReady(bool)` and `SetColour(index)`. The server checks each one against
  its current state. Refusals:
  - `colour_taken`: another seat holds the index. Nothing changes.
  - `colour_unknown`: the index is outside the palette.
  - `lobby_changed`: a `SetReady(true)` names a revision older than the last change of an occupant or a
    colour. So a player cannot ready on a state that they did not see. Two ready presses in the same instant
    both pass, because a ready change does not make an older ready stale.
  - `not_in_lobby`: any lobby action after the start (S:308 freezes the room rules for the running match).
- An accepted colour change clears the ready flag of that seat (L5). A colour change to the colour that the
  seat already holds changes nothing.
- The room has no creator and no administrator in MP1. The D56 rows for a creator (S:322-327) start at M9.
  No MP1 action can reassign a seat of another player.

#### 6.7.7 Message order

With a lobby policy:

1. Client: `Hello`. Server: a seat and `LobbyState { you: seat }` to the new member, and `LobbyState` (next
   revision) to every member. Or `JoinRejected`.
2. In the lobby: `LobbyAction` from a member; `LobbyState` to every member after each accepted change;
   `LobbyRefused` to the sender after a refused action.
3. At the start step, in one frame: `Welcome { slot_id, tick: 0, tick_hz, match_id, start }`, with
   `start = MatchStart { start_tick: 1, teams, colours }`, then the `Snapshot` of tick 0. Teams and colours come
   with `Welcome` (section 6.8) because `Welcome` now comes at the start [backend M6].
4. Deltas from tick 1. `MatchState` changes ride in the deltas.
5. At the end tick: the delta of the end tick, with the outcomes, then one `MatchResult` (reliable).
6. Client: `LeaveMatch`. Server: a kick after `KICK_DELAY` (`session.rs:30`).

Under `Immediate`: `Hello`, then `Welcome { tick: the current tick, start: MatchStart { start_tick: 1, teams,
colours: the default colour of each slot } }`, then the `Snapshot`, as today (`session.rs:268-283`).

A reconnect in Running: `Hello`, `Welcome { tick: the current tick }`, `Snapshot`, deltas.

Orders:

- A seated connection has no `Session` before the start. Today the server drops the commands of such a
  connection with no answer (`session.rs:357-360`). MP1 answers each one with an `OrderResult` with the reason
  `match_not_started`. The refusal costs a token, as every envelope does (net 5).
- An order from a slot that has an outcome gets `slot_defeated` while the match runs (only possible with more
  than two teams; a sim test covers it, row MAT1).
- An order after the end tick gets `match_over`. Today the server skips such commands with no answer
  (`session.rs:386-388`).

The player id in the browser (decision D-H). Today the browser saves the player id in sessionStorage at
`Phase::Joined` (`crates/turts_client_web/src/lib.rs:259`). `Joined` comes from `Welcome`
(`crates/turts_client/src/join.rs:107-113`), and `Welcome` now comes at the start. So the client gets a new
phase `Seated { seat }` at the first `LobbyState` with `you` set, and the save moves to `Seated`. Also the
rejoin controller counts `Seated` as joined (`join.rs:479`, `crates/turts_client/src/rejoin.rs:306-311`).
Without both changes, a reload in the lobby loses the seat, and a reload with a stored id ends after 15 s in
the lobby with the text "This game is open in another window. Close it, then reload this page"
(`rejoin.rs:7-8`, `:338-341`) [backend M6]. Step 19 is a reload in the same tab; closing the tab clears
sessionStorage (net 3) [net 3].

The wire never carries the player id of another player. The id is a bearer value: `judge_hello` gives a held
slot to any `Hello` with that id (`session.rs:154-156`). Today only `Hello` carries it
(`crates/turts_net_protocol/src/messages.rs:20-24`). Lobby and result records name seats and slots with labels
only [backend M5].

#### 6.7.8 Connectivity, surrender and victory as sim inputs (decision D-D)

The engine supplies facts. The mod data decides what they do. With no clause, nothing happens (D28, S:57)
[backend H4] [net 2].

- The room writes `SlotConnection { slot, connected }` into the command log at the tick of the change: the
  first sim tick that starts after the server sees the change. It goes through the `CommandQueue`, at the
  same place in the fixed schedule as admitted orders and `--command-script` entries (`session.rs:117-122`,
  `crates/turts_server/src/command_script.rs:41-44`).
- `connected: false` is written when the connection of a claimed slot ends. `connected: true` is written when
  the owner reconnects. A claimed slot starts as connected, so the start writes no record. A slot that is never
  claimed has no record and is never counted as disconnected.
- "Disconnected" means that the server ended the session. The server notices a lost client at the netcode
  timeout, `client_timeout_secs: 10` (`crates/turts_client/src/join.rs:345`). Probe of this repair
  (`scratchpad/be-repair/lag.sh`, debug build, load 13): after a native client ended, a same-id `Hello` was
  refused until 9.1 s (clean exit) and 9.5 s (SIGKILL), and accepted at 9.8 s and 10.0 s. The gate 1 figure of
  14.5 s came from one-second tries that the server partly accepted (`scratchpad/be-probe/server4.log` has
  three `took slot 0` lines), so it is not a clean measure. NOT TRACED: whether a browser reload or a tab close
  sends a clean close, which would make the notice immediate [backend M7].
- What the lag bounds. Call it L (at most about 10 s):
  - The lobby leave text shows at most L after the real loss. LOB1 allows 11 s.
  - The disconnect count starts when the server notices. The defeat comes at most L + N after the real loss
    (130 s for `frontier`). The alert "Player 2 lost connection." and the banner "Waiting 1:59 for Player 2 to
    return." (the words of 7.6; the count starts at 2:00) show from the tick of `connected: false` (`MatchState.connections[].since_tick`,
    6.7.13), and the client counts the banner down in match time from that tick, so every window that shows it
    shows the same count.
  - A reload before the server notices gets `This game is open in another window`. The browser retries every
    1 s for 15 s (`rejoin.rs:5-6`). 15 s is more than L + 1 s, so the reload gets its slot. If a later change
    makes L larger than 13 s, step 19 breaks. LOB1 and MAT1 record L.
- Surrender is an order: an envelope with a mod order of behaviour `Surrender`, no units and no target. It goes
  through the normal order path, the token bucket and the command log. It defeats the slot at the tick of the
  order with the reason `Surrendered`. A mod with no `Surrender` order cannot surrender (`unknown_order`).
- The disconnect rule is the mod clause `DisconnectedFor { secs }`. The sim counts it in ticks: `secs x tick_hz`
  from the sim `MatchConfig`. `frontier` writes 120 (decision P8 b). The test package writes 5 (section 6.7.12).
- The outcome check runs once for each tick, after all removals and after the death path, before `Collect`.
  At the end of tick t, for each slot with no outcome, a clause fires when:
  - `KeyUnitsLost`: the slot has no live, completed unit of a listed type. The sim keeps a count for each slot
    and changes it at spawn, at completion and at death. It does no world scan for each tick (perf H2).
  - `Surrender`: the slot surrendered in tick t.
  - `DisconnectedFor`: the last connection record of the slot is `false`, from tick d, and t - d is at least
    the limit.
  A slot with a fired clause is defeated in tick t. The reason is the first fired clause in the mod's list
  order. Then, with teams (team = slot in MP1, decision P2 a):
  - More than one team still has a slot with no outcome: the slots defeated in tick t get `Defeat`.
  - One team: the slots defeated in tick t get `Defeat`, and each slot of that team gets `Victory` with the
    reason `OpponentsDefeated`. The match ends at tick t.
  - No team: the slots defeated in tick t get `Draw` (the teams alive at the start of the tick, section 5).
    Slots defeated earlier keep `Defeat`. The match ends at tick t.
- A defeated player keeps the slot (D25) [rigour 12]. What happens to the units of a defeated or disconnected
  slot is mod policy (S:354). In MP1 a defeat of one of two teams ends the match.
- The end. The end tick is t. The sim inserts `MatchStopped { reason: MatchOver }` (a new `StopReason`,
  `crates/turts_sim/src/economy/events.rs:183-197`) after the `Collect` chain of tick t. So tick t is
  published, and no later tick runs (`plugin.rs:151`, `:157-160`).
- Replay. The outcome depends only on the command log, the initial state and the tick. A replay of the command
  log gives the same outcomes and the same end tick (G9, invariant 15, S:581). A server timer that counts wall
  time breaks this; row MAT1 has that red.

#### 6.7.9 The result and the statistics (decision D-K)

- Each slot gets an outcome (`Victory`, `Defeat`, `Draw`), a reason (`KeyUnitsLost`, `Surrendered`,
  `Disconnected`, `OpponentsDefeated`), and the tick of the outcome. The result screen names the reason of the
  result in words (7.6): for example "Player 2 surrendered", "Player 2 left", "Command centre destroyed"
  [player 9].
- No rematch button in MP1 (D-K). "Leave" says how to start the next match (section 6.7.10).
- The statistics for each slot: units trained, units lost, units destroyed, structures completed, structures
  lost, credits gathered, and the match time (end tick - start tick + 1, in ticks). The sim counts them from its
  events: `Destroyed` with the cause `Killed { by }` gives "units destroyed" to the owner of `by`
  (`events.rs:69-73`, `:100-103`). NOT TRACED: that the owner of `by` is known when the event is written (the
  killer can die in the same tick); the plan checks it. NOT TRACED: the name of the credit delivery event.
- The statistics are sim state. They go on the wire only in `MatchResult`, after the end tick, because before
  the end they leak enemy state. `MatchState` (public, in the deltas) carries outcomes and connections, and no
  statistics [net 10].
- The result screen reads only `MatchState` and `MatchResult`. It never works out a result from the unit list:
  under fog (M5) the unit list is partial (`docs/research/mp1/input-technical-director.md:214-216`). Whether the mod or the shell owns the screen
  follows D70, which is Proposed (S:99).

#### 6.7.10 Leave and the supervisor (decisions D-C and D-K)

- "Leave" sends `LeaveMatch` and shows the shell text: "The match is over. Reload this page to join the next
  match." The server marks the slot as left and kicks the connection after `KICK_DELAY`, so the server does not
  wait for the 10 s timeout.
- Ended ends with exit code 0 at the first of:
  - no connection with a `Session` remains;
  - `after_match_secs` of wall time after the end tick (a new `server.ron` field; default 60).
- A reload before the old process has ended reaches the old process. An owner that left, and any new id, get
  `JoinRejected` `The match is over`. The client shows: "The match is over. Reload this page in a moment to
  join the next match." The words can change in 7.6; the meaning stays.
- The supervisor for MP1 is the new script `scripts/mp1-serve.sh <server.ron> [turts_server options]`. It runs
  the release `turts_server` in a loop and does not build it:
  - exit 0: it prints `mp1-serve: match <n> ended; starting match <n+1>` and starts the server again at once;
  - any other exit: it prints `mp1-serve: turts_server exited with <code>; stopped` and exits with that code,
    so a crash does not loop;
  - SIGINT or SIGTERM: it stops the server and exits with 0.
  The play test (section 13) runs the server with this script. Each new process serves the page again from
  `static_dir`.
- The restart gap is the boot time: about 1 s for `muster` in a debug build, 42 MB resident (gate 1 backend
  L10). `docs/measurements/mp1.md` records the boot-to-Lobby time for `frontier` in release [backend L10]. A
  reload in the gap gets the existing fetch error, and a second reload works.

#### 6.7.11 Persistence and the limits that keep M9 and M10 cheap

MP1 keeps no data between processes (no SQLite, D40; no save, D41). The result exists only until the process
ends. These rules hold in MP1 so that a save (M10) and rooms (M9) need no rewrite [backend L9]:

- No ownership outside `Slots`. The room drops its seat table at the start step. After the start, every
  question "which player owns slot s" reads `Slots`.
- No outcome, connection state or match timer in a `turts_server` resource. They are sim state (the sim
  `MatchState`), made only from the command log and the tick, so a save holds them. The `Shutdown` timer and
  the `after_match_secs` timer are process policy, not match state, and a save does not hold them.
- No configuration change after the start (S:308). A lobby action after the start gets `not_in_lobby`.
- One serializable `MatchConfig`, frozen at the start step: `{ lockfile_hash, tick_hz, start_tick, slots:
  [{ slot, team, colour }], settings }`. It holds no player id. The sim gets the part without colour
  (`{ tick_hz, start_tick, teams, settings }`), because colour never enters the sim. An M10 save stores
  `MatchConfig`, `Slots`, the sim state and the lockfile. M10 decides whether a restore writes
  `connected: false` for each slot.
- `settings` is an empty list in MP1; the mod settings schema is M9 (MD12, S:314).
- MP1 adds no new output of a player id. The existing stderr line `turts_server: player <id> took slot <n>`
  (`session.rs:264-267`) stays until D50 replaces the bearer id.
- Hosting (D58): the sim does not tick in Lobby and Countdown, so a waiting server costs almost nothing. The
  outcome check costs O(slots) for each tick with the counts above [backend L10].

#### 6.7.12 Test hooks

In the style of the existing hooks (`crates/turts_client/src/actions.rs:20-46`, `:107-118`; the options in
`CLAUDE.md`) [backend M8]:

- Server options:
  - `--start-now`: start at boot as under `Immediate` (section 6.7.4).
  - `--input-log PATH`: write every sim input that the server admitted, with its tick, as one JSON array in the
    `--command-script` format. A replay is `turts_server <server.ron> --start-now --command-script PATH`. RLY1
    uses the same file (6.5).
  - `--command-script` gets a second entry kind: `{"tick": T, "slot": S, "connection": false}`. Surrender is an
    order envelope and needs no new kind.
  - `--stats` gets `room_state` and `match_over_tick`.
  - `server.ron` gets `all_disconnected` and `after_match_secs`. The help test reads the fields from
    `config.rs` (`crates/turts_server/tests/help.rs:1-3`), so both fields go into `--help`.
- Headless client:
  - `--actions` kinds: `ready`, `unready`, `set_colour` (new field `colour`, a palette index), `surrender`,
    `leave`, and the test-only `lobby_move` (one `MoveTo` envelope for the listed `units` to `world_point`,
    sent through the normal send path with no interaction core, to test `match_not_started`).
  - Timing. A match action keeps `tick`: ticks after the `Welcome` tick (`actions.rs:117-118`). There is no
    `Welcome` tick in the lobby, so a lobby action (`ready`, `unready`, `set_colour`, `lobby_move`) and `leave`
    use `on: { event, wait_ms }`: the action runs `wait_ms` after the first state in which `event` holds, after
    the previous action ran. Events: `seated` (`you` is set), `full` (every seat has a human), `countdown`,
    `waiting`, `result` (`MatchResult` received). An action with both `tick` and `on` is a parse error.
  - `--rejoin` (requires `--player-id`): the headless client uses the stored-identity rejoin controller. Today
    only a client with a window does (`crates/turts_client/src/main.rs:38-43`); a headless client sends one
    `Hello` and exits with 1 on a refusal (gate 1 probe 3).
  - New `--out` report fields: `lobby_states` (each `LobbyState` with its receive time in ms), `lobby_refusals`,
    `welcome` (`tick`, `start_tick`, `teams`, `colours`), `match_states` (each change, with its tick), `outcome`
    (the own slot: `outcome` and `reason`, or null), `match_result` (or null), `match_over_tick` (or null),
    `stats_first_seen_tick` (the tick of the first received record that holds statistics, or null).
- The disconnect limit for tests is mod data in a test package, not an engine override. The test package
  `mods/standoff` (Game, `^0.2.4`; its ids pass the NC1 scan, checked when it is written): two slots, one key
  unit and one armed unit for each slot, symmetric, so that one script tick of attack orders kills both key
  units in one tick; `start: AllSeatedReady(countdown_secs: 1)`; `defeat: [KeyUnitsLost(..),
  DisconnectedFor(secs: 5)]`; one `Surrender` order. At 20 Hz the limit is 100 ticks.

#### 6.7.13 Records for the wire table (section 6.8)

| Record | Direction, channel | Fields | Who gets it |
| --- | --- | --- | --- |
| `LobbyState` | server to client, reliable | `revision: u32`, `you: Option<u8>`, `seats: Vec<Seat>`, `settings: Vec<SettingEntry>` (empty in MP1), `start: Waiting \| Countdown { remaining_ms: u32 }` | room members |
| `Seat` | part of `LobbyState` | `index: u8`, `occupant: Open \| Human { label: String } \| Bot { content: String }`, `ready: bool`, `colour: u8` | room members |
| `LobbyAction` | client to server, reliable | `revision: u32`, `action: SetReady(bool) \| SetColour(u8)` | - |
| `LobbyRefused` | server to client, reliable | `revision: u32`, `reason: colour_taken \| colour_unknown \| lobby_changed \| not_in_lobby` | the sender |
| `Welcome` (changed) | server to client, reliable | today's fields (`messages.rs:26-31`) plus `start: MatchStart` | the slot owner |
| `MatchStart` | part of `Welcome` | `start_tick: u32`, `teams: Vec<u8>`, `colours: Vec<u8>` (index = slot) | the slot owner |
| `MatchState` | in the delta, on change | `connections: Vec<{ slot, connected: bool, since_tick: u32 }>`, `outcomes: Vec<Option<{ outcome, reason, tick }>>` | public |
| `MatchResult` | server to client, reliable, once | `end_tick: u32`, `slots: Vec<{ slot, label, outcome, reason, tick }>`, `stats: Vec<{ slot, units_trained, units_lost, units_destroyed, structures_completed, structures_lost, credits_gathered }>`, `duration_ticks: u32` | public, after the end tick only |
| `LeaveMatch` | client to server, reliable | none | - |
| `JoinRejected` (new reason) | server to client | `The match is over` | the sender |
| `CommandRejectReason` (new) | in `OrderResult` | `MatchNotStarted`, `SlotDefeated`, `MatchOver` | the sender |
| `SlotConnection` | sim input in the command log, not on the wire | `slot`, `connected` | - |

#### 6.7.14 Decisions for the CEO from this section

1. `all_disconnected` default: `Continue` (recommended: with the mod clause the match ends by itself) or
   `Shutdown(after_secs)`.
2. `AllSeated` (the D23 automatic start): build it in MP1 (one more LOB1 case) or keep it for M9 with the
   creator start (recommended: MP1 has no room creation, and `frontier` uses `AllSeatedReady`).
3. Ended: exit when no session remains, or after `after_match_secs` = 60 (recommended), or only after the time.
4. The disconnect notice L: keep the 10 s netcode timeout (recommended for MP1), or lower it (a network-seat
   change that also changes MC 10.5).

### 6.8 Wire and API (X19)

**One protocol step, 4 to 5** (`NET_PROTOCOL_VERSION`, `crates/turts_net_protocol/src/lib.rs:13`), for all of MP1.
Every record goes through the per-slot export (`crates/turts_server/src/export.rs:25`, `:106`). The visibility
filter is in one place: a per-viewer visible-unit set in export, used for units, positions, healths, sites and
effects. In MP1 the set holds every unit [net 6].

**The record list and the visibility class of each record** [net 6] [backend M5] [backend M6]:

| Record | Message and channel | Who gets it | Sent |
| --- | --- | --- | --- |
| Site existence and footprint: `UnitSnapshot.site: bool`; `sites_completed: Vec<UnitId>` | Snapshot, WorldDelta (state) | Public | at spawn and at completion |
| `SiteProgress{unit, done, total, per_tick, at_tick, state, charged}` (`state`: the job state of 1.1) | WorldDelta | Owner | on change of rate or job state |
| Production job records, the same form as `SiteProgress` | WorldDelta | Owner | on change of rate or job state |
| `ConstructionQueue{constructor, orders: [(content, point)]}`: the held orders | WorldDelta | Owner | on change |
| `SupplyRecord{slot, resource, provided, consumed}` | WorldDelta | Owner | on change |
| `WeaponRecord` with `ready_tick` in place of `cooldown` | WorldDelta | Owner (as today, `export.rs:174-177`) | on change |
| `UnitOrderRecord.rally: Option<RallyTarget>` | WorldDelta `unit_orders` | Owner (order record, `export.rs:96-100`) | on change |
| `unit_orders` changes and `orders_cleared: Vec<UnitId>` | WorldDelta | Owner | on change; the Snapshot keeps the full table |
| `effects`: `Fired{unit, target, weapon: u8}`, `Destroyed{unit}` | WorldDelta | each client that may see the unit (all clients in MP1) | at the tick |
| `effects`: `Credited{unit, resource, amount}` (a delivery or a refund at a unit, for the "+N" text) | WorldDelta | Owner | at the tick |
| `healths` | WorldDelta (as today) | Public in MP1 (`export.rs:180`) | on change |
| `LobbyState{revision, you, seats, settings, start}`; a seat is `{index, occupant: Open, Human{label} or Bot{content}, ready, colour}` | reliable | room members | on change |
| `LobbyRefused{revision, reason}` (`colour_taken`, `colour_unknown`, `lobby_changed`, `not_in_lobby`) | reliable | the sender | for each refused lobby action |
| `Welcome` gains `start: MatchStart{start_tick, teams, colours}` (today it has `slot_id`, `tick`, `tick_hz`, `match_id`: `crates/turts_net_protocol/src/messages.rs:26-31`) | reliable | the slot owner | at each `Welcome` |
| `MatchState{connections: [{slot, connected, since_tick}], outcomes: [Option<{outcome, reason, tick}>]}` | WorldDelta, on change (the Snapshot holds it) | Public | on change |
| `MatchResult{end_tick, slots, stats, duration_ticks}`: the statistics | reliable, once | Public, and only after the end tick | once |
| `JoinRejected` with the new reason `The match is over` | reliable | the sender | at each refused `Hello` |
| New refusal reasons: the `site_*` keys of 6.1, `match_not_started`, `slot_defeated`, `match_over` | reliable `OrderResult` | the sender | for each envelope |
| Client to server: the order target `ContentAtPoint{content, point}`; `LobbyAction{revision, action}`; `LeaveMatch` | reliable | the server | for each action |

The fields of the lifecycle records are in 6.7.13, which is the authority for them. There is no `MatchStart`
message of its own (it is a part of `Welcome`), no `SlotConnection` on the wire (it is a sim input in the
command log, 6.7.8; clients see the connection state in `MatchState.connections`), and no `Surrender` message
(surrender is an order envelope with a mod order of behaviour `Surrender`, 6.7.8).

- A seat never carries a `player_id`. The id is a bearer value (`session.rs:152-156` gives a held slot to any
  `Hello` with the id), and today only `Hello` carries it (`messages.rs:21`) [backend M5].
- Message order: with a lobby, `Hello`, then `LobbyState`, then `Welcome` and `Snapshot` at the match start;
  under `Immediate`, `Hello`, then `Welcome`, as today. The lifecycle and the claim boundary are in 6.7
  (decisions D-B, D-C, D-D) [backend M6] [net 1].
- `MatchState.outcomes` names the reason of each outcome (key units lost, surrender, disconnect, draw), so the
  result screen can show "Player 2 surrendered" (decision D-K) [player M9].
- The statistics are in no message before the end tick: only `MatchResult` carries them. A wire check asserts it
  (row RES1), because a pixels driver cannot see data that the client holds and does not show [net 10].

**`Fired`** [net 7] [perf M1] [mod M6]. `Fired{unit, target, weapon}` has the weapon index on the unit and no
string. It drops the `damage`, `ammo` and `applied` fields of the sim event (`economy/events.rs:85-92`), and it
leaves out any id that the viewer may not see. Measured by the network lens: 6 B with an index against 20 B with a
content id.

**Records with a start tick and a rate** [perf M1]. A site, job or weapon record carries a value at a tick
(`done` at `at_tick`, or `ready_tick`) and a rate (`per_tick`). It is sent only when the rate or the job state
changes, not each tick. The client evaluates it at the tick of its last applied delta, never later, so this is not
prediction. Measured by the performance lens: `WeaponRecord.cooldown` changes every tick, about 500 records for
each tick at 1,000 units in combat.

**`unit_orders` as changes** [net 4] [perf M1]. Today each delta carries the full owner table
(`export.rs:190-195`); it is 64 to 83 % of the delta (network lens) and 77 % of the moving delta at 1,000 units
(performance lens). In protocol 5:

- A delta carries the complete record of each unit whose record changed, from the writer's change list (section
  9), plus `orders_cleared` for each unit whose record left the view.
- A record leaves the view when the sim gives no record for the unit: `views.rs` returns none for a unit without
  `UnitOrders` (`crates/turts_sim/src/economy/views.rs:558-569`). Today the client clears such a unit only because
  each delta replaces the whole table (`crates/turts_client/src/replica.rs:523-525`).
- The Snapshot keeps the full table. The replica merges deltas in chain order only.
- Row ORDW1 (section 12) plants the trap.

**Effects are never replayed.** Effects ride the unreliable state channel
(`crates/turts_net_lightyear/src/protocol_plugin.rs:20-25`). After a resync the client applies the kept deltas in
one commit. The client presents an effect only from a delta that it applies in normal order; a delta that it
applies in a resync batch gives no effect, and the client also drops an effect whose tick is older than the
presentation tick minus K (8.3; K = 4 ticks, Proposed) [net 7]. Nothing in the state depends on an effect:
removal and health are the state. A `Credited` effect gives only the "+N" text (7.10); the credits counter reads
the replicated balance.

**The rate-limit bucket runs first for every envelope** [net 5]. Today a `BadSequence` refusal and
`production_precheck` come before the bucket (`crates/turts_server/src/session.rs:403-418`, bucket at `:419-424`),
so a refused envelope costs no token. In protocol 5 the bucket runs first. No new bucket class. Engine orders
(6.5) do not pass admission.

**No client prediction** [net 11]. A grep finds no prediction in the client; progress comes from replicated
values. The replica is the only source of credits, progress, supply and outcomes (D18). A bar never goes past the
value of the last applied delta. The pending site is the one predicted object: it shows from the click until the
reliable `OrderResult`.

**One mod API step, 0.2.3 to 0.2.4** (`ENGINE_API_VERSION`, `crates/turts_mod_api/src/lib.rs:52`), additive. The
full table is section 6.10.

**Lane 0 freezes the shapes first** (section 10): every schema field, version-table row and wire record of 6.8
and 6.10, with the writers empty. A 0.2.4 feature that its lane has not built yet is refused by the packer from
one list, `UNBUILT_FEATURES`, with the text "`<feature>` is not built in this engine build". Each feature lane
removes its own line. So the feature lanes do not all edit `schema.rs`, `version_table.rs` and `messages.rs`.

### 6.9 Units and buildings in the way (decision P9)

Probe MP1-P1 measured today's behaviour (`docs/research/mp1/probe-obstacles.md`; native, deterministic, the
same result in two runs). Result: PASSABLE WITH LIMITS.

- One building is passable. One unit and twenty units pass a static footprint of radius 2.5 at every offset;
  they arrive 1.00 to 1.22 times later than on a free line. A click on a building ends on a ring round it.
- A compact base traps units. With a gap between footprints that is smaller than a unit's diameter, 20 to
  50 % of tanks stay `Blocked` inside the base, and a lone tank also. With the 0.5 gap of the first content
  proposal, 7 of 20 infantry stay too. With every gap at least 2.0, 0 units are `Blocked`.
- A building near the drop-off stops the economy. A power station 1 to 2 from the command centre on the ore
  side gives 0 harvester trips: the harvester stops `Blocked` and its gather order is cleared
  (`gather.rs:1126-1149`, `orders.rs:785-819`).

MP1 takes two measures, and no route search:

1. Engine (small, generic, stays valid at M4): the approach point of a receiver or a node is the nearest ring
   point that no static footprint blocks. About 40 lines in one file with one test (estimate of the probe,
   NOT TRACED). This changes a behaviour that has no field. It applies to a package of edition 0.2.4 only
   (6.10 row T1), so the frozen M2 stream (package `base`, `^0.2.3`) and every MC package keep their values.
2. Mod placement rule, as data: `placement.gap`, at least the largest mobile diameter plus 0.2 (2.0 for the tank of
   the proposal); the packer refuses a smaller gap (6.1, 6.10 row K2). The cost: bases are wider, and a player
   cannot build a wall.

**Entry condition from the performance lens** [perf H4]. Slice part A3 (two neighbour grids) is an entry condition
for any MP1 content with a footprint above the mobile radius. The command centre (2.5), the power station (1.5)
and the turret (1.0) are above it (infantry 0.4, tank 0.9). Measured by the performance lens on a loaded host
(ratios hold, absolute values about 2x high): one static unit of radius 2.5 raised the tick of 5,000 movers from
42.7 to 195.5 ms (4.6x), and of 1,000 movers from 6.9 to 11.0 ms. The slice plan already puts A3 "before MP1 content
adds a command centre or a long-range tank" (slice plan section 5, row E3). So no frontier package with these
footprints merges, and no MP1 measurement uses one, before A3 merges (section 10).

Recorded limit, as MC ruling O14: MP1 has no route round a group of obstacles. A route over the static
footprints is the first step of M4 navigation (the M4 design, `2026-10-28-m4-terrain-design.md:178-191`), not
of M4b. A `Blocked` unit in physical escape can walk up to 1.154 into a footprint (MC design 6.5 permits it);
a player can see it; it stays until M4b.

Decision P9 for the CEO: accept these two measures (recommended), or bring the M4 route step into MP1 (400 to
700 lines in 5 to 7 files, estimate of the probe; it moves the first match later).

### 6.10 Public mod API 0.2.4

Status: D15 (Baseline): mods target a versioned engine API, never Bevy entities, schedules or renderer details.
S:207-211 (5.2) and S 5.5: manifests declare engine API compatibility. Decision D-I: this table is the full step
from 0.2.3 to 0.2.4. Lane 0 builds it; gate 1 checks it [mod H1]. Every type below is a number, a string, a
content id, a list or an enum of these. No Bevy type crosses the boundary.

**Conventions for every row.**

- Content references are qualified ids (`frontier:tank`). Local ids follow the existing local id rule.
- Error text form, as today: `<package>/<file>: <field path>: <message>` (`ModError` with package, file, field;
  for example `frontier/units/command_center.ron: removal: a unit with a queue cannot be removed`).
- Every name lookup (content reference, damage type, armour class, reason key, cursor context, state key, action
  name, model node) that fails adds the list of allowed names and the hint "; did you mean `<name>`?" for the
  nearest name at edit distance 2 or less, with the rule of `crates/turts_mod_runtime/src/parse.rs:23-29`. Today a
  map unit reference has no list and no hint [mod M7].
- A new struct field is optional in RON (`#[serde(default)]`). The required fields of today stay required.
- `Ratio` is `(numerator: u32, denominator: u32)`, the shape of `RefundRatio` today (`schema.rs:404-407`);
  `denominator` is above 0.
- Each new export group and each new id kind is added to the no-content scan kind list
  (`crates/turts_checks/tests/support/mod.rs:86-100`), so NC1 can plant one id of each new kind [mod H3].

**Version-table detection** (decision D-I) [mod M4]:

- RAW: the field is written in the source file. The packer parses each file a second time as `ron::Value` and walks
  its keys. Probe (this repair, ron 0.12.2, the workspace version, `Cargo.toml:15`): `ron::Value` keeps the keys of
  `armour: None` and `orders: []`, and it drops enum variant names (a unit variant becomes `Unit`). So RAW finds
  fields, not variants. RAW applies to every row, the 0.2.2 and 0.2.3 rows included; today `^0.2.2` with
  `orders: []` packs, and a 0.2.2 engine refuses that file (mod-builder gate run). No repo package changes: all
  of them require `^0.2.3` (`mods/*/manifest.ron`).
- VAR: the parsed value holds the new variant. Rule: no default produces a new variant, so a parsed variant is a
  written variant.
- COMBO: a combination of written fields on one parsed unit, as the `weapon with movement` row of today
  (`version_table.rs:52-56`).
- ED (edition): the root package's edition is the lowest released API that its `engine_api` requirement admits.
  An ED row selects a behaviour for edition 0.2.4 and refuses nothing. A `^0.2.3` package keeps the 0.2.3
  behaviour.
- A package with no 0.2.4 field keeps the 0.2.3 behaviour. The changed behaviours with no field are listed in rows
  T1 to T4; idle acquisition and retaliation (TGT2) are not among them, because they need the `acquire` field
  (row U4).
- VAL1 needs one red package for each RAW row (with an explicit `None` or `[]`), for each VAR row and for each COMBO
  row, and one `^0.2.3` and `^0.2.4` pair for each ED row.

**Exports**

| # | File | Field | Type | Default when absent | Validation | Error text | Version row and detection |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A1 | `manifest.ron` | `exports.combat` | list of paths | `[]` | each file exists and parses; one damage table in the resolved package set | `frontier/manifest.ron: exports.combat[1]: a match has one damage table; combat/damage.ron already declares it` | `exports.combat` since 0.2.4, RAW |
| A2 | `manifest.ron` | `exports.match` | list of paths | `[]`: no victory clause, start policy `Immediate`, no site cap | at most one file in the resolved package set; the root Game package owns it; the file type is `MatchDef` (6.7.4) | `frontier/manifest.ron: exports.match: at most one file, found 2` (6.7.4) | `exports.match`, RAW |
| A3 | `manifest.ron` | `exports.visual_effects` | list of paths | `[]` | each file exists and parses | as A1 | `exports.visual_effects`, RAW |

**Damage table** (file of `exports.combat`, type `CombatDef`)

| # | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | `damage_types` | list of local ids | required | at least 1; unique | `frontier/combat/damage.ron: damage_types[2]: duplicate damage type rocket` | covered by A1 |
| C2 | `armour_classes` | list of local ids | required | at least 1; unique | as C1 | covered by A1 |
| C3 | `cells` | list of `(damage: ContentRef, armour: ContentRef, ratio: Ratio)` | required | each reference resolves; each pair of type and class has exactly one cell | `frontier/combat/damage.ron: cells: no cell for damage frontier:rocket and armour frontier:structure`; `cells[4].damage: unknown damage type frontier:rockt; did you mean frontier:rocket?`; `cells[7]: second cell for frontier:bullet and frontier:vehicle` | covered by A1 |
| C4 | `minimum` | u64 | required | none | parser text `missing field minimum` | covered by A1 |

**Unit, weapon and supply fields** (files of `exports.units` and `exports.resources`)

| # | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- |
| U1 | `armour` | `Option<(class: ContentRef, flat: u64)>`; `flat` defaults to 0 | `None`: no class | class resolves; required for a unit with `health` when the package has a damage table | `frontier/units/tank.ron: armour: a unit with health needs an armour class, because the package declares a damage table` | `armour`, RAW |
| U2 | `weapon.damage_type` | `Option<ContentRef>` | `None`: the damage of today | resolves; required when the package has a damage table | `frontier/units/rifleman.ron: weapon.damage_type: unknown damage type frontier:bulet; did you mean frontier:bullet?` | `weapon.damage_type`, RAW |
| U3 | `weapon.damage` (exists) | u64 | required | at least 1 (today 0 packs) | `frontier/units/rifleman.ron: weapon.damage: must be positive` | T3, ED |
| U4 | `acquire` | `Option<(range: f32, leash: f32)>` | `None`: the behaviour of today (fire only inside weapon range; no retaliation) | needs `weapon`; `range` finite and at least the weapon range; `leash` finite and at least 0 | `frontier/units/rifleman.ron: acquire.range: 4.0 is below the weapon range 5.0`; `acquire: needs weapon` | `acquire`, RAW |
| U5 | `provides`, `consumes` | list of `(resource: ContentRef, amount: u64)` | `[]` | resource is a `Supply` resource; amount above 0; one entry for each resource in each list | `frontier/units/power_station.ron: provides[0].resource: frontier:credits is not a Supply resource` | `provides`, `consumes`, RAW |
| U6 | `weapon.when_short`, `producer.when_short`, `constructor.when_short` | list of `(resource: ContentRef, effect: Rate(Ratio) or Disable or BlockStart)` | `[]`: not affected | `Supply` resource; `Rate` with 0 < numerator <= denominator, on producer and constructor only; `Disable` on weapon only; `BlockStart` reserved | `frontier/units/guard_turret.ron: weapon.when_short[0].effect: Rate applies to producer and constructor only`; `...: BlockStart is reserved for a later engine API` | `*.when_short`, RAW |
| S1 | resource `kind` | new variant `Supply(scope: Slot, counts: Completed or Queued)` | none | `scope` is `Slot` (M6 adds `Network`); `counts: Queued` reserved | `frontier/resources/power.ron: kind.counts: Queued is reserved for a later engine API` | `kind: Supply`, VAR |
| U7 | `income` (decision D-L, CEO to confirm) | `Option<(resource: ContentRef, amount: u64, period_ms: u32)>` | `None`: no income | `resource` is a `Balance` resource; `amount` above 0; `period_ms` above 0 and a whole number of ticks (the conversion of `try_ms_to_ticks`, `crates/turts_sim/src/boot_m3.rs:591`); the unit has `health`. The unit adds `amount` once in each period while it is complete and its health is above 0, as one ledger event (section 5, row INC1) | `frontier/units/command_center.ron: income.resource: frontier:ore is not a Balance resource` | `income`, RAW |

**Construction and payment**

| # | File | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- | --- |
| K1 | units | `constructor` | `Option<(buildings: [ContentRef], concurrent_sites: u16, queue_capacity: u16, progress: Autonomous or NeedsBuilder(reach: f32), when_short)>` | `None` | `buildings` are production ids whose output has `placement`; a production id is in at most one of `producer.recipes` and `constructor.buildings` in the package set; `concurrent_sites` >= 1; `queue_capacity` >= 1; `NeedsBuilder` reserved; beside `producer` allowed, beside any other work capability refused | `frontier/units/command_center.ron: constructor.buildings[0]: unknown production frontier:build_powr; did you mean frontier:build_power?`; `constructor.progress: NeedsBuilder is reserved for a later engine API`; `machine: a unit may have at most one work capability; constructor is allowed only beside producer` | `constructor`, RAW; `constructor beside producer`, COMBO |
| K2 | units | `placement` | `Option<(gap: f32, node_clearance: f32, in_build_area: bool, site_health: Ratio)>`; `site_health` defaults to 1/1 | `None`: the unit is not buildable | static units only; `gap` finite and at least the largest diameter of a unit with movement in the package set plus 0.2; `node_clearance` finite and at least 0; `site_health` numerator <= denominator | `frontier/units/power_station.ron: placement.gap: 0.5 is below 2.0, the largest mobile diameter (frontier:tank, 1.8) plus 0.2` | `placement`, RAW |
| K3 | units | `build_area` | `Option<f32>` | `None` | finite and above 0; if a `placement` has `in_build_area: true`, some unit has `build_area` | `frontier/units/power_station.ron: placement.in_build_area: no unit declares build_area, so no site can be placed` | `build_area`, RAW |
| K4 | production | `output_unit` of a building | existing | none | for a production id in `constructor.buildings`: the output is static and has `placement` (it lifts `content.rs:557` for these ids only) | `frontier/production/build_guard_turret.ron: output_unit: a building output needs placement` | `production with a static output`, COMBO |
| K5 | production | `cancel_refunds.destroyed` | `Ratio` | 0/1: no refund | numerator <= denominator | `frontier/production/build_guard_turret.ron: cancel_refunds.destroyed: the numerator is above the denominator` | `cancel_refunds.destroyed`, RAW |
| K6 | match file | `open_sites_per_slot` (field of `MatchDef`, 6.7.4) | `Option<u16>` | required when a unit in the package set has `constructor`; else `None`: no slot cap | at least 1; an order above the cap waits as a held order (6.1) | `frontier/match/match.ron: open_sites_per_slot: a package with a constructor needs open_sites_per_slot` | covered by A2 |
| K7 | units | lifted rule: `health` without `removal` | combination | death destroys the contents (`ContentsPolicy::Destroy`) | none | none | `health without removal`, COMBO |
| K8 | units | lifted rule: `removal` with a queue (producer, machine, research, constructor) | combination | destruction refunds by `cancel_refunds.destroyed` (6.4) | none | none | `removal with a queue`, COMBO |
| P1 | production | `payment: ChargeOverTime` | new variant of `PaymentPolicy` (`schema.rs:398`) | none | production entries only | `frontier/technologies/armour.ron: payment: ChargeOverTime is not supported for technologies` | `payment: ChargeOverTime`, VAR |

**Orders, repair and relations** (files of `exports.orders` and `exports.units`)

| # | File | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- | --- |
| O1 | order | `behaviour: Construct` | variant | none | target `ContentAtPoint`; each listed unit has `constructor` | `frontier/orders/order_construct.ron: units[0]: frontier:tank has no constructor` | `Construct`, VAR |
| O2 | order | `behaviour: CancelConstruction` | variant | none | target `None`; each listed unit has `placement` | as O1 | `CancelConstruction`, VAR |
| O3 | order | `behaviour: SetRally` | variant | none | target `Point`, `Unit`, `PointOrUnit` or `Node`; each listed unit has `producer` | as O1 | `SetRally`, VAR |
| O4 | order | `behaviour: Repair` | variant | none | target `Unit`; each listed unit has `repairer` | as O1 | `Repair`, VAR |
| O5 | order | `target: ContentAtPoint` | new variant of `TargetKind` | none | only with `Construct` | `frontier/orders/order_x.ron: target: ContentAtPoint needs behaviour Construct` | `ContentAtPoint`, VAR |
| O6 | `orders/rules.ron` | smart target `OwnDamaged`; `needs: Constructor` and `needs: Repairer` | new variants of `SmartTarget` and `Capability` | none | `OwnDamaged` with a `Repair` order; `Ground` with `needs: Producer` only for a `SetRally` order | `frontier/orders/rules.ron: smart[5].order: Ground with needs Producer needs a SetRally order` | each variant, VAR |
| O7 | `orders/rules.ron` | `rally` | `Option<[SmartOrder]>` | `None`: the follow order comes from `smart` | each order resolves | as the smart list of today | `rally`, RAW |
| O8 | production | `hotkey` | `Option<String>` (an action name) | `None`: the key of the producer's Produce order (today, `sources.rs:391-393`) | the action exists in the input map; the default keys of all actions are unique across all cards (the check is global today, UX F1) | `frontier/production/train_rifleman.ron: hotkey: unknown action key_rifelman; did you mean key_rifleman?` | `hotkey`, RAW |
| O9 | order | `behaviour: Surrender` (6.7.4, 6.7.8) | variant | none | target `None`; `units: []` | `frontier/orders/order_surrender.ron: units: a Surrender order lists no units` | `behaviour Surrender`, VAR |
| O10 | production | `card` | `Option<CardCell>` (the existing `CardCell{row, column}`, `crates/turts_mod_api/src/orders.rs:25-28`) | `None`: the entry is a row of the build-menu list, in `producer.recipes` order, as today (`crates/turts_client/src/widgets/sources.rs:395-397`) | two entries of one constructor or producer in one cell are refused (UX F2: two items in one cell overlap) | `frontier/production/build_guard_turret.ron: card: cell (0, 1) is also the cell of frontier:build_power_station` | `card`, RAW |
| R1 | units | `repairer` | `Option<(rate_per_second: u64, reach: f32, cost: [(resource, amount: u64, per_health: u64)], targets: TargetFilter, classes: [ContentRef], auto_range: Option<f32>)>` | `None` | `rate_per_second` > 0; `reach` finite > 0; `per_health` > 0; `amount` >= 0 and `cost: []` legal (free repair); `classes` resolve to armour classes, `[]` means all; `auto_range` finite and at least `reach`; not a work capability | `frontier/units/engineer.ron: repairer.cost[0].per_health: must be positive` | `repairer`, RAW |
| L1 | any target filter | `slots: Ally`, `slots: Enemy` | new variants of `SlotRelation` | none | none | none | each variant, VAR |

**Match file and map** (file of `exports.match`, type `MatchDef`; semantics in 6.7)

| # | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- |
| M1 | `victory` | `Option<VictoryRule>`; `VictoryRule::LastTeamStanding { defeat: [DefeatClause] }`; `DefeatClause::KeyUnitsLost { units: [ContentRef] }` or `DefeatClause::DisconnectedFor { secs: u32 }` | `None`: the engine never ends the match (today, D28) | `units` at least 1, resolve, have `health`; `secs` above 0; `DisconnectedFor` at most once. Surrender is an order (row O9), not a clause | `frontier/match/match.ron: victory.defeat[0].units[0]: unknown unit frontier:comand_center; did you mean frontier:command_center?`; the other texts in 6.7.4 | covered by A2 |
| M2 | `start` | `StartPolicy`: `Immediate`, `AllSeatedReady(countdown_secs: u32)`, `AllSeated(countdown_secs: u32)` | `Immediate`, as today (claim at `Hello`) [backend H3] | `countdown_secs` 0 to 30; `AllSeated` reserved (6.7.14 item 2); `AllSeatedReady` needs one different `slot_colours` entry for each player slot (row H15) | `frontier/match/match.ron: start: countdown_secs must be 0 to 30, found 45` | covered by A2 |
| M3 | map `placements[].start_order` | `Option<(order: ContentRef, target: Node(u32) or Point(WorldPos))>` (decision D-F) [player H1] | `None` | the order exists and lists the unit; the target kind matches the order; the node index is below the number of map nodes | `frontier/maps/twin_fields.ron: placements[1].start_order.target: node index 40 is outside the 18 map nodes` | `start_order`, RAW |

- With no `DisconnectedFor` clause, a disconnect changes nothing: D28 (Decided) keeps disconnect gameplay out of
  the engine; the engine gives only the connection facts (decision D-D) [mod L11] [backend H4].
- A mod with no order of behaviour `Surrender` cannot surrender: an envelope with an order id that the package
  does not define gets the existing reason `unknown_order` (6.7.8; `crates/turts_net_protocol/src/messages.rs:256`).
- No lobby settings schema is in 0.2.4: MD12 is `later:M9`, and S:314 (Baseline) stays for M9 [rigour 12]. The lobby
  colour is an index into `slot_colours` (row H15). Team choice is not in MP1 (decision P2 a).

**Interface** (file of `exports.interface`; UX part 8)

| # | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- |
| H1 | `reasons` keys | strings | — | each key is in `REASON_KEYS`: the wire reason keys (`turts_net_protocol/src/messages.rs`, `as_str`), the client keys of today (`no_applicable_order`, `queue_disabled`, `already_researching`) and the new keys of 6.1 and 6.8 | `frontier/interface/hud.ron: reasons.queue_ful: unknown reason key; did you mean queue_full?` [mod M8] | T2, ED |
| H2 | `cursors` keys | strings | — | each key is in `CURSOR_CONTEXTS`: `default`, `select`, `move`, `attack`, `gather`, `forbidden`, `place`, `repair`, `rally` | `frontier/interface/hud.ron: cursors.plcae: unknown cursor context; did you mean place?` [mod M8] | T2, ED; the keys `place`, `repair`, `rally`, RAW |
| H3 | `states` | map from state key to text | `{}`: the engine English text | keys in `STATE_KEYS`: `waiting_for_funds`, `slowed`, `disabled`, `output_blocked`, `held`, and `activity.<name>` for each wire `Activity`; a text can hold `{resource}` | as H1 | `states`, RAW; keys T2, ED |
| H4 | `alerts[].trigger` | new variants `ConstructionComplete`, `SupplyShort(ContentRef)`, `SupplyRestored(ContentRef)`, `InsufficientFunds(ContentRef)`, `SlotDisconnected`, `SlotReconnected`. Existing variants that 7.9 uses: `ProductionComplete` (a unit is ready), `GathererWaiting(f32)` (an idle gatherer), `OwnUnitDamaged`, `OwnUnitDestroyed` (`crates/turts_mod_api/src/interface.rs:387-399`) | none | `Supply*` names a `Supply` resource; `InsufficientFunds` names a `Balance` or `Item` | `frontier/interface/hud.ron: alerts[2].trigger: frontier:ore is not a Supply resource` | each new variant, VAR |
| H5 | `alerts[].units` | list of `ContentRef` | `[]`: all units | resolve | as M1 | `alerts.units`, RAW |
| H6 | `alerts[].kind`, `alerts[].clear_after` | `kind: Event or State`; `clear_after: Option<f32>` seconds | `Event`, the 0.2.3 behaviour (`crates/turts_client/src/feedback.rs:849-853`); `clear_after` 10 | `State` only with a state trigger (`SupplyShort`, `InsufficientFunds`, `GathererWaiting`, `SlotDisconnected`); `clear_after` finite and at least 0, and only with `State`. The rule of each kind is in 7.9 [player H3] | `frontier/interface/hud.ron: alerts[1].kind: State needs a state trigger; ProductionComplete is an event` | `alerts.kind`, `alerts.clear_after`, RAW |
| H7 | `Value` and `Source` | new variants `Supply(ContentRef, Provided or Consumed)`, `ConstructionQueue`, `Sites`, `Notices`, `SelectionTypes`, `Connection`, `Outcome`, `OutcomeReason`, `MatchStats`, `MatchClock`, `Hint` | none | references resolve | as M1 | each variant, VAR (walk of the parsed widget tree) |
| H8 | `PrimaryField` | `Activity`, `Cargo`, `Armour` | none | none | none | each, VAR |
| H9 | `ItemField` | `Role`, `BuildTimeNow` (the time at the current rate [player M7]), `SupplyChange`, `CausesShortage`, `State`, `Paid`, `Refund`, `Badge`. The existing `Time` keeps the base time, so no 0.2.3 layout changes | none | none | none | each, VAR |
| H10 | `Predicate` | `ItemHas(ItemField)`, `PrimaryHasValue(PrimaryField)`, `SupplyShort(ContentRef)`, `MatchOver`, `OutcomeIs(Victory or Defeat or Draw)`, `Placing`, `HintShown` | none | references resolve | as M1 | each, VAR |
| H11 | `world.placement`, `world.build_area_ring`, `world.gap_ring`, `world.progress_bar`, `world.cargo_bar`, `world.rally`, `world.float_text` | optional styles (colours `#RRGGBB`, sizes, font id). `placement` has `valid`, `invalid` and `stripes`; `gap_ring` also draws the clearance ring of a node; `float_text` has a `sound` id [player H4] | `None`: the engine style (valid `#56B4E9`, invalid `#D55E00` with stripes, white rings) | colours parse; font and sound exist | `frontier/interface/hud.ron: world.placement.invalid: not a #RRGGBB colour` | each, RAW |
| H12 | `world.health_bar.scale_to_footprint` | bool | `false` | none | none | RAW |
| H13 | `Button` `font`, `size` (X18) | `Option<String>`, `Option<u16>` | `None`: the tooltip font at 14 px (today, `widgets/mod.rs:627-633`) | font exists; size 8 to 64 | as the font check of today | `Button.font`, `Button.size`, RAW |
| H14 | `hints`, `hint_fallback` | `hints`: list of `(id, kind: Step or Situation, text, starts: HintCondition, done: HintCondition)`; `hint_fallback: Option<String>`. `HintCondition` (the condition list of 7.8): `MatchStarted`, `OwnUnits(units: [ContentRef], state: Pending or Site or Complete or Any, at_least: u16)`, `OwnJob(production: ContentRef)`, `OwnRally(unit: ContentRef)`, `OrderAccepted(order: ContentRef, target: Point or Unit or Any)`, `SupplyShort(ContentRef)`, `JobOnHold(secs: f32)`, `HintDone(id)`, `HintShownFor(id, secs: f32)`, `All([HintCondition])`, `Any([HintCondition])`, `Not(HintCondition)` | `[]`; `None`: no fallback line | ids unique; `HintDone` and `HintShownFor` name a hint of the list; references resolve; `secs` finite and above 0. The selection and done rules are in 7.8 (decision D-F) | `frontier/interface/hud.ron: hints[2].starts: HintDone(build_powr) names no hint; did you mean build_power?` | `hints`, `hint_fallback`, RAW |
| H15 | `slot_colours` (exists) | list of colours | required | with `AllSeatedReady`, at least one different colour for each player slot (the check and its text are in 6.7.4); the server refuses a colour index that another seat holds (`colour_taken`, 6.7.6) | `frontier/match/match.ron: start: AllSeatedReady needs 2 different slot colours in the interface, found 1` | none (existing field) |
| H16 | `effects_voices`, `audible_range` | `Option<u16>`, `Option<f32>` | `None`: 8 voices; `None`: no fall-off with distance | `effects_voices` 1 to 64: the cap of effect sounds that play at one time in the effects category; `audible_range` finite and above 0: the gain falls to 0 at this distance from the camera look-at point (8.4, AUD2) | `frontier/interface/hud.ron: effects_voices: must be 1 to 64` | `effects_voices`, `audible_range`, RAW |
| H17 | `alerts[].select_on_click`; the placeholders `{unit}` and `{slot}` in `alerts[].text` | `bool`; text | `false`; no placeholder | an unknown placeholder is refused | `frontier/interface/hud.ron: alerts[4].text: unknown placeholder {units}; did you mean {unit}?` | `alerts.select_on_click`, RAW; placeholders T4, ED |
| H18 | `start_selection` | `Option<ContentRef>` | `None`: no selection at the start (today) | resolves to a unit (7.8) | `frontier/interface/hud.ron: start_selection: unknown unit frontier:comand_center; did you mean frontier:command_center?` | `start_selection`, RAW |
| H19 | `UnitPresentation.role` (today `icon`, `portrait`, `responses`: `crates/turts_mod_api/src/interface.rs:410-415`) | `Option<String>` | `None`: no role line | none | none | `role`, RAW |

**Assets, visuals and effects** (RE parts 1 and 4; semantics in 8.1 and 8.3)

| # | File | Field | Type | Default | Validation | Error text | Row |
| --- | --- | --- | --- | --- | --- | --- | --- |
| V1 | assets | `models` | list of `(id, path)` | `[]` | GLB version 2; no `uri`; no `extensionsRequired`; no skins, animations, cameras or lights; at most 2 materials; the triangle and byte caps of decision D-M (8.1; one cap for each model, no class caps) | `assets/models/tank.glb: 9,412 triangles; maximum is 8,000 for one model` (8.1) | `models`, RAW |
| V2 | visuals | `model` | `Option<(asset, team_material: Option<String>, turret_node: Option<String>, muzzle_node: Option<String>)>` | `None`: the shape only | the asset exists; the named material and nodes exist in the GLB | `frontier/visuals/tank.ron: model.turret_node: the model has no node turet; did you mean turret?` | `model`, RAW |
| V3 | visual effects file | each effect | `(id, kind: Flash or Spark or Tracer or Burst or Smoke, colour, size: f32, ms: u32, sound: Option<String>)` | none | ids unique; sound exists | `frontier/fx/effects.ron: effects[1].sound: unknown sound asset fx_shot` | covered by A3 |
| V4 | visuals | `effects` | `Option<(fire, hit, hit_reduced, death: Option<String>, damaged: Option<(effect, below: Ratio)>)>`; `hit_reduced` is the hit of a table ratio below 1/2 (7.10, 8.3) | `None`: no effect; `hit_reduced` absent: `hit` | ids exist | as V3 | `effects`, RAW |

- S:247 (5.8, Baseline) keeps the exact supported glTF features Open. Rows V1 and V2 are the MP1 subset only,
  Proposed for MP1 [rigour 12]. The byte and triangle caps are a CEO decision (section 2, RE:24) [mod M9].
- RE part 6 (forward +Z, pivot at the ground centre, scale 0.8 to 1.2 of the footprint, the `team` material) is
  advice in section 8.1, not a pack check, except the named nodes and the material of row V2.

**Edition rows** (behaviours with no field)

| # | Behaviour | Edition 0.2.3 (and older) | Edition 0.2.4 |
| --- | --- | --- | --- |
| T1 | The approach point of a receiver or a node (6.9) | today | the nearest ring point that no static footprint blocks |
| T2 | Unknown `reasons`, `cursors` and `states` keys | pack (today) | refused (H1 to H3) |
| T3 | Weapon damage 0 | packs (today) | refused (U3) |
| T4 | `{unit}` and `{slot}` in an alert text | shown as written (today) | replaced; an unknown placeholder is refused (H17) |

**What this table chose where the inputs disagree** [mod H1]:

| Point | Inputs | Chosen | Why |
| --- | --- | --- | --- |
| Damage formula | design `max(min, floor(d x ratio) - flat)`; GD `max(1, floor(d x percent / 100))` | the design form (6.3) | it holds the GD form as a case |
| Home of `acquire` | weapon (design); unit (GD) | unit | one leash for each unit |
| Power | `Network` and `UnitDef.power` (GD); `Capacity` (design) | `Supply(scope: Slot)` with `provides` and `consumes` | one name for power and a population cap; `power` is a frontier id |
| Construction order | `Place` (GD, UX); `Construct` (design) | `Construct` | it says what it does; `Placement` is already a map type name |
| Structure queue | `lanes` (GD); named producer queues (design 7.3) | the constructor queue (6.1); no named producer queues in 0.2.4 | one home for each job (D-E); CEO list |
| Payment name | `ConsumeOverTime` (GD); `ChargeOverTime` (design) | `ChargeOverTime`, stage `WaitingForFunds` | the design name |
| Relations | Enemy, Ally, Own (GD); Ally, Enemy (design) | `Ally`, `Enemy`; `Same` is "own" | no duplicate of `Same` |
| Home of victory, disconnect, start policy | map `match` (GD); not stated (design) | the `match` export file | the map refuses `match` today; one file for the match rules |
| Alert names | `LowPower`, `NoCredits` (UX) | `SupplyShort(res)`, `SupplyRestored(res)`, `InsufficientFunds(res)` | no frontier words in the engine |
| Unit ready | `UnitReady` or `ProductionComplete` (UX) | `ProductionComplete` (exists) | no duplicate |
| Lobby palette | `slot_colours` (UX); "mod palette" (design) | `slot_colours` index | one list of colours |
| Match file shape (reconcile pass) | `victory` list with `DisconnectDefeat`, `start_policy: AllReady` (part 2); `MatchDef` with `DefeatClause` and `AllSeatedReady` (6.7.4, part 1) | 6.7.4 | the lifecycle seat owns the semantics; surrender is an order, not a clause |
| Placement reason keys (reconcile pass) | `site_too_close` (part 2); `site_too_far`, `site_near_building`, `site_near_ore` (part 3) | the list of 6.1 | `ore` is a `frontier` id; "too close" does not say to what |
| A state alert that repeats (reconcile pass) | `rearm` (part 2); `kind` and `clear_after` (7.9, part 3) | `kind` and `clear_after` | rows ALR2 and GOAL1 test the 7.9 rule |
| Idle gatherer trigger (reconcile pass) | `GathererIdle` (part 2); `GathererWaiting` (7.9) | `GathererWaiting(f32)` | it exists today (`interface.rs:398`) |
| Build time at the current power (reconcile pass) | `Time` changed (part 2); `BuildTimeNow` (7.5) | `BuildTimeNow` | a changed `Time` changes a 0.2.3 layout with no field |
| Power in the tooltip (reconcile pass) | `PowerChange` (7.5); `SupplyChange` (part 2) | `SupplyChange` | no `frontier` word in an engine name |

**Additive later, recorded now** [mod L12]: square footprints (a new `Footprint` variant), batches, a hold-a-point
victory clause, `counts: Queued`, `BlockStart`, `NeedsBuilder`, `Network` scope, named producer queues,
`consume_on_use` on a repairer. Each one is a new variant or an optional field, so a 0.2.4 package stays valid.

## 7. Player interface

This block is the whole section 7, with its subsections 7.0 to 7.10. Status: D65, D67 (line (a) and its constraint)
and D73 are Decided lines with Proposed designs; D66, D70 and D74 are Proposed (S:94-103). Every new item is an id, a
number or a string; the engine supplies widgets and the mod places them (D67). Each new engine item of this section is
for all mods. Section 6.10 (the API table of decision D-I) holds its field, type, default, validation, error text and
version-table row; 7.5 is the interface input to that table.

### 7.0 Terms

The terms of this section (player slot, lobby seat, team, team material, site, pending site, held order,
constructor queue, producer queue, job state, alert, notice, hint, done, Click to play) are in the one list of
section 1.1 [rigour 11]. The reconcile pass removed two leftovers of revision 3 from this list: the "placed order"
is now the held order of 6.1, and the queues have no ids (`structures` and `units` were queue ids; decision Q1).

### 7.1 The player path

The oracle list of the play test is the section `## Player path` at the end of this design: lobby lines L1 to L5,
the hint list H1 to H7, F and X1 to X3, and match steps 1 to 19. The testers never see it. The first three steps are
the lesson of the CEO play of MC on 2026-11-11: the goal is on the screen in one line, one hint line names the first
action and its key, the screen shows a reward when the action is done, and nothing sounds before the player acts.

The 60 s rule of PT3: the clock starts when the player presses "Click to play" in that window. It ends at the first
client update in which a site of an own Power Station exists, or a complete own Power Station (a server entity; a
pending site does not end it, and the hint H1 can be done before the clock ends, 1.1). PT3 is red when the end
comes later than 60.0 s, or never. The client records both times in `turts_debug.hints`
(`click_ms`, and for each hint `shown_ms`, `done_ms`, and for H1 also `site_ms`) [rigour 10].

### 7.2 Construction

- Start: select the Command Centre, then click a build entry or press its key (7.4). This starts placement on the
  client only; the server gets no order (GHO1). The left click sends `Construct{content, point}`.
- Ghost: the unit's own visual, translucent, with its footprint ring, in the slot colour.
- Rings while the player places [player H4]:
    - the build-area ring of the own Command Centre: a solid line in the slot colour at radius 12.5 (footprint 2.5
      plus `build_area` 10, 6.10 row K3);
    - a gap ring round each building and site within 15 of the pointer: a thin dashed line at its footprint plus
      `placement.gap`;
    - a clearance ring round each ore node within 15 of the pointer: a thin dashed line at its radius plus
      `placement.node_clearance`.
- The site check runs in the order of 6.1, with the reason keys of 6.1 (the one list). The first check that fails
  gives the reason; the ghost shows one reason only. Each reason is a reason key with a mod text (keys are checked at
  pack time: an unknown key is refused with a "did you mean" hint [mod M8]):

| Check | Reason key | Text at the pointer | What the ghost shows |
| --- | --- | --- | --- |
| A part of the footprint is outside the map | `site_outside_map` | "Outside the map" | stripes |
| The centre is farther than `build_area` from the edge of the own Command Centre | `site_out_of_area` | "Too far from your base" | stripes; the build-area ring |
| The footprint overlaps a unit, a building or a site | `site_blocked` | "Something is in the way" | stripes |
| No overlap, but the gap to a building or a site is below `placement.gap` | `site_near_building` | "Too close to a building" | stripes; the gap ring of the ghost and of that building turn vermilion, so the missing distance shows between them |
| The distance to the edge of an ore node is below `placement.node_clearance` | `site_near_resource` | "Too close to ore" | stripes; the clearance ring of that node turns vermilion |
| The constructor queue holds 5 held orders (`queue_capacity`) | `queue_full` | "Construction queue is full (5/5)" | stripes |

  An order above the site caps is not refused: it waits as a held order (6.1), so this table has no cap row.

- Valid: a solid ghost ring in the slot colour. Invalid: Okabe vermilion `#D55E00`, diagonal stripes, the `forbidden`
  cursor and the text. Colour is never the only signal.
- Input: left click places. Shift and left click places and keeps the ghost for one more. Right click or Esc leaves
  placement and never gives an order. No rotation (circles, `crates/turts_mod_api/src/schema.rs:28-30`).
- Preview against the server verdict [net 9]: the ghost uses the same pure rule as the server (`turts_rules`), with
  the client replica. The replica is at least one tick plus the latency old (gate1-network.md:78), so the client cannot
  see a unit that walks in, or a site that the other player places in the same tick. On a mismatch the player sees:
  the pending site; then the refusal; the pending site goes; the reason text shows at the site point for 3 s, with one
  error sound; the credits do not change; the hint does not advance (7.8). The client never shows a site before the
  server result (D18).
- Fog (M5): a reason never names a hidden blocker; the placement rule takes the visibility predicate of the target
  queries, which is always true in MP1 [net 6].
- A held order that waits for an open site shows a ghost outline with its number at its point ("2"); its queue row
  says "Waiting to start".
- Progress: a world progress bar on the site; the site model rises (8.2).
- Cancel: select the site and press C ("Cancel construction"), or right-click its queue row. The tooltip shows
  "Refund: N credits" before the click (N is what the site paid, times the mod ratio). After the server result, "+N"
  rises at the site.

### 7.3 Production

- The Command Centre has two queues that run at the same time, and neither has an id (decision Q1): the constructor
  queue (held orders, `queue_capacity` 5; one open site for each slot, `open_sites_per_slot` 1, mod data) and the
  producer queue (jobs, capacity 5). The left click of 7.2 carries the point, so a held order waits only for an open
  site; then the site owns progress, charge and job (decision D-E). There is no "ready to place" state.
- Each entry shows a count badge and a fill on its icon. A queue row shows the icon, a bar and the seconds left at the
  current power.
- Row states, each in words: "On hold: no credits" (a striped bar); "Slowed: low power"; "Waiting to start" (a held
  order); "Waiting: exit blocked" (a complete unit job with no free exit point). "On hold: no credits" shows from the
  first tick in which the job cannot pay until the job pays in every tick for 1 s, so it does not flicker under the free
  income.
- Pay over time (decision D-A): entries stay enabled while their queue has space; no cost blocks them at the order. The
  tooltip says "Cost N credits, paid while it builds". The row tooltip says "Paid N of M credits".
- A full queue disables the entry with "Queue is full (5/5)" (units) or "Construction queue is full (5/5)".
- Cancel: right-click a row, or right-click the entry (the newest job of that type). "+N" rises at the Command Centre
  with the refund sound (7.10). A waiting job or order refunds 0, because it paid 0.
- Rally: with the Command Centre selected, a right click on the ground sets the rally (a smart rule before Move), or L
  then a click. A flag and a dashed line from the Command Centre show while it is selected (owner only), with a pulse
  when it is set. A rally on ore makes new Ore Haulers gather; other new units attack-move to the flag.

### 7.4 Keys

The build menu needs one key for each entry; today every row shows the key of the producer's Produce order
(`crates/turts_client/src/widgets/sources.rs:391-393`). The key check is global (no action scope), so each MP1 default
key is unique across all cards. The action scope of MCD:312 is not built in MP1; the MC design text is corrected to
say so. All defaults are plain letters; none is in `RESERVED_BROWSER_KEYS` (`crates/turts_mod_api/src/input.rs:51-68`).

| Action | Key | Where |
| --- | --- | --- |
| Move | M | card |
| Stop | S | card |
| Hold | H | card |
| Attack, attack-move | A | card |
| Gather | G | card |
| Repair | R | card |
| Set rally | L | card |
| Cancel (selected site, or the newest job) | C | card |
| Power Station | P | build menu |
| Guard Turret | T | build menu |
| Rifleman | I | build menu |
| Launcher | V | build menu |
| Engineer | E | build menu |
| Ore Hauler | O | build menu |
| Tank | K | build menu |

Shift and a train key queues five. Esc leaves placement, else opens the menu. The camera, selection and group keys
stay as in MC; KEY1 checks the full list against them (the MC defaults: NOT TRACED here). Each tooltip names its key
("[P]"); a key with no binding shows "Unbound" (MCX1).

### 7.5 HUD sources and schema items

Input for the table of section 6.10 (decision D-I). The names below are the names of 6.10 (rows H4 to H19, O8, O10, M3,
U7); where the two differed, the reconcile pass took one name and changed both places (the table "What this table chose"
of 6.10). Each item is additive in API 0.2.4. Presence for the version table means that the field is written in the
source file (decision D-I). When an item is absent, the 0.2.3 behaviour stays.

- Start: `InterfaceDef.start_selection: Option<ContentRef>` (default none: no selection, as today). The camera keeps
  `CameraStart::OwnedCentre` (`crates/turts_mod_api/src/interface.rs:327`).
- Hints (7.8): `InterfaceDef.hints: Vec<HintDef>` (default empty), `hint_fallback: Option<String>`, `Value::Hint`,
  `Predicate::HintShown` (6.10 rows H14, H7, H10). Today `Source` and `Predicate` have no hint (`interface.rs:216-240`).
- Alerts (7.9): `AlertDef.kind: Event | State` (default `Event`, the 0.2.3 behaviour), `clear_after: f32` (default 10,
  `State` only, at least 0), `units: Vec<ContentRef>` (default empty: all units), `select_on_click: bool` (default
  false); the placeholders `{unit}` and `{slot}` in `text` (an unknown placeholder is refused); triggers
  `SupplyShort(res)`, `SupplyRestored(res)`, `InsufficientFunds(res)`, `ConstructionComplete`, `SlotDisconnected`,
  `SlotReconnected` [mod M5], and the existing `GathererWaiting(secs)`; the alert row as a button with the core action
  `FocusAlert` (D74: an agent drives it headless) (6.10 rows H4 to H6, H17).
- Power and match: `Value::Supply(res, Provided | Consumed)`, `Predicate::SupplyShort(res)`, `Value::Connection` (the
  banner of a disconnected slot), `Value::Outcome`, `Value::OutcomeReason`, `Value::MatchStats`, `Value::MatchClock`,
  `Predicate::MatchOver` (6.10 rows H7, H10).
- Fields: primary `Armour`, `Activity`, `Cargo`; item `Role`, `BuildTimeNow`, `SupplyChange`, `CausesShortage`,
  `State`, `Paid`, `Refund`, `Badge`; predicates `ItemHas(field)` and `PrimaryHasValue(field)` (MCX1) (6.10 rows H8 to
  H10).
- Content: `UnitPresentation.role: Option<String>` (today `icon`, `portrait`, `responses` only:
  `interface.rs:410-415`); production entry `hotkey` and `card`; Button `font` and `size` (6.10 rows H19, O8, O10,
  H13).
- World styles: `placement` (valid, invalid, stripes), `build_area_ring`, `gap_ring`, `progress_bar`, `cargo_bar`,
  `rally`, `float_text` (with a sound id) (6.10 row H11).
- Cursor contexts `place`, `repair`, `rally` (new) and `forbidden` (exists), and the reason keys of 6.1 plus
  `match_not_started`, `slot_defeated` and `match_over` (6.8): an unknown key is refused at pack time with a "did you
  mean" hint [mod M8] (6.10 rows H1, H2).
- Core (D74): placement mode and the effect `Ghost{content, point, valid, reason}`; the hint state; `FocusAlert`. All
  run with no renderer.

The power source is named `Supply`, because the same source gives the supply cap of a later mod.

### 7.6 Lobby and result screen

Owners: the shell owns the lobby and the "Leave" screen; the mod owns the result screen. This split is D70, which is
Proposed (S:99) [rigour 12]. MP1 has no settings schema (MD12 is `later:M9`): the lobby shows the seats, the colour
from the mod palette, and Ready [rigour 12]. The server state machine (Boot, Lobby, Countdown, Running, Ended, then
exit 0; decision D-C) and its cases are in 6.7. The words below are what each player sees in each case. "Player 1"
and "Player 2" are slot labels (the `{slot}` text).

| Case of 6.7 | This window | The other window |
| --- | --- | --- |
| Lobby, one seat taken | "Slot 1: You — Not ready", "Slot 2: Open — waiting for a player"; status "Waiting for a second player." | none |
| A second player takes a seat | "Slot 2: Player 2 — Not ready"; status "Press Ready to start." | "Slot 2: You — Not ready"; the same status |
| One player is ready | "You are ready. Waiting for Player 2 to press Ready." | "Player 1 is ready. Press Ready to start." |
| Both ready: Countdown | "Match starts in 3", "2", "1" | the same |
| A player changes colour or clears Ready during Countdown | "Countdown stopped: Player 2 is not ready." | the change clears the ready state of that player only |
| A player leaves the lobby or the countdown | the seat row is "Slot 2: Open — waiting for a player"; status "Player 2 left. Waiting for a second player." | none |
| A seated player reloads the page in the lobby | the same seat, "Not ready" (the browser keeps the player id from the seat on, backend M6) | "Player 2 — Not ready" |
| Running starts: the claim of every seated slot (decision D-B) | "Click to play" | the same |
| A new player id during Running | shell screen: "This match has started and has no open slot. Wait for it to end, then reload this page." | nothing |
| The other player's session ends during Running | alert "Player 2 lost connection."; banner "Waiting 1:59 for Player 2 to return." that counts down in match time | none |
| The other player returns | alert "Player 2 is back."; the banner goes | the match continues in the own slot (MC SLOT1) |
| Ended | "Victory", "Defeat" or "Draw" within 1 s; after 3 s the result screen | the same |
| A claimed player connects after the end | the result screen of that match | none |
| A new player id after the end | shell screen: "The match is over. Reload this page in a few seconds to join the next match." | none |
| The page cannot reach the server (the next process is not up yet, D-C) | shell screen: "The server does not answer. It can be starting the next match. Reload this page in a few seconds." | none |

Notes: the session end comes at most about 10 s after a close (the netcode timeout; the native probe of 6.7.8 gave
9.1 to 10.0 s; the browser lag is NOT TRACED, section 15), so the banner can start late; the 2:00 is counted in ticks
from the logged disconnect (decision D-D). 6.7.5 sets no lobby timeout (the room waits with no time limit), so the
lobby has no "room closed" screen.

Result reasons (decision D-K). The engine gives an outcome and a reason code for each slot (MAT1); the mod gives the
words:

| Reason | Winner | Loser |
| --- | --- | --- |
| The key units of the other team are lost (winner `OpponentsDefeated`, loser `KeyUnitsLost`, 6.7.9) | "Victory. You destroyed the enemy Command Centre." | "Defeat. Your Command Centre was destroyed." |
| No team alive in the same tick (`Draw` with `KeyUnitsLost`) | "Draw. Both Command Centres were destroyed at the same moment." | the same |
| Surrender (`OpponentsDefeated`, `Surrendered`) | "Victory. Player 1 surrendered." | "Defeat. You surrendered." |
| The disconnect clause (`OpponentsDefeated`, `Disconnected`) | "Victory. Player 2 left the match and did not return within 2:00." | (on a later connect) "Defeat. You were away for more than 2:00." |

The result screen (mod layout): a dim full canvas, the headline, the reason line, the match time, one row for each
slot (colour, label, outcome, and the statistics of `MatchResult`, 6.7.13: credits gathered, units trained, units
lost, enemy units destroyed, buildings built, buildings lost), and one button "Leave" (`Input("leave_match")`). There is no rematch button in MP1
(decision D-K) [player 9]. "Leave" opens the shell screen: "The match is over. This server starts a new match. Reload
this page to join it. If the page shows an error, wait a few seconds and reload again."

The surrender menu item: "Surrender" asks "Surrender and end the match? You will be defeated." with "Cancel" and
"Surrender".

### 7.7 MC items

Button `font` and `size`; "Unbound"; Controls groups and keyboard navigation (search waits for M8); the alert list clip
marks and row pitch; a value predicate so that "x1 - HP" does not show for a unit with no health; the grouped failure
notice of ruling O14 ("3 of 8 units: Cannot reach that place", one sound, a click selects the failed units); and a plain
page when the browser gives no GPU adapter (the CEO met a panic with a console message only, 2026-11-11). The page
text: "TuRTS cannot start: this browser gives no graphics adapter (WebGPU). Turn on hardware acceleration in the
browser settings. On Linux, Chrome has WebGPU off: open chrome://flags, turn on "Unsafe WebGPU Support", and restart
Chrome."

### 7.8 Goal line and hints

- The goal line is a mod `Text` widget with the literal "Destroy the enemy Command Centre" (no new engine item). It
  shows from the first frame after "Click to play" until the result.
- The Command Centre is selected in the first frame (`start_selection`), so its build menu shows.
- The guide does not open at the start. The hint line teaches the first actions.
- One hint line shows at a time, at a place that the mod sets. The full list with its words is in the Player path.
- Hint kinds: a step hint teaches the next action in order; a situation hint answers a state (no Ore Hauler, low power,
  jobs on hold); the fallback line names an action that the player can always do.
- Selection rule, in each client update: the first situation hint whose start holds and whose done does not hold;
  else the first step that is not done and whose start holds; else the fallback line, if a step is not done; else no
  line.
- A step whose done condition already holds when the list reaches it is done at once and does not show.
- Done: a step is done in the first client update in which its done condition holds. It stays done when the
  condition holds through a site, a complete building or an order that the server accepted. When it holds only
  through a pending site, the line changes at once (so placing a building names the next action in the same frame),
  but if the server refuses that order, the step is not done and its line shows again [player H2, rigour 10].
- Conditions are client facts from the replica and the own order results: own units of a type (pending, site,
  complete or any), an own job of a type, no own unit of a type, a rally of the own producer, an accepted own order of
  a type, a supply short, a job on hold for N s, another hint done, a hint shown for N s, and `Not` (with `All` and
  `Any` to join them). Their names in the package are the `HintCondition` variants of 6.10 row H14. The interaction
  core evaluates them, so an agent reads the hint state headless (D74).
- The client records the times in `turts_debug.hints` (7.1).

### 7.9 Alerts

Two kinds of alert rule [player H3]:

- Event: one line for each event, coalesced by place (`coalesce_radius`); the sound plays at most once in each
  `cooldown` seconds for the rule. This is the 0.2.3 behaviour (`crates/turts_client/src/feedback.rs:849-853`).
- State: the line and the sound come when the state starts. A state that holds in the first replica after "Click to
  play" is not a start, so nothing sounds at tick 0. The line stays while the state holds, and nothing sounds again. The
  state ends only after it stays false for `clear_after` seconds. When it starts again, the line comes back; the sound
  plays only if the last sound of the rule is at least `cooldown` seconds old. So a state that clears at each delivery
  and comes back 5 s later gives one line and one sound in each `cooldown` window.

Rev 3 had one rule form, a cooldown per rule (`crates/turts_mod_api/src/interface.rs:375-384`), and MC content gave
seven rules a cooldown of 1.0 s (`mods/forge/interface/hud.ron:130-136`): that was "it beeps non stop".

The rules of `frontier`. Each text names a thing on the screen, and what to do when there is something to do. A click
on a line moves the camera to its place (`FocusAlert`); "Ore Hauler idle" also selects the hauler.

| Rule | Trigger | Kind | cooldown, clear_after | Text | Sound, ping | Place |
| --- | --- | --- | --- | --- | --- | --- |
| Construction complete | `ConstructionComplete` | Event | 2 s | "{unit} complete." | yes, yes | the building |
| Unit ready | `ProductionComplete` | Event | 2 s | "{unit} ready." | yes, no | the new unit |
| Low power | `SupplyShort(power)` | State | 60 s, 10 s | "Low power: everything builds at half speed and turrets are off. Build a Power Station (P)." | yes, yes | an offline turret, else the Command Centre |
| Power restored | `SupplyRestored(power)` | Event | 30 s | "Power restored." | no, no | the Command Centre |
| Not enough credits | `InsufficientFunds(credits)` | State | 90 s, 15 s | "Not enough credits: jobs are on hold. Your Ore Haulers bring more." | yes, no | the Command Centre |
| Base under attack | `OwnUnitDamaged`, units Command Centre, Power Station, Guard Turret | Event | 20 s | "Base under attack. Click to see it." | yes, yes | the damaged building |
| Ore Hauler under attack | `OwnUnitDamaged`, units Ore Hauler | Event | 15 s | "Ore Hauler under attack. Click to see it." | yes, yes | the hauler |
| Ore Hauler destroyed | `OwnUnitDestroyed`, units Ore Hauler | Event | 5 s | "Ore Hauler destroyed. Train a new one at the Command Centre (O)." | yes, yes | the place of the loss |
| Ore Hauler idle | `GathererWaiting(5.0)` | State | 60 s, 5 s | "Ore Hauler idle. Select it and right-click ore." | yes, yes | the hauler |
| Lost connection | `SlotDisconnected` | State | 1 s, 0 s | "{slot} lost connection." | yes, no | none |
| Back | `SlotReconnected` | Event | 1 s | "{slot} is back." | no, no | none |

### 7.10 Reading a fight

- Role line: each unit tooltip and build entry shows the mod `role` text [player 6]:
    - Rifleman: "Strong against infantry. Weak against tanks and buildings."
    - Launcher: "Strong against tanks and Ore Haulers. Weak against riflemen."
    - Engineer: "Repairs vehicles and buildings. Has no weapon."
    - Ore Hauler: "Brings ore to the Command Centre. Has no weapon."
    - Tank: "Strong against riflemen, vehicles and buildings. Weak against launchers."
    - Guard Turret: "Strong against tanks and infantry. Needs power. Cannot move."
    - Power Station: "Makes 100 power."
    - Command Centre: "Trains units and builds buildings. Makes 3 credits each second. Lose it and you lose."
- A reduced hit shows a small hit effect: the client computes the table ratio from the weapon of `Fired`, the armour
  class of the target and the damage table of the package; a ratio below 1/2 picks `hit_reduced` (8.3) [player 6].
- The tooltip of a build entry shows the build time at the current power: "Build time 12 s", or under a shortage
  "Build time 12 s (24 s now: low power)" [player 7].
- Each delivery shows "+N" at the receiver with the delivery sound (8.4). N and the receiver come from the owner
  effect `Credited{unit, resource, amount}` of 6.8, which the server sends for a delivery and for a refund; the
  client derives nothing from the hauler's load. The free income shows no float; it shows only in the credits
  counter.

## 8. Presentation

This block is the whole section 8, with its subsections 8.1 to 8.5. From RE, with the gate-1 changes.

### 8.1 Models (decision P1, option b)

- `models` in the package assets (`AssetsDef`, `crates/turts_mod_api/src/assets.rs:5-10`, has images, fonts, sounds and
  cursors only), and an optional `model` on `VisualDef` (`crates/turts_mod_api/src/schema.rs:501-504`):
  `{asset, team_material, turret_node, muzzle_node}`. `shape` stays required: the fallback while a model loads, and the
  content cut line.
- Status: S:247 keeps the model formats and features Open. The checks below are the MP1 validation profile, not the
  final engine format; a later milestone can widen it with a version-table row [rigour 12].
- The validator uses `gltf = "=1.4.1"`: `bevy_gltf` 0.20 asks for `1.4.0` (`bevy_gltf-0.20.0/Cargo.toml:122-123`) and
  the workspace lock has `1.4.1` (`Cargo.lock:3929-3930`), so the validator parses what the client parses.
- The browser loads from the package bytes in memory (Bevy 0.20 `MemoryAssetReader`,
  `bevy_asset-0.20.0/src/io/memory.rs:290`), with no fetch.
- Team colour: the client replaces the team material with the slot material at spawn.
- The client builds one flat template for each model and spawns its own entities; it does not spawn scenes.
- The MC packages keep their shapes, so the MC pixel rows do not change.

The asset brief as pack-time checks [mod M9]. Each refusal is a `ModError` with the package, the file and the field;
the texts follow the style of the existing asset checks (`crates/turts_mod_runtime/src/content_mc_validation.rs:175`,
`:212`). The numbers in brackets are the caps of decision D-M (CEO).

| Check | Error text (example) |
| --- | --- |
| The file is GLB version 2 | `assets/models/tank.glb: not a GLB version 2 file` |
| No `uri` in buffers or images | `assets/models/tank.glb: buffers[0] has an external uri; a model must embed all its data` |
| No `extensionsRequired` | `assets/models/tank.glb: extensionsRequired lists KHR_draco_mesh_compression; an MP1 model accepts no required extension` |
| No skins, animations, cameras or lights | `assets/models/tank.glb: has 1 skin; an MP1 model accepts no skins` (the same form for the others) |
| Embedded images are PNG, within the existing image caps (2048 px, `content_mc_validation.rs:17`) | `assets/models/tank.glb: image 0 is image/jpeg; an MP1 model accepts embedded PNG only` |
| At most 2 materials | `assets/models/tank.glb: has 3 materials; maximum is 2 (body and the team material)` |
| Triangles, from the index accessor counts [8,000 for one model] | `assets/models/tank.glb: 9,412 triangles; maximum is 8,000 for one model` |
| Bytes [4 MiB for one model; 16 MiB for all models] | `assets/models/tank.glb: 4,500,000 bytes; maximum is 4,194,304 bytes` and `assets/assets.ron: models: total 17,000,000 bytes exceed the 16,777,216 byte package limit` |
| The team material exists | `visuals/tank.ron: model.team_material: material `team` is not in assets/models/tank.glb; materials are: body` |
| Each named node exists (`turret_node`, `muzzle_node`, when given; the `frontier` names are `turret` and `muzzle`) | `visuals/tank.ron: model.turret_node: node `turet` is not in assets/models/tank.glb; did you mean `turret`?` |
| Scale: the XZ radius from the POSITION bounds is 0.8 to 1.2 times the footprint radius | `assets/models/tank.glb: radius 2.31 is outside 0.72 to 1.08 (0.8 to 1.2 x the footprint radius 0.9 of frontier:tank)` |
| Pivot: the lowest point is at ground level, min y from -0.05 to 0.05 | `assets/models/tank.glb: lowest point y = -0.40; the pivot must be at ground level` |

Advice, not checked (the validator cannot measure it): forward is glTF +Z (Blender -Y) and +Y is up; the team material
covers at least 15 % of the top view; body colours are neutral greys and browns, with no saturated green and none of
the slot hues; textures 512 px for units and 1024 px for the Command Centre; triangles about 1,500 for infantry, 3,000
for vehicles and 8,000 for buildings (RE part 6).

### 8.2 Heading and aim

The client derives the yaw of a unit from its movement, with a turn rate; no wire change. A turret node aims from the
`Fired` event. Walking animation needs skinning and stays at M13; infantry gets a small bob. A site model rises from
the ground with its progress.

### 8.3 Effects

- Pooled unlit meshes for the muzzle flash, the full hit (`hit`) and the reduced hit (`hit_reduced`, smaller and
  dimmer, 7.10), a line for a tracer, an expanding burst for a death, smoke below 50 % and fire below 25 % health on a
  building, dust on a site, an arc for repair, an offline mark on an unpowered turret, and the "+N" float.
- One pool of 256 instances. The pool never grows: when it is full, a new effect takes the oldest instance, and no
  effect spawns an entity of its own [perf L1]. No compute shader, so WebGL2 stays possible.
- Effects are never replayed: the client drops an effect whose tick is older than the presentation tick minus K
  (K = 4 ticks, Proposed), so a resync after a lost delta gives no burst [net 7].
- A mod names each effect by content id; the server never reads them. Effect colours avoid the slot hues of the pixel
  drivers.

### 8.4 Sound

Effect sounds need a small engine part, as section 16 says: an effect definition names a sound id; the engine plays it
at the place of the effect; the gain falls with the distance from the camera look-at point (to 0 at the mod
`audible_range`); the effects category plays at most the mod cap at once (`effects_voices`, default 8; both fields
are 6.10 row H16), and a sound beyond the cap is dropped. X14 takes this engine part [rigour 12]. The audio engine and its categories exist (D69,
Proposed; the alert cap of two at `crates/turts_client/src/audio.rs:795`). The sound set is mod content: the minimal
set of decision P5 (fire, hit, death, construction complete, low power, unit ready, error), plus the reduced hit and
the delivery sound with the "+N" [player answer 5].

### 8.5 Measurement first

- The first presentation task measures the frame time at 400 units with primitives and with models, in the browser on
  the real GPU (RE part 5: 10.7 to 43 ms is an estimate, not a measurement). Before any art exists, it uses placeholder
  models at the triangle and byte caps of decision D-M (with 512 px and 1024 px textures, the sizes of 8.1), and it records the wasm memory high-water with them
  (exit item 5): the performance lens estimated about 16 MiB of textures and a heap growth of about 35 MiB that never
  shrinks (perf L3, an estimate, NOT TRACED).
- The debug state is built only when the harness asks for it, at most 4 times a second, and for the last presented
  frame. Today the client rebuilds it in each frame (`crates/turts_client/src/lib.rs:147`,
  `crates/turts_client/src/debug.rs:441`); at 400 units that costs 4.4 to 5.1 ms native (gate1-performance.md:15).
  VIS3 records the frame time with the debug state off and on [perf H1].
- The play-test floor (Proposed; decision D-N): frame time p95 at or below 50 ms and p50 at or below 33 ms at 400
  units on the play-test machine. PT3 does not start below it [perf L2].

## 9. Architecture

**Crates.** The crate table after MP1 is TD part D, with these rows added or changed (each row is checked by
`scripts/boundary.sh`, and each new row has a fixture case that goes red):

| Crate | Logical module (S 4.1) | Change | Forbidden dependencies |
| --- | --- | --- | --- |
| `turts_rules` (new) | simulation core, used by presentation | pure rules: placement check with the visibility predicate, `relation`, the damage formula, the charge and heal difference functions, the shortage effect. Own f32 point type, because the lockfile has two glam versions (TD part B) | any `bevy*`, every workspace crate, `wgpu`, `winit`; host and wasm32 rows |
| `turts_mod_api` | mod runtime and API | the 0.2.4 schema of 6.10, `REASON_KEYS`, `CURSOR_CONTEXTS`, `STATE_KEYS` | any `bevy*` |
| `turts_mod_runtime` | mod runtime and API | RAW presence, the 0.2.4 rows, the edition, `UNBUILT_FEATURES` | any `bevy*` |
| `turts_net_protocol` | multiplayer | protocol 5 (6.8) | any `bevy*` |
| `turts_sim` | simulation core; production and logistics | construction, supply, damage, relations, match state, `UnitIndex` | `bevy_render`, `bevy_winit`, `bevy_pbr`, `turts_mod_runtime`, `turts_audit` |
| `turts_interaction` | presentation and player interface | placement mode; `Core::step` split by input kind; uses `turts_rules` | any `bevy*`, `turts_sim`, `wgpu`, `winit`, `turts_audit` |
| `turts_server` | headless host (owner: `backend-engineer`) | `room.rs`; match lifecycle (6.7) | `bevy_render`, `bevy_winit`, `bevy_pbr` |

The simulation stays headless: `turts_sim` and `turts_rules` have no rendering crate on host or wasm32.
`simulation-core-engineer` owns the headless-runnable core; `backend-engineer` owns the headless host.
`turts_rules` builds for wasm32 (`cargo check -p turts_rules --target wasm32-unknown-unknown`).

**One copy of each rule.** The sim and the interaction core call `turts_rules`; the client holds no second copy
of a placement, relation or damage rule. An instrument for CON2: a grep fails when placement arithmetic appears in
`turts_client` or `turts_interaction` outside calls to `turts_rules`. The placement arithmetic compares squared
distances and uses no square root, no trigonometry and no fused multiply-add, so it gives the same f32 result on
wasm32 and native.

**UnitId index in lane 0** [perf H2]. `UnitIndex`: a map from `UnitId` to `Entity`, with one insert at each spawn
and one remove at each removal. It is used for lookup only and is never iterated, so the tick order does not
depend on it. Lane 0 uses it at these sites before any feature lane adds a lookup (the performance lens measured a
scan at 5.0 µs at 1,000 entities and 25.9 µs at 5,000, against 0.026 µs for a map lookup):

1. `crates/turts_sim/src/economy/production.rs:282`: one scan for each active producer in each tick.
2. `crates/turts_sim/src/economy/transaction_m3.rs:635` (`find_unit_entity`), and `:524` (the ordered attackers of
   a removed unit: a reverse set, kept where an ordered target is set or cleared; if those writers are not in one
   place, lane 0 keeps the scan and records its cost).
3. `crates/turts_sim/src/economy/gather.rs:1206` (`find_unit`): about 10 calls for each gatherer in each tick.
4. `crates/turts_sim/src/economy/combat.rs:94` and 5. `crates/turts_sim/src/orders.rs:703`: a full scan and maps
   in each tick. They become one per-tick unit frame (positions, owners, teams), built once and used by combat,
   engagement and the acquisition grids of 6.3. This part belongs to the targeting lane and rebases on slice parts
   A2c and A3.
6. `crates/turts_server/src/export.rs:299` (`owner_of`): a linear scan for each record and slot (4.1 to 7.2 ms for
   each tick at 5,000 units and 2 slots, performance lens). `view.units` is sorted by unit id
   (`crates/turts_sim/src/world.rs:61`), so a binary search replaces it.
7. `crates/turts_sim/src/orders.rs:3056`, `:3085` (`validate_target`): once for each command, so not hot; they use
   the index because the index exists.

**String ids become indices at install** [perf M4]. Damage type, armour class, acquire, footprint and the supply
resources resolve to small integer indices when the catalog is installed. No tick or frame parses a content id.
Today `combat.rs:265` parses one for each weapon in each tick, and `crates/turts_client/src/visuals.rs:648` one for
each unit in each frame. The CON2 ghost reads a footprint list that is rebuilt for each applied delta, not for each
frame.

**New records enter the committed tick from their writer** [perf M5]. Each new record kind (sites, supply, job
progress, effects, order changes) enters `CommittedTick` (`crates/turts_sim/src/economy/events.rs:138`) from the
change list of the code that writes it. No new full view and no diff of the world for each tick (the idle tick
already costs 3.0 ms at 1,000 units, performance lens).

**The client debug state** [perf H1] [perf M2]. Decision: lane 0 of MP1, not the slice. The slice plan is gated,
and its client task S45 is a pure move; the debug state touches the harness bridges that every MP1 pixel row uses.

- Today `rebuild_debug_state` runs in `Last` for each frame (`crates/turts_client/src/lib.rs:147`), and the web
  client copies it for each frame (`crates/turts_client_web/src/lib.rs:166-170`, `:372`). Measured (native): 4.4
  ms and 148 KB at 400 units; 96.8 ms and 1.78 MB at 5,000 units.
- After lane 0 it is built only when the harness asks for it, at most 4 times each second. VIS3 records the frame
  time with the debug state off and on. Every MC browser row stays green (REG2).
- `health_trace` (`crates/turts_client/src/replica.rs:1182`) is pruned to a window. MP1 adds no field to the
  replica candidate [perf M2]. The other replica costs of perf M2 stay as they are in MP1: each delta clones the
  candidate (`replica.rs:432-443`, `:451`), and `replace_keyed` (`:1029`) is O(changes x records). Exit item 5
  records native `apply_delta` p50 and p99; section 15 point 12 holds the open part.

**Behaviour handler table.** After the pure moves of the slice: a table in `turts_sim/src/orders/`, so a new
behaviour (Construct, CancelConstruction, SetRally, Repair) is one new file and one table line.

**The API bump.** `ENGINE_API_VERSION` 0.2.4 and `RELEASED_ENGINE_APIS` gain one entry
(`crates/turts_mod_api/src/lib.rs:52-55`). The literal "0.2.3" appears in `crates/turts_mod_api/src/lib.rs`,
`crates/turts_mod_runtime/src/version_table.rs` and `crates/turts_mod_runtime/tests/t2c_package_rejects.rs`
(grep); lane 0 checks each test that rewrites an `engine_api` text, so that no test turns into a no-op.

**Refused in any MP1 plan** (TD part E): a validator branch for an MP1 behaviour; a second movement path; a
default-mod rule name in engine code; a frozen-value change other than the one approved re-freeze; client
authority over placement or outcome; an Open item closed without a measurement or a decision (tick rate, numeric
budgets, transport, a scripting form).

## 10. Lanes and order

A dependency graph, not waves; a task starts when its inputs are merged (PR part 4). One worktree and one build
cache for each lane; reviewers in their own worktrees; fast checks for each task; the slow checks one time for each
merged tree. The GPU is the scarce resource: two headed clients for one match is tested in the first two days.

**Start points from the pre-MP1 slice** (slice plan section 5; slice design revision 6, section 9 item 15):

| Point | Slice tasks merged first | MP1 work that can start |
| --- | --- | --- |
| now | none | L-LOBBY (server room; it does not touch the sim) |
| E1 | S1, S2, S3, S45, D0p (the pure moves and the audit crate) | lane 0a; L-DMG, L-PWR, L-PAY, L-FX, L-MATCH, L-DES. Every sim write is inside a captured operation (`capture_operation`, `crates/turts_sim/src/economy/transaction.rs:105`). An edit of `economy/combat.rs` rebases on A2c and A3 |
| E2 | A1c (the replay validator is deleted) | lane 0b; L-CON, L-RLY, L-REP, L-TGT (they add orders) |
| A3 | A3 | L-CONTENT tests and measurements with the frontier footprints (6.9, perf H4) |

**Lanes:**

- **Lane 0a** (E1): the 0.2.4 schema of 6.10 with `UNBUILT_FEATURES`; RAW presence, the edition and every
  version-table row; the protocol 5 records of 6.8 with empty writers; `turts_rules` with its tests; `UnitIndex` and
  hot sites 1, 2, 6 and 7 of section 9; string ids to indices; the client debug state on request and the pruned
  health trace; the `Core::step` split.
- **Lane 0b** (E2): the behaviour handler table in `orders/`; hot site 3 (`gather.rs`, which rebases on slice
  parts A2b1 and A2b2).
- **Feature lanes:** L-DMG (X2 damage), L-PWR (X6), L-PAY (X7), L-FX (X14), L-MATCH (X11, backend), L-DES (X1), L-CON
  (X4, X4b, X5 server part), L-TGT (X2 targeting, hot sites 4 and 5), L-RLY (X9), L-REP (X8), L-LOBBY (X12, backend),
  L-UI (X5 ghost, X13, the HUD of section 7, X18), L-CONTENT (X15), L-PRES (X16, X17), and the instruments (X20).
- Each feature lane owns its own pixel rows; the final browser driver holds only the full match.

**Two stages, CEO order of 2026-11-11** (orchestrator brief; the vault note is NOT TRACED): first one thin
complete playable loop with simple shapes, which a person plays at a checkpoint; then the breadth.

**Stage 1: the thin loop.** A person builds power, harvests, trains, fights, wins and sees the result screen,
with simple shapes.

| Item | Stage 1 part |
| --- | --- |
| X19 | all of it, in lanes 0a and 0b |
| X1 | the two lifted rules and destruction |
| X0, X0e | lane 0 and the entry condition A3 |
| X2 | the damage table, armour, idle acquisition, and retaliation with the hold rule (TGT2; sections 4, 11, 12 and 16 put it in stage 1: the hold case of D-G needs it, and the reconcile pass removed the stage-2 note of revision 4 part 2) |
| X4 | construction with held orders and sites (cancel, X4b, is stage 2) |
| X5 | the ghost with valid and invalid styles and the reason at the pointer |
| X6 | supply, the shortage effects, the power text |
| X7 | charge over time for jobs and sites |
| X10 | `Ally`, `Enemy`, team = slot |
| X11 | `LastTeamStanding`, defeat and draw (surrender and disconnect are stage 2) |
| X11b | Boot, Running, Ended, exit 0, and the supervisor script (LEAVE1 without the lobby seat) |
| X13 | the outcome and its reason (statistics are stage 2) |
| X24, X25, X26 | the start order of a placement (STO1), the hint list (GOAL1), the free income (INC1) |
| X14 | the `Fired` record and one tracer line for each shot; death by removal |
| X15 | the 8 types, Twin Fields, the credits and power HUD, the goal line and the hint list, the start gather order of decision D-F, simple shapes |
| X20 | the headless two-client match and the pixels driver of the stage 1 path |

Stage 1 runs with the start policy `Immediate`; the lobby lane can land earlier, but stage 1 does not wait for it.

**Checkpoint C1.** A person plays one full match in two browser windows with simple shapes: the hauler gathers
from tick 0, the person places a power station from the first hint, trains units, fights, destroys the other
command centre, and both windows show the outcome with its reason. Conditions: lanes 0a and 0b, the stage 1 lanes
and slice part A3 are merged; the stage 1 rows of section 12 (the one list, "Stage 1 rows") are green, each with its
recorded red, and REG2 is green. The person signs off C1 in the vault. When C1 fails, stage 2 does not start until its cause is fixed.

**Stage 2: the breadth.** X4b, X8, X9, X11c, X12, X11b (the lobby states), X11 (surrender, the disconnect clause,
statistics), X13 (statistics), the rest of X14, X16, X17, X18; then X21 (the play test), X22 (the final regression)
and X23 (the architecture audit).

The producer re-estimates section 11 and the critical path for the two stages.

## 11. Budget

At the MC rate (PR part 3: `docs/research/mp1/input-producer.md:76-101`). The split into the two stages and the
split of X11, X14, X15, X20 and X21 are qa-lead estimates from that table (arithmetic in the gate-1 repair
report). The gate added work that PR part 3 does not hold: the UnitId index in lane 0 (3 to 5 runs, from the
MP2 list of PR part 3), the debug-state gate (1 to 2 runs, estimate, NOT TRACED) and the free income of D-L (1 to 2 runs, estimate, NOT TRACED). The lobby state machine
and the process per match (D-B, D-C) are inside X12; disconnect and surrender as sim inputs (D-D) are inside
X11; `unit_orders` as changes only is inside X19; the acquisition grid (D-J) is inside X2.

| Part | Runs |
| --- | --- |
| Pre-MP1 slice (before MP1) | 11 to 18 |
| Stage 1 tasks: X0, X0e, X1, X2, X4, X5, X6, X7, X10, X11, X11b part, X13, X14 part, X15 part, X19, X20 part, X21 player agent, X24, X25, X26 | 54 to 90 |
| Stage 2 tasks: X4b, X8, X9, X11b part, X11c, X12, X14 rest, X15 rest, X16 (8 to 16), X17, X18, X20 rest, X21 people, X22, X23 | 44 to 74 |
| Gates (spec, plan, stage-1 verify, final verify, re-gates of repairs) | 20 to 35 |
| Total | 129 to 217 |
| Total without models (P1 a) | 121 to 201 |

Build time on the critical path: 55 to 160 hours (PR part 3; not measured again). Gates and the person play come
on top. MP2, coarse: 48 to 80 runs, because the UnitId index (3 to 5 runs) moves from MP2 to MP1 lane 0. These
numbers replace the early estimate of 95 to 150 runs for MP1 and MP2 together.

## 12. Acceptance rows

Each row names the check (what is observed), the red build, and where it runs. The red build is a plant: a
concrete change to a file or a mechanism that a real build can contain, with the failure that the check gives
on it. "The feature does not exist" is not a red build, because a check that is red only on an empty build
can stay green on a wrong build [rigour 6]. Where today's tree already fails a check, the row says so; that is
a real red (OBS1, DBG1, the explicit-empty packages of VAL1). Each row records its red output and its green
output with the commit; VIS3 is the one row that is recorded, not judged. A row that a balance number can
break (DMG1 duels) says if a failure is a red of the engine or a retune of the content.

Runs: `headless` is a `cargo test` or a process test with no window and no rendering plugin (specification
section 4.1); `pixels` is a pixels-only driver with a bridge oracle; `people` is the play test. The plan fixes
the commands. A headless row that needs a match uses `--command-script`, with a test package that has no
start policy (`Immediate`, D-B) or with `frontier` and `--start-now` (IMM1), so it needs no client that presses
ready. The rows PAY1, PAY2, CON3, DES1, REP1 and INC1 need
`turts_audit`, which slice part P1 adds (it is not in today's tree).

Stage 1 rows: NC1, VAL1, IMM1, DMG1, TGT1, TGT2, CON1, CON2, PWR1, PAY1, INC1, DES1, OBS1, STO1, DBG1, ORDW1,
MAT1 (victory, defeat and draw), RES1 (the outcome and the reason line), LEAVE1 (the exit after the end and the
restart), GOAL1, GHO1, PRD1, HUD5, KEY1, ALR2, FX1 (the wire event, the flash and the death effect), REG2. Stage 2
rows: CON3, PAY2, REP1, RLY1, LOB1, LOB2, SUR1, MAT1 (surrender and disconnect), RES1 (the statistics and the wire
check), LEAVE1 (the lobby seat of the next match), HUD6, SMT1, AUD2, MDL1, VIS3, MCX1, NOGPU1, FX1 (the rest), PT3
(people), and REG2 again. Section 10 (the stage tables and checkpoint C1) uses this list.

| Row | Check | Red build | Runs |
| --- | --- | --- | --- |
| NC1 | The no-content scan (`crates/turts_checks/tests/no_content.rs`, `real_source_scan_is_clean`, :166) derives a token for every content kind of API 0.2.4: the kinds of today's list (`crates/turts_checks/tests/support/mod.rs:86-99` and the loops to :267) and each new kind of the 0.2.4 table (D-I), for example damage types, armour classes, supply resources, victory clauses, start policies, hint lines, effects and models (the queues have no ids, decision Q1), and the interface kinds (alerts and widgets) that today's list does not hold. For each kind the scan keeps the qualified id and the bare local id (`add_id`, support/mod.rs:449-456; decision N1). A bare id that equals an engine word is red and names the package, the definition path and the field; the repository package renames the id, or the scan gets an exception row (the table at support/mod.rs:272-292) with a reason and `identifiers_only: true`, so that a string literal of the word stays red. The modder guide states this rule; a mod outside `mods/` is never scanned [mod H3]. No 0.2.4 engine field name equals a `frontier` local id (for example a field `power` while `frontier` has a resource `power`) | For each new kind, one planted string literal in a scanned crate (for example `"frontier:cannon"` as a damage type and `"frontier:structure"` as an armour class, in `crates/turts_net_protocol/src/lib.rs`): a derivation that misses the kind gives 0 hits for its plant, and the row is red on that plant. The gap is real today (qa-lead run on d729a97; the code of 1b15402 is the same): the literal `"muster_unit_damaged"` (an alert id, `mods/muster/interface/hud.ron:19`) gives 0 hits and exit 0, while `"muster:trooper"` gives 48 hits and exit 101 | headless |
| VAL1 | The two M3 rules are lifted (`crates/turts_mod_runtime/src/content_m3_validation.rs:374-376` and `:420-424`; 6.10 rows K7, K8), and the work-capability exemption of `constructor` beside `producer` (`:418`, D-E) has its row. Each row of the 0.2.4 table (6.10, D-I) has its red packages, by its detection kind: for a RAW row, a `^0.2.3` package that writes the field, and a second one that writes it with an empty value, an explicit `None` (for example `armour: None`) or an explicit `[]` (for example `orders: []`) [mod M4]; for a VAR row, a `^0.2.3` package with the variant; for a COMBO row, a `^0.2.3` package with the combination; for an ED row, a `^0.2.3` and `^0.2.4` pair that shows the two behaviours. Each refusal has the form `` `<row name>` needs engine api 0.2.4; `^0.2.3` admits 0.2.3 `` (`crates/turts_mod_runtime/src/version_table.rs:101-108`). A reserved variant (`NeedsBuilder`, `counts: Queued`, `BlockStart`, `AllSeated`) is refused with "reserved for a later engine API". A package with a constructor and no `open_sites_per_slot` is refused (6.10 row K6). An unknown reason key or cursor key is refused with a `did you mean` hint [mod M8] | (1) A table row missing: its red package packs with exit 0. (2) Presence read from the parsed value, the form of today (`orders_present: !orders.is_empty()`, `crates/turts_mod_runtime/src/content_mc_validation.rs:65`): the explicit-empty packages pack with exit 0; this red exists today (`^0.2.2` with `orders: []` packs with exit 0, mod-builder probe at gate 1). (3) A reserved variant accepted. (4) Key check off: `queue_ful` and `plcae` pack with exit 0 (today's behaviour, mod-builder probe) | headless |
| DMG1 | Damage is `hit = max(minimum, floor(base x num / den) saturating_sub flat)` with a u128 intermediate, `minimum` on the damage table, `flat` on the unit armour, and the cell (damage type, armour class) of the table (6.3); no float takes part. A test package, not `frontier`, holds two cells where the integer form and a float form differ: damage 100 with 29/100 gives 29, and damage 50 with 106/100 gives 53; and a 2 x 2 table whose transpose differs: (bullet, light) 100/100, (bullet, heavy) 10/100, (rocket, light) 25/100, (rocket, heavy) 150/100; each shot asserts its exact damage. A package with no damage table keeps the damage of today (`hit = base`; REG2 and the M3 fault set show it). Each balance ratio of section 5 is a headless duel with the `frontier` numbers; a duel that fails is a content defect and a retune (S:807 keeps the numbers Open), not a red of the engine; the duel results go to `docs/measurements/mp1.md` [rigour 6] | (1) A float multiplier (`damage as f64 * (num as f64 / den as f64)`): the cell 100 x 29/100 gives 28; with f32 the cell 50 x 106/100 gives 52. The qa-lead probe found 146 differing cells among 80,000 (damage 1 to 400, ratio n/100 for n 1 to 200) and no cell that differs for both f64 and f32, so the row needs both cells. (2) The table read with its axes swapped: the shot (bullet, heavy) gives 25, not 10. (3) `flat` taken before the ratio: the case with flat 5 on 100 x 29/100 gives 27, not 24 | headless |
| TGT1 | An idle unit with `acquire` (6.3; on the unit, 6.10 row U4) and a ready weapon acquires the nearest enemy (distance, then unit id) inside `acquire.range` within K = `ACQUIRE_PERIOD_TICKS` (4 at the MP1 tick rate) ticks after the enemy enters the range (D-J); it chases while its distance from its idle point is at most `leash`, then goes back to the idle point. An ordered target comes first. A holding unit never moves: its position bits are equal on every tick of the scenario; it fires only at an enemy inside its weapon range. A query runs only when the weapon is ready and has no valid target, and a unit queries only in a tick t with `(t + unit_id) mod K = 0`: a query counter (the `cfg(test)` counter pattern of `crates/turts_sim/src/movement.rs:13-21`) shows 0 queries for a unit that keeps a valid target, and no unit queries twice in K consecutive ticks [perf H3]. Candidates come only from the grids of `Enemy` teams: a `turts_rules` unit test with a teams map of two slots for each team shows that an ally inside the range is never a candidate. A unit without `acquire` keeps the behaviour of today (REG2) | (1) Hold handled as idle: the holding unit chases inside its leash, and the position check fails at the first tick that it moves (`unit U on hold moved at tick T`). (2) A query in each tick, or before the readiness check as today (`choose_target` before the cooldown test, `crates/turts_sim/src/economy/combat.rs:201`, `:229`): the counter shows a query in each tick. (3) The leash ignored: the unit follows past the leash. (4) A grid built with the relation `Other`: the ally of the unit test becomes a candidate | headless |
| CON1 | The constructor queue holds each `Construct` order, never progress or charge (D-E, 6.1). A placement is refused for each cause with its reason key of 6.1, one case each, and the first failing check gives the cause: `site_outside_map`, `site_out_of_area`, `site_blocked` (an overlap with any footprint, a unit included), `site_near_building` (a gap below `placement.gap` to a static footprint), `site_near_resource` (inside `placement.node_clearance` of a node), and `queue_full` (the queue holds `queue_capacity` held orders) [player H4, net 5]. A valid placement creates a site entity in the tick of the order. From that tick the site owns the progress, the charge (PAY1) and the job, and the constructor counts only its open sites. The two caps make an order wait and never refuse it (6.1): with `concurrent_sites` 2, two sites of one constructor progress in the same ticks; a third order waits in the queue, and the engine places its site (an engine order, 6.5) in the tick after one of the two ends. With two constructors of one slot and `open_sites_per_slot` 1, the order of the second constructor waits while the site of the first is open, gets no `OrderResult` refusal, and the engine places its site in the tick after that site ends. The site completes, and the building has full health [rigour 1, mod H2] | (1) Progress advances in the constructor's queue after placement (the site is only a marker): after the placement tick the queue holds a job with progress, and the second site stays at progress 0 while the first progresses (`site S2 progress 0 at tick T`). (2) For each cause, the check of that cause off: its case is accepted. (3) The check order changed: the case with two failing causes gives the second cause. (4) The site check off: every case is accepted. (5) The slot cap refuses in place of the wait: the order of the second constructor gets a refusal, and no site appears after the first site ends | headless |
| CON2 | For a scene with no motion and no new site between the client frame and the server tick, the client verdict from its replica equals the server verdict for 1,000 seeded points [net 9]. The seeds put at least 20 points in each band where a wrong rule changes the verdict: a gap from 1.8 to 2.0 from a footprint, over a unit, inside the node clearance, and at the edge of the build area. The test counts the points in each band and fails when a band has fewer than 20, so that a new seed cannot make the row unable to go red. The client gets its verdict from `turts_rules` (section 9); `scripts/boundary.sh` refuses a placement function in `turts_client` other than the call | (1) The client uses its own copy of the rule with the gap 1.8: the band points differ (`point P: client valid, server refused`). (2) The client leaves out the unit blocker class: the points over a unit differ. (3) A copy of the placement function in `turts_client`: the boundary check is red | headless |
| CON3 | `CancelConstruction` on a site refunds `floor(charged x num / den)` with `cancel_refunds.running` (6.4). The case cancels at progress 37/100 of a cost of 500 with the ratio 3/4: the charge is 185 and the refund is 138. A cancel of a held order refunds 0. A destroyed site refunds by `cancel_refunds.destroyed`, which is 0/1 when absent: one case with it absent (refund 0) and one with 1/2 (refund 92). After the cancel and after the destruction, the claims are released in the same tick: a new site on the point passes the site check, and the open-site count goes down by 1 (a held order that waits at the cap gets its site in the next tick, 6.1). The ledger (`turts_audit`) balances after each case | (1) Refund of the full cost: 375 or 500, not 138. (2) The running ratio used for a destruction: 138 for the destroyed site, not 0 or 92. (3) The open-site count kept: the held order at the cap never gets its site (`held order H1 not placed at tick T`) | headless |
| PWR1 | Supply (`Supply(scope: Slot, counts: Completed)`, 6.2) counts units that exist and are not sites: `provides` and `consumes`, once for each tick at the start of the economy phase. In a shortage each capability applies its own `when_short`: `Rate(1/2)` on the producer and the constructor, `Disable` on the turret weapon (it neither fires nor acquires); a capability with no entry (the rifleman weapon, repair) is not affected. Recovery restores both at the first computation after the supply returns. A weapon that the player disabled stays disabled after recovery (a shortage is not `SetEnabled`). The rule reads only mod data: a second test package names its supply resource `zeta:steam` and puts `Rate(1/3)` on a different capability, and the same checks pass | (1) A check keyed on the `frontier` resource id or on the word power: the `zeta:steam` package shows no slowdown (`rate 1/1 under shortage`); NC1 is also red on the literal. (2) Sites counted: a station that is still a site adds supply. (3) The shortage implemented with `SetEnabled`: the weapon that the player disabled is enabled after recovery. (4) `Disable` that stops fire but not acquisition: the disabled turret records a target | headless |
| PAY1 | Charge over time (P4, D-A, 6.4): a job has `done` and `total = duration_ticks x Q`; the total charged at `done` is `floor(cost x done / total)` for each cost entry, with a u128 intermediate; each tick charges the difference. At completion the total equals the cost; the balance never goes below 0. A tick that cannot pay every difference charges nothing and does not advance (job state `WaitingForFunds(resource)`, the first resource in cost order that cannot pay), and the job resumes in the tick after the next delivery. The ledger has one `Charged` event for each job end (completion, cancel, entry into `WaitingForFunds`) with the sum since the last event, not one for each tick. Cases: a cost that the duration does not divide (500 over 240 ticks); a stall at 0 credits; two cost entries where only the second cannot pay; a product `cost x done` above the u64 range (cost 2^40, `total` 2^25) [perf M3] | (1) Charge at the order: the balance falls by the cost in the order tick. (2) A fixed amount `cost / duration` in each tick: the total at completion is 480, not 500. (3) A u64 intermediate: the large case overflows (an error or a wrong total). (4) One charge for each cost entry on its own: the first entry is charged while the job does not advance. (5) One ledger event for each tick: 240 events for the job, not 1 | headless |
| REP1 | Repair (6.5): the heal of each tick is `floor(rate x t / tick_hz)` minus the same value at `t - 1`, at most `max - health`; the total charged after H applied points is `floor(H x amount / per_health)`, and each tick charges the difference; a tick that cannot pay heals nothing (job state `WaitingForFunds`); a cost of 0 heals with no charge; nothing is charged at full health. Health changes only inside a transaction: (a) static: the only production writers of the health table in `turts_sim` are `crates/turts_sim/src/boot_m3.rs:110` and `crates/turts_sim/src/economy/transaction_m3.rs:218` (a `turts_checks` test with the syn scanner and its production filter); (b) at run time: an audit rule says that for each unit and tick the health change equals the `Fired` damage plus the repair heal of that tick | (1) A repair system that writes `world.resource_mut::<Healths>()` directly: the static check names the file and line (qa-lead probe: a plant in `economy/combat.rs` gives `RED health writer economy/combat.rs:83`), and the audit rule finds a health change with no event. (2) Repair at 0 credits: health rises while the balance is 0. (3) A charge at full health, or a charge for the requested heal and not the applied heal | headless |
| RLY1 | A new unit follows the rally through `apply_order` (`crates/turts_sim/src/orders.rs:2610`) with the source `Engine` [net 8]. The follow order is the smart rule of the rally target (`orders/rules.ron` `rally`, else `smart`, 6.5): a rally on ground gives the mod's ground order, a rally on a node gives a hauler its gather order. The order log (`--order-log`) records the order with the source `Engine` and no seq; it costs no token and gives no `OrderResult`; a refusal shows in `last_failure`. Two runs of the same `--command-script` give identical order logs and economy logs (`cmp`), and the economy-log replay validator (`validate_records`, `crates/turts_server/src/economylog.rs:371`) passes | A fake player envelope: the server injects the follow order as a player envelope with a seq of the slot. The order log shows the source `Player`; the client report has an `OrderResult` for a seq that the client never sent; the token count of the slot falls in the spawn tick | headless |
| MAT1 | Outcomes (X11, D-D), package `standoff` (limit 100 ticks at 20 Hz). Scripts, `--start-now --command-script`: (a) attack orders kill B's key unit at tick T: A `Victory`/`OpponentsDefeated`, B `Defeat`/`KeyUnitsLost`, `match_over_tick` T, and the tick log has no tick after T; (b) both key units die in one tick: `Draw` for both; (c) `connection: false` for both slots at tick 10: `Draw`/`Disconnected` at tick 110; at ticks 10 and 20: slot 0 `Defeat` at 110 and slot 1 `Victory` at 110; (d) slot 0 false at 10 and true at 60: no outcome at 110. Two headless clients: (e) B `surrender` at tick 40: both reports have the outcome with the reason `Surrendered` and the same `match_over_tick`, and B's key unit is alive at the end; (f) B killed: the input log has `connection: false` for slot 1 at tick D, at most 11 s after the kill, and B gets `Defeat`/`Disconnected` at D + 100; (g) B killed, then B with `--rejoin` before the limit: `connection: true`, and no outcome by D + 200; (h) the `--input-log` of (e) and of (f), replayed with `--start-now --command-script`, gives the same outcomes and `match_over_tick`; (i) `muster` (no victory data), one client killed: no outcome after 300 ticks; (j) after the end, a new id gets `The match is over`, and B (an owner that did not leave) gets `welcome` and the same `match_result`. A sim test with three slots: an order from a defeated slot gets `slot_defeated` | the client works out the outcome from the unit list ((e) and (f): `outcome` is null, because B's key unit is alive; in case (a) the two agree, so only (e) and (f) can show this red); the disconnect count is a wall-clock timer in the server ((h): the replay has no connections, so it gives no outcome); a defeat with no clause ((i), D28); a draw rule that gives `Victory` to the lower slot index ((b), (c)) | headless, two clients, and scripts |
| LOB1 | Lobby start and claim boundary (D-B), package `standoff`. Run 1, headless clients A, B, C: A seats: A's first `lobby_states` entry has `you: 0`, seat 1 `Open`, start `Waiting`, and A has no `welcome`; server `--stats` `ticks` stays 0 for 3 s. B seats: both reports show two `Human` seats with different colours. Only A is ready for 3 s: start stays `Waiting`, no `welcome`. B ready: start `Countdown`; B's `lobby_move` in the countdown gets `match_not_started`. Both `welcome` records have `tick` 0, `start_tick` 1, and equal `teams` and `colours`; the first snapshot tick is 0 for both; the tick log starts at tick 1. After the start, C (new id) gets `no open slot`; B killed, then B with `--rejoin` gets slot 1 again. Run 2: B sends `unready` in the countdown: start goes back to `Waiting` and no `welcome` comes; B `ready` again: the match starts. Run 3: A and B seated, B killed: within 11 s A's lobby state shows seat 1 `Open` (the time is recorded as L), and C gets seat 1. Pixels, two windows: the words of L2 to L5, "Match starts in 3", "2", "1", and both windows show "Click to play" | the sim claims at `Hello` (run 3: C gets `no open slot`); the sim ticks in the lobby (the `welcome` ticks are above 0 and differ, as today: 76 and 171 in the gate 1 probe); a start with one ready (run 1); an order admitted before the start (run 1: `lobby_move` gets another result); `unready` does not stop the countdown (run 2) | headless, three clients; pixels, two windows |
| RES1 | Result (D-K). Pixels, two windows: within 1 s of the end tick both windows show the outcome and the reason line (7.6 words, for example "Player 2 surrendered"); within 3 s more, the result screen shows both slots, their outcomes and reasons, and the statistics; the oracle compares each number on the screen with the `match_result` of that client. Wire check (net 10), headless, two clients: in each report `stats_first_seen_tick` equals `match_over_tick`, and each client got exactly one `MatchResult`; an export unit test in `turts_server` builds the per-slot export at each tick of a scripted `standoff` match and finds no statistics before the end tick | the sim view gives the statistics at each tick (headless: `stats_first_seen_tick` is 1; the export test is red at tick 1); a result screen without the reason (pixels); a pixels driver alone cannot show the first red, because a client can hold a value and not show it | headless, two clients; pixels, two windows |
| FX1 | A `Fired{unit, target}` event, with the weapon as an index (net 7, perf M1), reaches each client that may see the shooter or the target (both clients in MP1). In window B an enemy turret turns to its target and its muzzle flash shows; a death shows its effect; a reduced hit (table ratio below 1/2, for example a bullet on a tank) shows `hit_reduced` and a full hit shows `hit` (`turts_debug.effects.recent`). With 400 riflemen that fire, the effect pool stays at 256 and the effect entity count does not grow. After a T8 resync, no effect older than the presentation tick minus K plays | (a) today's event path: the event is staged for the shooter's slot only (`crates/turts_sim/src/economy/combat.rs:393-395`), so window B sees no enemy fire; (b) the pool grows past 256, or one entity spawns for each shot; (c) `hit_reduced` never chosen; (d) the kept deltas of a resync replay their effects: a burst after the resync | headless and pixels |
| HUD5 | Forced states in a scenario, each read as exact text: "Credits 1200"; "Power 40 used / 100 made"; under a shortage "Low power: 130 used / 100 made" with the mark; queue rows "On hold: no credits" with a striped bar, "Slowed: low power", "Waiting to start", "Waiting: exit blocked"; a disabled entry "Queue is full (5/5)". A starved job under the free income (it pays in some ticks and not in others) shows "On hold: no credits" in each of 10 frames sampled over 1 s | (a) the `Supply` fields `Consumed` and `Provided` swapped: "Power 100 used / 40 made"; (b) the row state without its 1 s hold: the words are absent in some of the 10 frames; (c) the job state `WaitingForFunds` mapped to the low-power words | pixels |
| KEY1 | Each build entry has its own key, shown in its tooltip and its build-menu row (the table of 7.4); no two MP1 default bindings are equal (the check is global; there is no action scope); no default key is in `RESERVED_BROWSER_KEYS` (`crates/turts_mod_api/src/input.rs:51-68`); in a pixels run each key starts the placement or the job of its entry | (a) today's build menu: every row shows the key of the producer's Produce order (`crates/turts_client/src/widgets/sources.rs:391-393`); (b) a package with two actions on KeyP: the packer refuses with `input/input.ron: actions: KeyP is the default of build_power_station and pause; each default key must be unique`; (c) a default on Ctrl and KeyT: refused as a browser key | headless and pixels |
| OBS1 | A harvester completes 20 trips with a building 1 to 2 from the command centre on the ore side. The placement gap is a mod number (`placement.gap`), and the packer refuses a gap below the largest mobile diameter plus 0.2 [mod L10]. With the gap of the mod, 0 of 20 tanks are `Blocked` in a compact base, and the site check refuses a smaller gap. The row runs after slice part A3 (X0e) and records the movement tick time with the command-centre footprint in `docs/measurements/mp1.md` | Today's tree is the red build (probe MP1-P1, `docs/research/mp1/probe-obstacles.md`): 0 harvester trips with a station 1 to 2 from the command centre. (1) The approach point ignores statics: 0 trips. (2) The gap rule off: 7 of 20 infantry stay `Blocked` at the gap 0.5. (3) A package with a gap below the largest diameter plus 0.2 packs with exit 0 | headless |
| DBG1 | The client builds its debug state only when the harness asks for it, at most 4 Hz (D-J) [perf H1]. With no request, a headless client at 400 units builds 0 debug states in 600 frames (a build counter in the debug module). With a request, the rate is at most 4 Hz. REG2 shows that the MC oracles still get their debug state. VIS3 records the frame time with the debug state off and on | Today's tree is the red build: `rebuild_debug_state` (`crates/turts_client/src/debug.rs:441`) runs in `Last` on every frame (`crates/turts_client/src/lib.rs:147`); the counter shows 600. Gate 1 measured 4.4 ms (min) and 5.1 ms (p50) a frame and 148 KB at 400 units (native, loaded host) | headless |
| MDL1 | The `frontier` package with models loads in the browser from the package bytes in memory: the browser network log has no request for a `.glb` path. Each check of 8.1 refuses its plant at pack time with its error text. The `shape` shows until the model is ready: with a test hook that delays readiness by 500 ms, the pixel finder sees the shape at the unit's screen point in each frame of the delay. The team material shows the slot colour | (a) a URL fetch of the model; (b) for each check of 8.1, a plant that packs with exit 0; (c) the shape removed at spawn before the model is ready: frames with no unit pixels; (d) the team material not replaced: the body colour shows where the slot colour must be | headless and pixels |
| VIS3 | Frame time p50 and p95 at 400 units, with primitives and with models, with the debug state off and on (8.5), in `docs/measurements/mp1.md` (recorded, not judged). The play-test floor (Proposed, decision D-N): p95 at or below 50 ms and p50 at or below 33 ms on the play-test machine; PT3 does not start below it | The recorded numbers have no red. The floor check is red on a build with the test hook `frame_stall_ms=60` (a busy wait of 60 ms in each frame), which shows that the check can fail | pixels |
| MCX1 | The MC items of 7.7, one check each: a Button with `font` and `size` renders in that font and size; an action with no key shows "Unbound" in the menu, the tooltip and the binding rows; the Controls menu shows its groups and Up, Down, Enter, Delete and Esc work; the alert list row pitch equals the row height plus the gap within 1 px at UI scale 1.0 and 2.0, and no pixel of a row shows above the list top; a unit with no health shows no "x1 - HP"; a partly refused group order shows "3 of 8 units: Cannot reach that place" with one error sound, and a click on the line selects the failed units | Each MC defect put back; each one exists in the MC build 8d6902c (PT1 person run 1 and the O15 list), so each red can happen | pixels |
| NOGPU1 | With no graphics adapter (Chrome started with WebGPU off), the page shows the plain text of 7.7, with how to turn WebGPU on, and the console shows no panic | The MC build 8d6902c: a panic with a console message only (PT1 person run 1) | pixels |
| PT3 | The two-person play test of section 13, on a play-test machine that passes the VIS3 floor. The clock starts when the player presses "Click to play" in that window; the 60 s rule ends at the first client update with an own Power Station site (7.1), not at a pending site. A person signs off | Red when a tester cannot find an object, an action or a reason (the tester says so aloud, or the observer sees no progress for 60 s), or when no Power Station site exists 60.0 s after the press. The timing instrument can go red: GOAL1 plant (b) moves `H1.done_ms` late and plant (d) leaves it absent, and the PT3 timing script reports red on both | people |
| REG2 | All MC rows, the fault sets, and the frozen values after the one re-freeze, on the MP1 tree, at the end of stage 1 and at the end of stage 2. Each `done:` row of the Yardstick coverage section runs with the test that its `test` cell names | Any regression. The plant that shows that REG2 can go red: `clamp_to_map` returns at once (`crates/turts_interaction/src/camera.rs:358`), and `bounds_hold` fails with `ArrowRight must reach and stop at the authored map margin: Vec2(78.50007, 32.0)` at `crates/turts_interaction/tests/camera.rs:345` (qa-lead mutation run on d729a97; the code of 1b15402 is the same; green before, red, green after the restore). The plan adds one browser-row plant (NOT RUN) | all |
| GOAL1 | In the first presented frame after the press of "Click to play", in each window: the own Command Centre is inside the viewport; it is selected (the selection panel shows "Command Centre"); the goal line says "Destroy the enemy Command Centre"; the hint line shows the H1 text of the Player path. No alert line and no alert sound come before the first input of the player. With no input, the Ore Hauler drives to the ore and the first "+300" shows before 40 s. A pixels-only driver that does only what the hint lines say presses P and left-clicks inside the build-area ring: the hint line shows the H2 text within 2 presented frames of the frame with the pending site, and a Power Station site exists before 60 s from the press (`turts_debug.hints.H1.site_ms` at most 60,000) | Plants, each red: (a) `start_selection` ignored: no unit selected; (b) the done condition of H1 wired to `Complete` instead of `Pending` or `Site`: H1 stays about 24 s after the click; (c) a state alert that counts the state of the first replica as a start: "Low power" sounds at tick 0; (d) the hint source evaluated only at the start: H2 never shows; (e) the map placement without its start order: no "+300" before 40 s | pixels, two windows |
| STO1 | A map placement with a `start_order` (a mod order of behaviour `Gather` with a node target, 6.10 row M3, decision D-F) gathers from the start tick with no client input. Two runs: (1) `--start-now` with no client; (2) `--start-now` with one headless client whose `Hello` comes at tick 100 or later. In both runs: the unit moves toward its node from the start tick (`--tick-log`: its position at tick 2 differs from its map placement; the tick log holds positions only); the first delivery comes within 1 s of the first-trip time of the model (29.7 s, `pacing.py new`); the order log (`--order-log`) has exactly one line for the start order, at the start tick, with the source `Engine`, and no line for it has a seq; the start order is not an input of the command log (6.5), and a replay of the `--input-log` (`--start-now --command-script`) gives the same delivery tick; the order costs no token and gives no `OrderResult` | (a) the field parsed and not used: no delivery in 60 s in run 1. (b) The start order sent as a player envelope: its order-log line has the source `Player` and a seq, and it shows in the admission evidence. (c) The start order applied at the first `Welcome`, not at the start tick: run 1 has no delivery in 60 s, and in run 2 the unit keeps its placement position until the `Welcome` tick (100 or later). (d) The gather state written outside `apply_order` (a boot system sets the unit's gather order directly): the unit gathers, but the order log has no `Engine` line for the start order, so the check of one `Engine` line fails | headless |
| GHO1 | P, or a click on the build entry, starts placement; the server order log has no new line. The ghost follows the pointer, and the build-area ring of the own Command Centre shows (radius 12.5). Over a valid point: a solid ghost ring in the slot colour. At one fixed point for each cause on a test map: stripes, the `forbidden` cursor, and the exact text of 7.2 at the pointer: "Outside the map", "Too far from your base", "Something is in the way" (over a unit, and over a building), "Too close to a building" (the gap only: the gap rings of the ghost and of the building show in vermilion), "Too close to ore" (the clearance ring shows). Right click and Esc end placement: no new order line, and the credits do not change. Left click places: a pending site shows in the same frame; after the server result a world progress bar grows on the site until it is complete. Shift and left click places and keeps the ghost. Mismatch case (a scenario unit walks onto the point after the client verdict): the pending site goes, "Something is in the way" shows at the site point, one error sound plays, the credits do not change, and the hint does not advance | (a) the gap folded into the overlap check (the rev-3 text): the gap-only point shows "Something is in the way"; (b) the old ore text "Cannot build on ore"; (c) the world style `build_area_ring` or `gap_ring` not drawn: no ring pixels at radius 12.5, or none round the building at the gap-only point; (d) the smart path runs before the placement mode: a right click in placement sends a move (an order line); (e) the client reads a fixed gap of 0.5, not `placement.gap`: the ghost is valid at the gap-only point and the server refuses | pixels |
| PRD1 | The Command Centre runs one site from its constructor queue and one job from its producer queue at the same time (decision D-E). A second `Construct` order waits as a held order ("Waiting to start") while one site is open (open-site cap 1, mod data) and gets its site when that site completes. With one open site and 5 held orders (`queue_capacity` 5), the next `Construct` order is refused with `queue_full` (the open site has left the queue, 6.1). A new unit appears at an exit point of the producer. With every exit point blocked, the complete job waits ("Waiting: exit blocked"), charges nothing more, and the unit appears when an exit point is free | (a) the producer advances only its first queue with an active job: the unit job stops while a site progresses; (b) the open-site cap not counted: two sites progress at once; (c) the exit check ignores static footprints: a unit appears inside the footprint of the Command Centre | headless |
| INC1 | The Command Centre adds 3 credits each second (mod data `income`, decision D-L) only while it is complete and has health above 0; each period is one ledger event, so `turts_audit` balances; a slot with 0 credits and no Ore Hauler that queues a hauler gets it in at most 170 s (section 5: 166 s); a destroyed Command Centre stops the income in the same tick | (a) the income added outside a ledger event: the audit is red; (b) income from a site or from a destroyed Command Centre; (c) a float rate: the total after 600 s is not 1,800 | headless |
| PAY2 | Cancel of production under charge over time (D-A, 6.4): a cancel of the running job refunds `floor(charged x num / den)` with `cancel_refunds.running` (cancel at 37/100 of a cost of 500 with the ratio 3/4: refund 138); a cancel of a queued job refunds 0, because it charged 0; a cancel of a complete job that waits for a free exit refunds by `cancel_refunds.output_blocked` of the full cost. Each refund is one `Refunded` ledger event, and the ledger (`turts_audit`) balances after each cancel | (1) A refund base of the full cost: the running cancel refunds 375, and the queued cancel refunds 375. (2) A refund for a job that charged nothing | headless |
| DES1 | The one death path (`remove_unit`, `crates/turts_sim/src/economy/transaction_m3.rs:354`, called from `crates/turts_sim/src/economy/combat.rs:405`) removes a destroyed unit that has health and a queue (VAL1). In one operation in the tick of the death (6.4): it cancels the running and queued jobs of a producer and refunds each by `cancel_refunds.destroyed`; it drops the held `Construct` orders of a constructor (refund 0), and the open sites of that constructor continue; it releases every claim; it removes the unit from the UnitId index (section 9) [perf H2]; and it frees the footprint, so a site on the same point passes the site check and a unit moves across the point. The ledger balances. The test package has different `destroyed` and `running` ratios (1/2 and 3/4). Static: the only production despawn in `turts_sim` is `crates/turts_sim/src/economy/transaction.rs:747` | (1) A second removal path, a death system that despawns the entity: the static check names it (qa-lead probe: `RED despawn writer economy/combat.rs:86`). (2) Jobs left on a removed unit: the queue table holds a job for the removed id. (3) The open sites removed with their constructor. (4) The footprint kept: the site on the point is refused with the overlap cause. (5) The index not updated: a lookup of the removed id gives an entity. (6) The running ratio used for a death: the refund is 3/4 of the charge, not 1/2 | headless |
| ALR2 | The rules of 7.9 give their lines with the exact texts, sounds and pings: event alerts (Construction complete, Unit ready, Base under attack, Ore Hauler under attack, Ore Hauler destroyed, Power restored, the return of a slot) and state alerts (Low power, Not enough credits, Ore Hauler idle, a lost connection). From tick 0 to the first input of the player: no alert line and no alert sound. A state alert: one line and one sound when the state starts; no sound while it holds; it ends only after `clear_after` s clear; a new start inside `cooldown` shows the line with no sound. Forced case A (headless and pixels): 0 credits, a tank job, the free income on, and one hauler that delivers every 35 s, for 300 s: at most one "Not enough credits" sound in each 90 s window (`turts_debug.audio`). Forced case B: a hit on an Ore Hauler gives "Ore Hauler under attack" and not "Base under attack". A click on a line moves the camera look-at to within 2 world units of the place of the alert; a click on "Ore Hauler idle" also selects the hauler | (a) the rev-3 rule (one sound at each start of the state) in place of 7.9: case A gives more than one sound in a window (run S8 of section 5: 89 starts in 300 s); (b) the state of the first replica counted as a start: "Low power" sounds at tick 0; (c) the `units` filter not used: a hauler hit gives "Base under attack"; (d) the alert row a text and not a button: a click does nothing (today no alert row takes a click) | headless and pixels |
| HUD6 | The tooltip of each build entry, exact: the name; the role line of 7.10; "Cost N credits, paid while it builds"; "Build time T s", or under a forced shortage "Build time T s (2T s now: low power)"; the power change ("+100 power" or "Uses 30 power"), and "This causes low power" when it applies; the key ("[P]"). A disabled entry names its cause ("Queue is full (5/5)"). The info panel of one selected unit shows its armour ("Armour: Vehicle"); for the Ore Hauler also "Load 150/300 ore" and the activity ("Harvesting", "Returning", "Unloading", "Waiting: field empty", "Idle") | (a) the tooltip time from the base time only: under forced low power the Power Station tooltip says 12 s and the job takes 24 s; (b) `Role` not bound in the `frontier` tooltip: no role line; (c) the `Reason` field empty for `queue_full`: a disabled entry with no cause | pixels |
| SMT1 | MP1 smart rules and cursors. Command Centre selected: a right click on the ground sets the rally (a flag and a line show; no unit moves; no notice); a right click on ore sets a gather rally. Engineer selected: over a damaged own building the cursor is `repair`, and a right click repairs it (the health bar rises); over an undamaged own building the cursor is the default cursor. Ore Hauler selected: over ore the cursor is `gather`. In placement the cursor is `place` or `forbidden`. The oracle reads `turts_debug.cursor` for each context | (a) the smart table puts Move before SetRally: a right click on the ground with the Command Centre selected gives the notice "cannot move"; (b) the repair context tests "own" and not "own and damaged": the `repair` cursor over an undamaged building; (c) one cursor for the placement contexts: `place` over an invalid point | pixels |
| AUD2 | The sound set plays from the mod package: fire, hit, reduced hit, death, construction complete, low power, unit ready, error, and the delivery sound with each "+N" (one for each delivery). A weapon sound is positional: a shot far from the camera look-at point has a lower gain than a near shot, and a shot beyond `audible_range` has gain 0. With 40 units that fire at once, no more effect sounds play at once than `effects_voices` (`turts_debug.audio`) | (a) the effect sound played at the listener: equal gain near and far; (b) the cap counted for each sound id: with two weapon sound ids, twice the cap plays; (c) an effect that names a sound the package lacks: the packer refuses with `effects/frontier.ron: rifle_fire.sound: unknown sound `frontier:rifle_shot`; did you mean `frontier:rifle_shots`?`; (d) the "+N" float with no sound | pixels |
| LOB2 | Colour choice. Headless, two clients: at seating A holds colour 0 and B colour 1 (the lowest free index). B sends `set_colour` 0: B's `lobby_refusals` has `colour_taken`, and the last lobby state in both reports still has A 0 and B 1. A sends `set_colour` 2: both reports show A 2, and A's ready flag is cleared. The `welcome` `colours` of both clients equal the last lobby state. Pixels, two windows: L4 (the swatch of B's colour shows a cross and the tooltip "Player 2 has this colour"; a click on it changes nothing); after A picks colour 2, the units, rings and minimap marks of A have palette colour 2 in both windows | the server does not check the colour (headless: both seats hold 0 after B's `set_colour`; a pixels driver cannot reach this red, because the crossed swatch does nothing on click); the client takes the slot colour from the palette by slot index and not from `MatchStart.colours` (pixels: A's rings show palette colour 0); two seats get one default colour | headless, two clients; pixels, two windows |
| SUR1 | The menu item "Surrender" shows "Surrender and end the match? You will be defeated." Cancel: the server `--input-log` has no surrender envelope, and the match goes on for 5 s. Confirm: one surrender envelope; the server ends the match at the tick of that envelope; within 1 s both windows show the outcome (B "Defeat", A "Victory") with the reason, then the result screen. In each window, the first frame that shows the outcome has a replica tick at or after `match_over_tick` (frame log) | surrender with no confirmation (the envelope goes at the first click); the other window shows no result (`MatchResult` sent only to the slot that surrendered); the outcome shows before the server end tick (the client decides the outcome) | pixels, two windows |
| IMM1 | Default start policy `Immediate` (D-B). A package with no `exports.match` boots to Running: `muster` with no client has `--stats` `ticks` above 0 after 3 s, and a `Hello` gets `welcome` at once with `tick` above 0. The 31 ignored process tests of `crates/turts_server/tests`, hd1, T7 and the MC rows pass with no change to their commands. `standoff` with `--start-now` and a `--command-script` runs the script with no client. A `^0.2.3` package that writes `exports.match` is refused with "`exports.match` needs engine api 0.2.4; `^0.2.3` admits 0.2.3"; a `^0.2.3` package with an order of behaviour `Surrender` is refused with its row name | the default policy is a lobby (`muster` has 0 ticks; the `mc_group` tests time out); `--start-now` ignored (the `standoff` script run has 0 ticks); a version-table row missing (the `^0.2.3` package packs) | headless |
| LEAVE1 | One match for each process (D-C, D-K). Headless, two clients finish a `standoff` match, and each runs `leave`: the server exits with 0 within 2 s of the second `LeaveMatch`. Before that exit, a client that left gets `The match is over`, and an owner that did not leave gets `welcome` and `match_result`. With no `leave`, the server exits with 0 at `after_match_secs`. `scripts/mp1-serve.sh` starts a new process after exit 0; a new `Hello` gets a lobby seat, and the `welcome` of the next match has a different `match_id`. With a plant `server.ron` that names a missing `game_package`, the script stops with exit code 1 and does not start again | the server stays in Ended (no exit); a world reset in place (the same `match_id` after the restart); the script restarts after a non-zero exit; an owner that did not leave gets `The match is over` | headless and the script |
| TGT2 | An idle unit with `acquire` that takes damage targets its attacker when the attacker is inside its leash, also when the attacker is outside its acquire range (a rifleman with acquire range 7 under a turret with range 8). It takes the attacker, not the nearest enemy: the scene has an enemy with no weapon at 7.5 and the attacker at 8. When two attackers hit it in one tick, it takes the lower unit id (6.3). A holding unit that takes damage never moves; it fires back only when the attacker is inside its weapon range (D-G) [rigour 5]. A unit with an order that is not Hold (Move, Attack-move, Attack) keeps its order and the fire policy of that order (MC: `move_never_policy_suppresses_mobile_weapon_fire`, `crates/turts_sim/src/economy/combat.rs:649`; `attack_move_stops_to_fire_at_a_nearby_target`, `:1076`); when the order ends, the idle rule applies. This is the interrupt rule of coverage item OR26. A unit with no weapon, or with a disabled weapon, does nothing | (1) Retaliation ignores hold: the holding unit moves toward its attacker (`unit U on hold moved at tick T`). (2) Retaliation interrupts a player Move: the current order is no longer the Move before arrival. (3) Retaliation takes the nearest enemy: the target is the enemy at 7.5 (`expected target A, got D`). (4) The first or the higher attacker id taken when two hit in one tick | headless |
| ORDW1 | `unit_orders` travels as changes (6.8) [net 4]: a delta carries the complete record of each unit whose record changed, and `orders_cleared: Vec<UnitId>` for each record that left the view; the Snapshot keeps the full table; the replica merges in chain order only (`crates/turts_client/src/replica.rs:237-239`). Scenario: a plain move ends on a unit that then has no `UnitOrders` (then the sim gives no record: `crates/turts_sim/src/economy/views.rs:558-568`); the client drops the delta that clears it (T8 hook `--drop-delta-ticks`), then resyncs. After the resync the client shows no current order for the unit. With 200 idle units for 100 ticks, the `unit_orders` bytes of each delta are 0. The bytes of `unit_orders` for each tick go to `docs/measurements/mp1.md` (gate 1 measured 2,181 to 4,281 B for each tick with the full table, 64 to 83 % of the delta) | (1) No `orders_cleared` list: the client keeps the stale current order after the resync (today each delta carries the full table, and the replacement clears the record: `replica.rs:523-525`). (2) The full owner table in each delta: the idle bytes are above 0 | headless, two clients |

## 13. Play test

- Who: the `player` agent plays first, through the harness, with the goals text only. Then two people play in two
  windows or on two machines; one of them is not the CEO, so one tester is blind (decision P7). One person signs off.
- Machine: it passes the VIS3 floor (8.5).
- The observer has the Player path; the testers do not. The observer answers no question about the game, reads nothing
  to the testers, and writes the time, the step and the words each time a tester says that they do not know what to do.
- Goals text, match 1. Both testers get this text only [player 8]:

  "Play one match against the other person. Win: destroy the enemy Command Centre. Use only the mouse and the keyboard.
  Say aloud what you try to do, and say when you do not know what to do next. The observer does not answer questions
  about the game. Stop at the result screen, or after 40 minutes."

  "Command Centre" is the word of the goal line on the screen (7.8). The text does not name power, ore, units or
  turrets: the hint lines must teach them.
- Match 2, after match 1 [player 11]. To A: "Play a second match. When you want to, open the menu (Esc) and choose
  Surrender." To B: "Play a second match as usual."
- Match 3 [player 11; decision D-H]. To B: "Play a third match. After about two minutes, click the reload button of
  the browser (the round arrow) in this same tab, and play on. If the pointer stays inside the game, press Esc first.
  Later, close this tab and do not come back." To A: "Play a third match as usual." The browser keeps the player id in
  sessionStorage (`crates/turts_client_web/src/lib.rs:242`, `:266`): a reload in the same tab keeps it, and a closed tab
  loses it (gate1-network.md:27).
- Each match is a new server process (decision D-C). Between matches the testers reload the page; the "Leave" screen
  says so (7.6).
- Where a first-time player most likely stops, and the design answer:
    1. The player does not find building: the Command Centre is selected at the start, the goal line shows, and hint H1
       names the key and the ring.
    2. A silent stall from no credits or low power: the Ore Hauler gathers from tick 0; the numbers of section 5 give no
       hold above 4 s in the ordinary scripts; the row states; the alert rule of 7.9 (one sound for each state start,
       never at tick 0); the situation hints X1 to X3.
    3. A red ghost with no cause: the reason names the real cause, a ring shows the gap, and right click and Esc leave.
    4. The player does not see who beats whom: the role lines and the reduced-hit effect (7.10).
- Record for each tester: the hint times of `turts_debug.hints`, each spoken stop with its time and step, the result
  screen, and the answer to "Would you play a second match?".

## 14. Risks and the early tests

Updated with what gate 1 measured. "Early" is the first two days of the stage that holds the item.

| Risk | Early test |
| --- | --- |
| Units cannot pass buildings | Done: probe MP1-P1 gives PASSABLE WITH LIMITS (section 6.9). Today's tree is the red build of OBS1 (0 harvester trips with a station 1 to 2 from the command centre). The engine measure of 6.9 is the first OBS1 task in stage 1 |
| The lobby stops the existing instruments: 31 ignored process tests in `crates/turts_server/tests` (mc_group 19, unit_boot, det1, el1, command_script, t4, t5, t6b, t8) and hd1, T7 and the MC rows start with 0 or 1 client, and `--run-ticks` needs the tick to advance (backend H3) | Lane 0 sets `Immediate` as the default start policy, with its version-table row (D-B). Test: the 31 tests, hd1 and T7 on the lane-0 build; red if one test waits for a lobby [backend H3] |
| The client debug state costs 4.4 ms (min) and 5.1 ms (p50) a frame and 148 KB at 400 units, and 11.3 ms at 1,000 units (native, loaded host; NOT TRACED in the browser; perf H1) | DBG1 in lane 0: today's tree is its red build. VIS3 records the frame time with the debug state off and on [perf H1] |
| A command-centre footprint makes movement 4.6 times slower until slice part A3 lands: 5,000 movers 42.7 to 195.5 ms a tick, 1,000 movers 6.9 to 11.0 ms (perf H4) | A3 is an entry condition (X0e, D-J). Test: the perf probe again on the tree with A3; record the ratio in `docs/measurements/mp1.md` (no threshold: S 15) [perf H4] |
| Idle acquisition at scale: an idle tick of 45.2 ms at 5,000 units with 2,500 armed (range 9), against 13.5 ms with no weapons (perf H3) | The TGT1 query counter; the 5,000-unit tick of exit item 5 [perf H3] |
| The hosted runner has no GPU (decision note of 2026-11-11, open). The browser client runs at about 1 to 4 frames a second without a GPU | The MP1 pixel rows run on the local GPU gate with receipts until the CEO decides. Test: one MP1 pixel row on the hosted runner, to record the result |
| One GPU for a two-client match | Two headed clients for 10 minutes with combat; frame p95 and the wait for the GPU lock |
| The numbers do not end a match, or end it too fast. With two harvesters the own fields last 11.4 min; two players that defend can stall until the ore runs out at about 20 to 25 min (player 10) | A scripted headless match with the draft numbers; record the end time and the ore-out time. A stall is a retune of the numbers (S:807), not an engine defect [player 10] |
| The server detects a disconnect late. Gate 1 saw +14.5 s, which was not a clean measure; the repair probe of 6.7.8 gives 9.1 to 10.0 s (native, debug build; the netcode timeout is 10 s). The browser lag is NOT TRACED (backend M7) | MAT1 counts the 120 s from the end of the server session, and LOB1 and MAT1 record L. Test: the same probe on a release build, and a browser measure of L before the step-19 test (section 15 point 10) |
| The lobby and screens overrun, as the client shell did in MC (22 runs against 6) | Stage 2. A pixels-only stub: two clients reach the lobby, press ready, the match starts; red if the stub needs more than 4 runs |
| Models cost too much frame time | Stage 2: the 400-unit measurement of section 8.5 |
| Art production has no natural end | The cap of decision P1 and the shape fallback |
| The re-freeze and the audit change have a long tail of moved fault plants | The slice gate runs the fault sets with the replay validator disabled |
| `turts_audit` is not in today's tree (no hit in `crates/` or `Cargo.toml`); PAY1, PAY2, CON3, DES1 and REP1 need it | These rows start after slice part P1 is merged |
| The completeness gate checks form only: a `done:` path to any file that exists passes (`done:Cargo.toml` exits 0, qa-lead run) | The proposal R7b (section 15). Until it lands, qa-lead checks each `done:` token with a script at each gate |

## 15. Open points after gate 1

Only points that are truly open. The decisions with options are in section 2.

1. The engine measure of section 6.9: the estimate of about 40 lines in one file is NOT TRACED.
2. The pre-MP1 slice: its plan gate passed up to task A2a (commit 4bc157e). Parts A3 and P1 are entry
   conditions of MP1 rows (X0e; PAY1, PAY2, CON3, DES1, REP1). Its findings can change section 9.
3. The tick rate and the numeric budgets stay Open (S 15). The value K of D-J and the 4 Hz rate of the debug
   state are design values, not budgets. MP1 records numbers and promises none. The VIS3 floor before the play
   test is decision D-N of section 2.
4. The `frontier` numbers stay Open until measured (S:807). A balance duel of DMG1 that fails is a retune.
5. The MP2 row asks for remembered sightings, but fog comes at M5 (PR part 3): a question for the MP2 design.
6. MC is not closed (PT1 person run 1 failed; Tasks 23 and 24 are open). MP1 starts from the MC integration
   tree, and REG2 runs on that base. A fix in MC can move the base.
7. S:727 says M5 "Needs teams (MP1)"; with P2 (a) MP1 gives two teams of one, not team choice. The
   specification text needs an update after MP1, or the M5 spec brings team choice.
8. The second mod (mod L12) meets limits that MP1 records as additive later: square footprints, batches, and hold
   a point. Sections 6.4 and 6.5 decide the others: "a tick charges all costs or none", a repair cost of 0 is
   legal, the cost for each health point is an integer ratio, and `repairer` is not a work capability.
9. Proposal R7b for `scripts/qa/completeness.py` (qa-lead, not applied; the orchestrator decides): a `done:`
   row must name, in its `test` cell, a token that occurs in the evidence file. Two lines after
   `completeness.py:123`:
   `elif not target.is_file() or test in ("", "-", "none") or test not in target.read_text(encoding="utf-8", errors="replace"):`
   `red.append(diagnostic("R7", file_name, item, "done evidence {} does not name test {!r}".format(disposition.split(":", 1)[1], test)))`.
   The same commit fills the `test` cell of the 15 `done:` rows of the MC design and of the 14 `done:` fixture
   rows of `tests/qa/completeness/`, and adds case 22 (`mut-r7-token.md`: a `done:` path to a file that does
   not hold the named token; expected R7). Scratch proof: self-test 22/22 and `--enforce` exit 0 with the
   fill; today's script fails case 22.
10. The disconnect notice L in the browser (backend M7, browser part): whether a reload or a tab close sends a
    clean close is NOT TRACED; the native probe of 6.7.8 gives 9.1 to 10.0 s. Owner: `backend-engineer`, with
    `web-platform-engineer`. It must close in stage 2, before the step-19 test: the plan adds a browser measure of
    L, and step 19 breaks if L is above 13 s (6.7.8).
11. A pack check that each building of a map can be placed at one point at least (mod M7, proposed text (c):
    "no point of map `twin_fields` satisfies build_area 10 and gap 2.0"). Today K3 refuses only a package in
    which no unit declares `build_area`. Owner: `mod-runtime-engineer`. It must close at gate 2: the plan takes it
    as a stage-1 task of lane 0a, or drops it with a reason.
12. The replica cost of perf M2 beyond `health_trace`: the clone of the candidate for each delta
    (`crates/turts_client/src/replica.rs:432-443`, `:451`; the third site that perf M2 names, `:1090`, is NOT
    TRACED as a clone) and `replace_keyed` (`:1029`), O(changes x records). MP1 records `apply_delta` p50 and p99
    (exit item 5) and prunes `health_trace` (section 9). Owner: `network-engineer` (the client replica). It must
    close at gate 2: lane 0a takes the fix, or the plan moves it to M7 with the recorded numbers.
13. The texture and heap estimate of perf L3 (about 16 MiB of textures, a heap growth of about 35 MiB): an
    estimate, NOT TRACED. Owner: `bevy-rendering-engineer`. It closes in stage 2, at the first presentation task
    of 8.5, before row MDL1 runs.

The sweep of the gate-1 Medium and Low findings that sit in sections that no repair part owned (reconcile pass).
Each one is repaired in the text or is an open point above:

| Finding | Disposition |
| --- | --- |
| player 5 (the loss of the only hauler) | repaired: 7.9 "Ore Hauler under attack" and "Ore Hauler destroyed", hint X1, the free income (D-L, INC1), ALR2 case B |
| player 6 (who beats whom) | repaired: the role lines of 7.10, `hit_reduced` (8.3, 6.10 row V4), rows HUD6 and FX1 |
| player 7 (the tooltip time under low power) | repaired: 7.10, `BuildTimeNow` (6.10 row H9), HUD6 plant (a) |
| player 8 (the goals text) | repaired: section 13 (goal and test rules only; the words of the screen) |
| player 11 (matches 2 and 3) | repaired: section 13, one instruction for each match |
| mod M5 (`Supply` shaped to `frontier`) | repaired: 6.2 (`counts`, `when_short`, `BlockStart` reserved), 6.10 rows S1, U5, U6, H4 |
| mod M6 (weapon identity) | repaired: `acquire` on the unit (6.3, 6.10 row U4), `Fired.weapon` as an index (6.8), a missing cell is a pack error (C3) |
| mod M7 (error texts) | repaired: the error-text column and the "did you mean" rule of 6.10, U3 (damage 0); proposed check (c) is open point 11 |
| mod M9 (the asset brief) | repaired: 8.1 (pack checks with texts and advice), 6.10 rows V1 and V2 aligned with 8.1 |
| net 6 (visibility class of each record) | repaired: the table of 6.8, and the visibility predicate of the placement check (6.1) |
| net 7 (an effect burst after a lost delta) | repaired: 6.8 (no effect from a resync batch, and the age bound K of 8.3), FX1 plant (d) |
| net 11 (no client prediction) | repaired: 6.8 "No client prediction" |
| backend M5 (no player id on the wire) | repaired: 6.7.7, 6.7.13, 6.8 |
| backend M6 (the message order; the browser keeps the id from the seat) | repaired: 6.7.7, 6.8 |
| backend M7 (late disconnect notice; no headless rejoin) | repaired: 6.7.8 (L, the probe), `--rejoin` (6.7.12), 7.6 and section 14 aligned; the browser part is open point 10 |
| backend M8 (hooks and instrument reds) | repaired: 6.7.12, rows LOB1, LOB2, MAT1 |
| backend L9 (persistence) | repaired: 6.7.11 |
| backend L10 (hosting; boot time) | repaired: 6.7.10, 6.7.11, exit item 5 |
| perf M1 (wire) | repaired: 6.8 (`unit_orders` as changes, records with a start tick and a rate, `Fired` with an index), ORDW1 |
| perf M2 (the replica cost) | repaired in part: `health_trace` pruned (section 9); the rest is open point 12 |
| perf M4 (string parsing per tick) | repaired: section 9 "String ids become indices at install" |
| perf M5 (a full view and diff each tick) | repaired: section 9 "New records enter the committed tick from their writer" |
| perf L1 (the effect pool) | repaired: 8.3, FX1 plant (b) |
| perf L3 (memory) | repaired in part: 8.5 (placeholder models at the D-M caps; the wasm high-water, exit item 5); the estimate is open point 13 |

## 16. Cover that the yardstick does not ask for (from the coverage work)

The yardstick marks an objectives panel as COMMON, not CORE, so the completeness gate never asks for a visible goal
or a first action. That is the cause class of the CEO's failed MC play. MP1 covers it with its own rows:

- GOAL1, STO1 and PT3: the goal line, the selected Command Centre, the Ore Hauler that gathers from tick 0, the hint
  list of 7.8 with a defined "done", and the 60 s rule of 7.1.
- ALR2: the two alert kinds of 7.9. A state alert (low power, no credits, an idle Ore Hauler, a lost connection)
  sounds when the state starts, never at tick 0, never while it holds, and not again inside the rule `cooldown` when
  the state clears and comes back. An event alert (unit ready, construction complete, under attack) sounds at most once
  in each `cooldown`. MC row ALR1 stays green while a state fires again at each cooldown, which is how "it beeps non
  stop" passed MC.
- Refusals (OR22): a refused order names a cause that the player can act on, near the pointer. GHO1 covers placement
  with one reason for each real cause and the rings that show it; MCX1 covers the grouped notice; the same rule holds
  for every refused order.
- The floor (INC1): the free income of the Command Centre, so the loss of the only Ore Hauler is not a silent loss.
- TGT2: an idle unit that takes damage targets its attacker inside its leash; a holding unit fires back only inside
  its weapon range and never moves (decision D-G). Without it, a unit that is hit from beyond its acquire range does not
  react. With the repaired numbers no `frontier` weapon outranges an acquire range, but a later mod needs the rule, and
  the hold case needs it in MP1. X2 takes this.
- Sound (AUD2): effect sounds need a small engine part (an effect names a sound; the engine plays it at the place of
  the effect, with a cap); 8.4 says the same. X14 takes the engine part.
- Reading a fight (HUD6, FX1): role lines, the reduced-hit effect, and the build time at the current power.
- The play-test floor (VIS3): a frame time that a person can play with, before PT3 starts.
- Lobby (decision P2 a): teams and team choice stay later; the starting credits setting is not in MP1.
- 57 MC rows are `done` and rest on REG2 alone. MC is not closed: its person play failed and Tasks 23 and 24 are open.
  MP1 gate 2 checks again that each `done` path is green on the tree that MP1 starts from.

## Yardstick coverage

One row for each of the 181 CORE items of `docs/qa/yardstick-a.tsv`, in file order. An `in` row names a row
of section 12 in the `test` cell. A `done:` row names the file that holds the evidence of an earlier
milestone, and its `test` cell names the test, the assertion function or the clause in that file that tests
the item; the note gives the line. The `test` cell of a `done:` row is not checked by today's gate (the
gate checks only that the path exists: `scripts/qa/completeness.py:118-121`), so the qa-lead checked each
token with a script: all 67 tokens occur in their files (run log in the gate-1 repair report) [rigour 8].
Where the earlier evidence did not test the item, the row now names other evidence (CA13, CA16, PC17,
PC23, UI12, AN06) or a later milestone (PE04) [rigour 8]. MC acceptance is not closed (PT1 person run 1
failed on 2026-11-11; Tasks 23 and 24 are open), so REG2 runs every MC row again, and a red MC row makes
MP1 red. A `later:` row names a milestone after MP1 in `docs/qa/milestones.tsv`; the note gives the reason
and the MP1 part with its row, if MP1 builds a part. Where a CORE dependency of an item is later than MP1,
the item is `later:` even when MP1 builds its flat-ground part. This rule now also holds for `done:` rows:
MP07 and, by rule R4 of the gate, MF07 are `later:M4` [rigour 8]. Each row whose disposition depends on a
decision of section 2 names the decision in its note (P1 to P9, D-A to D-N, Q1, Y1) [rigour 8]. A different
choice changes the rows that the Changes column of section 2 lists.

| id | disposition | test | item | note |
| --- | --- | --- | --- | --- |
| IN01 | done:tests/browser/sel1-oracle.js | assertPhysicalClick | Screen-to-world picking | MC SEL1: the physical click at the projected screen point of a unit selects that unit (assertPhysicalClick :167, assertSelection :153); REG2 runs it again |
| IN02 | done:tests/browser/shell-oracle.js | judgeKm1 | Action mapping layer | MC KM1: clause keyq-bound-action, a rebound key drives its mapped action; REG2 runs it again |
| IN03 | done:crates/turts_interaction/tests/km3.rs | shift_click_toggles | Modifier semantics | MC KM3: shift_click_toggles (:9), shift_drag_adds (:36), exact_pointer_chord_precedes_default_unmodified_action (:301); REG2 runs it again |
| IN04 | done:crates/turts_interaction/tests/km3.rs | two_clicks_within_400ms_and_4px_double | Double-click detection | MC KM3: :104; the negative case two_clicks_500ms_apart_are_two_singles (:155); REG2 runs it again |
| IN05 | in | KEY1 | Hotkey per command button | MC HUD2. MP1: one key for each build entry |
| IN08 | done:tests/browser/shell-oracle.js | judgeKm2 | Browser input capture | MC KM2: clauses contextmenu-prevented-delta, outside-key-stopped, canvas-blur-overlay; REG2 runs it again |
| IN09 | later:M9 | - | Text input mode | as MC |
| PC01 | done:tests/browser/sel1-oracle.js | assertSelection | Single select | MC SEL1: one click selects one own unit (:153); REG2 runs it again |
| PC02 | done:tests/browser/sel-oracle.js | assertBoxSelectionCenters | Box (drag) select | MC SEL2: stage sel2-500-slot0 selects 500 units by a drag (rows/SEL2.json); REG2 runs it again |
| PC04 | done:tests/browser/sel-oracle.js | assertModifiers | Shift add and remove | MC SEL2: stages sel2-shift-add and sel2-shift-remove; REG2 runs it again |
| PC05 | done:tests/browser/sel-oracle.js | assertTypeSelectionIsVisible | Type select on screen | MC SEL3; REG2 runs it again |
| PC08 | done:tests/browser/shell-oracle.js | ctrl-1-assign | Control group assign and recall | MC SEL4: clauses ctrl-1-assign and 1-recall; REG2 runs it again |
| PC09 | done:tests/browser/shell-oracle.js | shift-1-add | Control group add | MC SEL4: clause shift-1-add; REG2 runs it again |
| PC10 | done:tests/browser/shell-oracle.js | double-tap-centre | Group double-tap centres camera | MC SEL4: clause double-tap-centre; REG2 runs it again |
| PC16 | done:tests/browser/sel1-oracle.js | sel1-foreign-selected | Inspect foreign entities | MC SEL1: stage sel1-foreign-selected (:37) selects a foreign turret and shows its panel; REG2 runs it again |
| PC17 | done:crates/turts_pack/tests/forge_cases.rs | c17_ownership_and_policy_rejections_are_exact | Ownership gate | was ORD2; ORD2 checks a selection filter (feedback-oracle.js:63), not the server gate. c17 asserts the exact ownership refusals; REG2 runs it again |
| PC19 | done:tests/browser/shell-oracle.js | judgeSel4DeathPrune | Selection hygiene | MC SEL4: clause same-frame-destroy-prune (:316); REG2 runs it again |
| PC20 | done:tests/browser/sel1-oracle.js | assertHoverWait | Hover highlight | MC SEL1 (:242); REG2 runs it again |
| PC21 | in | SMT1 | Context cursor | MC HUD4. MP1: place, repair, rally cursors |
| PC22 | in | SMT1 | Smart command | MC ORD5. MP1: rally and repair smart rules |
| PC23 | done:tests/browser/sel1-oracle.js | assertSelectionRingPixels | Selection visual | was VIS1; vis-oracle.js samples body colour only. MC SEL1 asserts the ring pixels (:176); REG2 runs it again |
| OR01 | done:tests/browser/orders-oracle.js | judgeOrd5 | Move | MC ORD5; REG2 runs it again |
| OR02 | done:tests/browser/orders-oracle.js | stop-after | Stop | MC ORD3: Stop clears the current order and the tail (:157-161); REG2 runs it again |
| OR03 | done:tests/browser/orders-oracle.js | judgeOrd4 | Attack target | MC ORD4; REG2 runs it again |
| OR04 | done:tests/browser/orders-oracle.js | judgeOrd4 | Attack-move | MC ORD4; REG2 runs it again |
| OR05 | done:tests/browser/orders-oracle.js | force-move-destination | Force move | MC ORD4 (:22, :413); REG2 runs it again |
| OR08 | in | TGT1 | Hold position | D-G: hold never moves, fires only inside weapon range. MC ORD3 |
| OR11 | done:tests/browser/orders-oracle.js | judgeOrd3 | Order queue per unit | MC ORD3: current order and a nonempty tail (:159); REG2 runs it again |
| OR12 | done:tests/browser/orders-oracle.js | route-queued | Shift-queued orders | MC ORD3 (:96, :111); REG2 runs it again |
| OR15 | in | RLY1 | Rally point | P3 (first cut if the schedule slips). Visible flag: SMT1 |
| OR16 | in | CON1 | Build order | D-E: the constructor queue holds the order until placement; the site owns progress. A worker that walks to the site: not in MP1 |
| OR17 | done:tests/browser/orders-oracle.js | judgeOrd5 | Gather order | MC ORD5; REG2 runs it again |
| OR20 | later:MB | - | Ability order | as MC |
| OR22 | in | MCX1 | Order validation feedback | MC ORD6. MP1: grouped failure notice (O14); also CON1 reasons |
| OR23 | done:tests/browser/feedback-oracle.js | judgeOrd7 | Order acknowledgement | MC ORD7: marker in the first pointer-release frame (:157); REG2 runs it again |
| OR24 | done:tests/browser/feedback-oracle.js | judgeOrd2 | Group order fan-out | MC ORD2; REG2 runs it again |
| OR25 | done:tests/browser/gm40-oracle.js | assertRestWindow | Group destination spread | MC GM40 (:132); REG2 runs it again |
| OR26 | in | TGT2 | Order state machine | D-G. Deps OR11 (done), CB03 (in), CB05 (in). TGT2 holds the interrupt rule: a player order is not interrupted; idle retaliates; hold fires back only inside weapon range |
| OR31 | done:tests/browser/feedback-oracle.js | judgeOrd6 | Command rate limit | MC ORD6; REG2 runs it again |
| GM01 | done:tests/browser/gm40-oracle.js | assertNoDiscOverlap | Destination spreading | MC GM40 (:162); REG2 runs it again |
| GM04 | done:crates/turts_server/tests/mc_group.rs | gma_second_group_passes_through_a_resting_group_and_both_rest_disjoint | Local avoidance | MC GMA (:258); REG2 runs it again |
| GM06 | done:crates/turts_server/tests/mc_group.rs | gm40_muster_forty_troopers_rest_disjoint_and_near_the_point | Unit collision size | MC GM40 headless (:164); REG2 runs it again |
| GM07 | later:M4b | - | Push idle friends | as MC |
| GM09 | done:tests/browser/gm40-oracle.js | assertRestWindow | Arrival | MC GM40 (:132); REG2 runs it again |
| GM10 | done:crates/turts_server/tests/mc_group.rs | gmu_post_target_ends_on_a_ring_round_the_post | Unreachable target | MC GMU (:491); REG2 runs it again |
| GM11 | later:M4b | - | Choke traffic | as MC |
| CA01 | done:tests/browser/cam1-oracle.js | 04-zoom-limit-a | Classic RTS camera | MC CAM1: pan, zoom limits (:35-38); REG2 runs it again |
| CA02 | done:tests/browser/cam1-oracle.js | 03-right-edge-band | Edge scroll | MC CAM1 (:34, :48); REG2 runs it again |
| CA03 | done:tests/browser/cam1-oracle.js | 02-arrow-right | Key scroll | MC CAM1 (:33, :44-47); REG2 runs it again |
| CA08 | done:tests/browser/mm1-oracle.js | afterLeftScreen | Minimap click | MC MM1: the camera centres on the clicked minimap point (:88-91); REG2 runs it again |
| CA09 | done:tests/browser/mm1-oracle.js | assertOrderFromInput | Minimap order | MC MM1 (:273); REG2 runs it again |
| CA13 | done:crates/turts_interaction/tests/camera.rs | bounds_hold | Bounds clamp | was CAM1; cam1-oracle.js has no look-at bound assert (rigour 8). bounds_hold (:322) asserts the stop at the map margin; red when clamp_to_map (camera.rs:358) returns early (qa-lead run); REG2 runs it again |
| CA14 | done:tests/browser/cam1-oracle.js | 08-space-centred | Centre on selection | MC CAM1 (:39); REG2 runs it again |
| CA15 | later:M4 | - | Height-aware camera | as MC |
| CA16 | done:tests/browser/sh1-settings-oracle.js | scroll-distance-ratio | Scroll speed setting | was SH1.json; the assert is sh1-settings-oracle.js:79 (ratio within 0.2), called from shell-oracle.js:8; REG2 runs it again |
| UI01 | in | KEY1 | Command card | MC HUD2. MP1: unique card cells and keys |
| UI02 | in | MCX1 | Selection panel | MC HUD2. MP1: value predicate, no 'x1 - HP' |
| UI03 | in | HUD6 | Info panel and portrait | MC HUD2. MP1: armour, load and activity |
| UI04 | in | HUD5 | Resource bar | MC HUD2. MP1: credits and power |
| UI05 | done:tests/browser/mm1-oracle.js | assertSlotDots | Minimap | MC MM1 (:307); REG2 runs it again |
| UI08 | done:tests/browser/mm1-oracle.js | mm1-window2-ping.png | Minimap alert flash | MC MM1 (:32, :43); REG2 runs it again |
| UI09 | in | HUD6 | Build menu | MC HUD2. MP1: build menu of the command centre |
| UI10 | in | HUD5 | Production queue display | MC HUD2. MP1: two queues, badges, row states |
| UI11 | in | HUD6 | Tooltips | MC HUD2. MP1: cost, power change and key in the tooltip |
| UI12 | done:tests/browser/vis-full-oracle.js | assertHealthBar | Health bars | was VIS1.json; the health bar assert is in row VIS1-full (vis-full-oracle.js:71-85); REG2 runs it again |
| UI14 | later:M5 | - | Fog rendering | as MC |
| UI16 | in | ALR2 | Event alerts | MC ALR1. MP1: new alerts; one alert for a state that persists |
| UI17 | later:M9 | - | Chat | as MC |
| UI19 | in | SUR1 | Surrender and leave | D-D: surrender is a sim input. Headless: MAT1 |
| UI22 | in | MCX1 | In-game options menu | MC SH1. MP1: Controls groups, keyboard navigation, Unbound |
| UI23 | in | SMT1 | Cursor set | MC HUD4. MP1: new cursor contexts |
| CO01 | in | GHO1 | Placement ghost | MC later:MP1 |
| CO02 | later:M4 | - | Footprint validity | MC later:MP1. Needs CO03. MP1 part (flat ground): GHO1, CON2 |
| CO03 | later:M4 | - | Placement rules | MC later:MP1. Needs MP01 (map format, M4). MP1 part (flat ground): CON1, CON2 |
| CO04 | in | CON1 | Construction model | D-E. MC later:MP1 |
| CO07 | in | CON1 | Under construction state | D-E. A site is an entity with health; attack on a site: CON3 |
| CO08 | in | CON3 | Cancel and refund | D-A, P3. MC later:MP1 |
| CO20 | in | DES1 | Destruction clears footprint | MC later:MP1 |
| CO21 | later:M4b | - | Placement vs units | MC later:MP1. Needs GM07 (push, M4b) and CO03. MP1 part: CON1 refuses a site over a unit |
| PR01 | done:crates/turts_pack/tests/economy_production.rs | head_starts_at_zero_then_advances_and_blocks_fifo | Production queue | M2 (:305); REG2 runs it again |
| PR02 | in | PAY2 | Cancel and refund | D-A: refund of what the job charged. MC done |
| PR03 | in | PAY1 | Payment policy | D-A (P4 b): charge over time. MC done |
| PR04 | in | PRD1 | Parallel production | D-E, Q1: the producer queue and the constructor queue run at the same time. MC later:MP1 |
| PR05 | in | PRD1 | Spawn and exit | MC later:MP1 |
| PR06 | in | RLY1 | Rally applied | P3. MC later:MP1 |
| PR07 | done:crates/turts_pack/tests/forge_cases.rs | c17_ownership_and_policy_rejections_are_exact | Prerequisites | M3 c17 (:1156, PrerequisitesMissing); REG2 runs it again |
| PR08 | done:crates/turts_pack/tests/forge_cases.rs | c15_research_completion_tick_shot_uses_new_damage | Research and upgrades | M3 c15 (:1057); REG2 runs it again |
| PR10 | in | HUD6 | Unavailable reason | MC HUD2. MP1: queue full and no-credit causes |
| PR15 | done:crates/turts_pack/tests/forge_cases.rs | c15_research_completion_tick_shot_uses_new_damage | Modifier application | M3 c15 (:1057); REG2 runs it again |
| PR17 | later:MB | - | Asymmetric factions | as MC |
| EC01 | in | PWR1 | Resource types | MC done. MP1: a Supply resource kind for power |
| EC02 | in | OBS1 | Gather cycle | P9 a, D-F: free approach point at the receiver; the harvester gathers from tick 0 |
| EC03 | done:crates/turts_pack/tests/economy_gather.rs | nearest_receiver_wins_before_id_tie_break | Drop-off choice | M2 test (:471); seen red under mutation (qa-lead, 2026-11-11); REG2 runs it again |
| EC04 | done:tests/browser/orders-oracle.js | gather-depleted | Node depletion | MC ORD3 (:273-277, remaining 0); REG2 runs it again |
| EC05 | done:crates/turts_pack/tests/economy_gather.rs | depleted_original_reselects_nearest_node_after_delivery | Auto-resume and seek | M2 test (:656); seen red under mutation (qa-lead, 2026-11-11); REG2 runs it again |
| EC10 | in | PAY1 | Insufficient funds | D-A: the job waits for funds and resumes. MC done |
| CB01 | done:crates/turts_pack/tests/forge_cases.rs | c14_overkill_destroys_once_and_retargets | Health and death | M3 c14 (:1005); REG2 runs it again |
| CB02 | done:crates/turts_pack/tests/forge_cases.rs | c16_target_at_twelve_stays_out_of_range_and_keeps_ammo | Weapons | M3 c16 (:1107); ammo: c13 (:981); REG2 runs it again |
| CB03 | in | TGT1 | Auto-target | D-G, D-J: acquire range, leash, acquisition within K ticks. MC ORD4 |
| CB04 | later:MP2 | - | Target priority | MC later:MP1. MP1 rule (D-G, D-J): ordered target, then the attacker (TGT2), then the nearest enemy (TGT1); no threat priority |
| CB05 | in | TGT2 | Retaliation | D-G. Was later:MP2 with a weak-cover list that does not exist (rigour 4). TGT2 is retaliation |
| CB07 | in | DMG1 | Damage and armour types | MC later:MP1 |
| CB09 | later:MB | - | Splash damage | as MC |
| CB10 | later:MB | - | Projectiles | as MC |
| CB21 | in | PWR1 | Defensive buildings | MC done. MP1: a turret is off in a power shortage |
| CB25 | later:MB | - | Bonus damage | as MC |
| FV01 | later:M5 | - | Black shroud | as MC |
| FV02 | later:M5 | - | Fog of war | as MC |
| FV03 | later:M5 | - | Sight radius | as MC |
| FV04 | later:M5 | - | Server-side visibility | as MC |
| FV05 | later:M5 | - | Shared vision | as MC |
| FV10 | later:M5 | - | Last-seen structures | as MC |
| UN01 | later:M4 | - | Ground units and movement classes | as MC |
| UN07 | later:MB | - | Abilities | as MC |
| UN11 | later:MB | - | Workers | D-E: constructor is a capability; no unit gathers and builds in 16.2 (S:798-800) |
| UN13 | in | DES1 | Death and removal | X1. MC done |
| PF01 | done:tests/browser/gm40-oracle.js | assertNoBlocked | Sensible path | MC GM40 (:195); REG2 runs it again |
| PF02 | later:M4 | - | Repath on change | as MC |
| PF03 | later:M4 | - | Movement layers | as MC |
| PF04 | in | OBS1 | No stuck units | P9 a: 0 of 20 tanks Blocked in a compact base |
| PF05 | done:tests/browser/feedback-oracle.js | judgeOrd7 | Instant start | MC ORD7: first movement in the tick that drains the order (:178); REG2 runs it again |
| PF10 | later:M7 | - | Mass pathing | as MC |
| MF01 | later:M8 | - | Main menu | as MC |
| MF02 | later:M9 | - | Lobby | Needs MD12 (later:M9). MP1 part (D-B, P2 a: ready and colour, team = slot): LOB1, LOB2 |
| MF03 | later:M9 | - | Map choice | as MC |
| MF04 | later:M9 | - | Game settings | as MC |
| MF05 | later:MP2 | - | AI in slots | as MC |
| MF06 | later:M4 | - | Start conditions | Needs MP01 (M4). MP1 part: fixed starts of Twin Fields and the start order of a placement (D-F, STO1) |
| MF07 | later:M4 | - | Load and countdown | Needs MP07 (later:M4) by R4. MP1 part: LOB1 countdown and one start tick for both clients (D-B, D-C). Y1 of section 2 brings it back to in |
| MF08 | in | MAT1 | Victory and defeat | D-D. MC later:MP1 |
| MF10 | later:M5 | - | Team victory | MC later:MP1. Needs TM01 (later:M5) |
| MF12 | in | RES1 | Score screen | D-K: the result names the reason; no rematch. Units, buildings, credits, time; no graphs, no APM |
| MF14 | in | MAT1 | Disconnect handling | P8 b, D-D: a victory clause of the mod, counted in ticks; no clause, no effect (D28) |
| SV01 | later:M10 | - | Single-player save and load | as MC |
| SV04 | later:M10 | - | Save version safety | as MC |
| SV05 | later:M10 | - | Mod save data | as MC |
| MP01 | later:M4 | - | Map format | as MC |
| MP02 | later:MB | - | Map editor | as MC |
| MP05 | later:M9 | - | Map metadata | as MC |
| MP07 | later:M4 | - | Map distribution | Was done; its dependency MP01 is later:M4, so the design rule of this section makes it later (rigour 8). The package download with its maps exists since M1 (acceptance.rs t2a_package_parity). A CEO decision can remove the edge MP07-MP01 as K20 did for MP09 (section 2, Y1) |
| MP09 | done:crates/turts_server/tests/mc_group.rs | gmu_clamps_finite_outside_target_to_the_inset_point | Playable bounds | MC GMU (:321); REG2 runs it again |
| MD01 | in | VAL1 | Unit definition | D-I: every 0.2.4 unit field has a version-table row |
| MD02 | in | DMG1 | Weapon and projectile definition | MC later:MB. MP1: damage type, acquire, effect ids; projectile travel stays MB (CB10) |
| MD03 | in | CON1 | Building definition | MC later:MP1. Power: PWR1 |
| MD04 | later:MB | - | Ability definition | as MC |
| MD05 | done:crates/turts_pack/tests/forge_cases.rs | c15_research_completion_tick_shot_uses_new_damage | Upgrade and tech definition | M3 c15: a technology definition applies its modifier; REG2 runs it again |
| MD06 | later:MB | - | Faction definition | as MC |
| MD07 | in | DMG1 | Damage and armour tables | MC later:MP1 |
| MD08 | in | KEY1 | UI definitions | MC HUD3. MP1: entry hotkey and card fields |
| MD09 | done:tests/browser/feedback-oracle.js | judgeAud1 | Audio definitions | MC AUD1: package sounds play by category; REG2 runs it again |
| MD10 | in | MAT1 | Victory definitions | D-D. MC later:MP1 |
| MD11 | later:MP2 | - | AI definitions | as MC |
| MD12 | later:M9 | - | Lobby settings schema | as MC. MP1 lobby settings are empty (D-B); section 7.6 must not claim a mod settings schema |
| MD17 | in | MDL1 | Asset pipeline | P1 b, stage 2. Static models and textures; animation stays M13 |
| MD18 | done:crates/turts_pack/tests/bad_packages.rs | a_field_typo_is_refused | Validation messages | M3 and MC bad packages (58 tests); REG2 runs it again |
| BT01 | later:MP2 | - | Skirmish AI | as MC |
| BT02 | later:M12 | - | Difficulty levels | as MC |
| BT03 | later:MP2 | - | Same command path | as MC |
| TM01 | later:M5 | - | Teams | Needs MF02 (later:M9). S:727 says M5 needs teams from MP1; MP1 gives two teams of one and the Ally and Enemy relations (X10), not team choice (P2 a). The M5 spec must bring MF02 or R4 makes M5 red |
| TM02 | later:M5 | - | Alliance effects | Needs TM01. MP1 part: the Enemy relation in TGT1 and TGT2 |
| TM03 | in | LOB2 | Player colours | P2 a: ready and colour choice. MC VIS1 |
| AU01 | done:tests/browser/feedback-oracle.js | muster_voice_order | Unit responses | MC ORD7 and AUD1: the order voice in the release frame (:159-164); REG2 runs it again |
| AU02 | in | ALR2 | Alerts | MC ALR1. MP1: alert sounds once for each state |
| AU03 | in | AUD2 | Positional effects | P5 minimal set; X14 engine part |
| AU06 | done:tests/browser/feedback-oracle.js | assertSinkGain | Volume categories | MC AUD1: categories effects and voice (:119, :159); REG2 runs it again |
| AU07 | in | AUD2 | Sound limiting | P5. MC later:MP1 |
| AN01 | later:M13 | - | Unit animations | as MC |
| AN02 | later:MB | - | Weapon effects | MC later:MP1. Needs CB10 (projectiles, MB). MP1 part: FX1 |
| AN03 | in | GHO1 | Construction visuals | MC later:MP1 |
| AN05 | in | FX1 | Death visuals | MC later:MP1 |
| AN06 | done:tests/browser/feedback-oracle.js | judgeOrd7 | Decals for orders | was VIS1; the move marker is asserted in ORD7 (:157); REG2 runs it again |
| AN07 | in | MDL1 | Team colour on models | P1 b, stage 2: team material on models |
| AN08 | done:crates/turts_client/tests/interp.rs | presentation_time_is_delayed_one_tick_and_clamped | Tick interpolation | M1 (:7); REG2 runs it again |
| AN11 | later:M13 | - | Instancing and level of detail | as MC |
| AC05 | done:tests/browser/shell-oracle.js | judgeCam2 | Cursor confine | MC CAM2: clauses pointer-lock, summed-motion-cursor; REG2 runs it again |
| AC06 | later:M13 | - | Graphics settings | as MC |
| PE01 | done:tests/browser/feedback-oracle.js | judgeOrd7 | Order responsiveness | MC ORD7: marker and voice in the release frame (:157-160); REG2 runs it again |
| PE02 | later:M7 | - | Stable frame rate | as MC |
| PE03 | later:M7 | - | Unit count | as MC |
| PE04 | later:M7 | - | Large selection speed | Was done:SEL2; SEL2 selects 500 units but asserts no time (rigour 8). A hitch bound is a numeric budget (S 15, Open); M7 is the budget check (S:731). MP1 records the box-select frame time in VIS3 (recorded, not judged) |
| PE07 | done:tests/browser/feedback-oracle.js | judgeOrd7 | Latency hiding | MC ORD7: marker and sound precede the OrderResult (:171); REG2 runs it again |

## Player path

This is the oracle list of PT3. The testers never see it: they get the goals texts of section 13. The source is UX
part 1, corrected by PT1 person run 1 (2026-11-11: no goal, no first action, an alert sound that repeats) and by
gate 1. It assumes decisions P2 (a), P5, P8 (b), D-A to D-H and D-K, D-L. Setup: two browser windows A and B, or two
machines; mouse and keyboard only. The brackets name the rows of section 12 that check the step.

Lines L1 to L5 are the lobby. They come first, but their time depends on the second person, so they have no clock.
The 60 s rule of 7.1 starts when a tester presses "Click to play": a person with no knowledge of the game must place
the Power Station site from the screen alone before 60.0 s.

- L1. A: the load screen shows each phase and a progress bar (MC SH1).
- L2. A: the lobby shows the goal line "Destroy the enemy Command Centre", the rows "Slot 1: You — Not ready" and
  "Slot 2: Open — waiting for a player", and the status line "Waiting for a second player." (LOB1).
- L3. B joins. Both windows show the row "Player 2 — Not ready" for slot 2, and the status "Press Ready to start." Each
  slot has a different default colour (LOB1, LOB2).
- L4. A picks the colour of B: the swatch shows a cross, and its tooltip says "Player 2 has this colour". A then picks
  a free colour, and both windows show it (LOB2).
- L5. Both press Ready. The rows show "Ready", then "Match starts in 3", "2", "1". A change by a player clears the
  ready state of that player and shows "Countdown stopped: Player 2 is not ready." At the end of the countdown both
  windows show "Click to play" (LOB1).

The hint list (mod data of `frontier`; the rules are in 7.8). One line shows at a time.

| Id | Kind | Text on the screen | Starts when | Done when |
| --- | --- | --- | --- | --- |
| H1 | step | "Build a Power Station: press P, then left-click inside the ring around your Command Centre." | the first client update after "Click to play" | an own Power Station is pending (not refused), a site, or complete |
| H2 | step | "While it builds, train a Rifleman: press I." | H1 is done | an own Rifleman job is in the producer queue, or an own Rifleman exists |
| H3 | step | "Select your Command Centre, then right-click the ground: new units walk there." | H2 is done | the own Command Centre has a rally point |
| H4 | step | "Train a second Ore Hauler for more credits: press O." | H3 is done | an own Ore Hauler job is in the queue, or 2 own Ore Haulers exist |
| H5 | step | "Build a Guard Turret near your ore field: press T." | H4 is done, and an own Power Station is complete | an own Guard Turret is pending (not refused), a site, or complete |
| H6 | step | "Drag a box round your units, press A, then left-click the ground: they attack on the way." | H5 is done, and 4 own Riflemen, Launchers or Tanks exist | the server accepts an own attack order on a point |
| H7 | step | "Find the enemy Command Centre and destroy it. Look for it on the minimap." | H6 is done | it has shown for 30 s |
| F | fallback | "Train more units while you wait: Rifleman (I), Launcher (V), Tank (K)." | a step is not done, and no step can start | a step can start |
| X1 | situation | "Your Ore Hauler is gone. Select your Command Centre and press O. Cancel other unit jobs first (right-click them), so it starts at once." | no own Ore Hauler and no Ore Hauler job | an Ore Hauler job is in the queue, or an Ore Hauler exists |
| X2 | situation | "Low power: build another Power Station (P)." | H1 is done, the base uses more power than it makes, and no own Power Station is pending or a site | the base uses no more power than it makes, or a Power Station is pending or a site |
| X3 | situation | "Jobs are on hold: you need credits. Wait for your Ore Haulers, or cancel a job (right-click it)." | a job has been "On hold: no credits" for 10 s | no job is on hold |

The steps:

1. In the first frame after "Click to play": the camera shows the own Command Centre, and it is selected. The top line
   says "Destroy the enemy Command Centre". The HUD shows "Credits 1200" and "Low power: 40 used / 0 made", because the
   Command Centre uses power and nothing makes it. No alert line shows and nothing sounds. The Ore Hauler drives to the
   ore by itself (GOAL1, STO1, HUD5, ALR2).
2. The hint line shows H1. The tester presses P or clicks the entry. The ghost follows the pointer, and the ring of the
   build area shows round the Command Centre. The tester left-clicks inside the ring where the ghost ring is solid
   (GOAL1, GHO1).
3. The reward shows at once: the pending site shows in the same frame, and the hint changes to H2 in the same frame.
   Then the site shows a progress bar that grows for 24 s (the tooltip said "24 s now: low power"), and the credits
   count down. At about 30 s the Ore Hauler unloads: "+300" rises at the Command Centre with the delivery sound, and
   the credits go up by 300. When the station is complete, the alert "Power Station complete." shows with its sound,
   the power text shows "Power 40 used / 100 made", and "Low power" goes (GOAL1, GHO1, ALR2, PWR1, HUD6).
4. The tester presses I (H2): the Rifleman job runs while the station builds. Then H3: the tester right-clicks the
   ground with the Command Centre selected. A flag shows with a line from the Command Centre; no unit moves. The new
   Rifleman walks to the flag, and the alert "Rifleman ready." shows (SMT1, RLY1, ALR2, PRD1).
5. The tester hovers the Guard Turret entry. The tooltip shows the name, "Strong against tanks and infantry. Needs
   power. Cannot move.", "Cost 600 credits, paid while it builds", the build time, "Uses 30 power" and "[T]" (HUD6).
6. The tester presses T. Over the Ore Hauler the ghost shows stripes and "Something is in the way"; close to the
   Command Centre, "Too close to a building", with the two gap rings in vermilion; close to an ore node, "Too close to
   ore", with its clearance ring; outside the ring, "Too far from your base". A right click ends placement: the ghost
   goes, and the credits do not change (GHO1).
7. The tester presses O (H4). The tester selects the Ore Hauler: the info panel shows "Harvesting" and the load goes
   up. A right click on another ore node shows the gather cursor, then the gather marker (SMT1, HUD6).
8. The tester places a Guard Turret near the ore field (H5). With the Command Centre selected, the tester presses I
   three times. The entry shows the badge "3". The first row of the producer queue shows its progress and the seconds
   left. The site of the Guard Turret progresses at the same time (HUD5, PRD1).
9. The tester right-clicks the active job in the queue. The job goes, "+N" rises at the Command Centre, and the
   credits go up by N, the amount that the job paid. A cancel of a waiting job gives back nothing, because it paid
   nothing (PAY2, HUD5).
10. The tester builds turrets until the base uses more power than it makes. The power text shows "Low power: 130 used
    / 100 made" with a mark. The alert "Low power: ..." shows once, with one sound. The queue rows say "Slowed: low
    power". Each turret shows an offline mark. Hint X2 shows. A second Power Station clears all of these, and "Power
    restored." shows with no sound (PWR1, HUD5, ALR2, GOAL1).
11. The credits reach 0 during a job. The row says "On hold: no credits", with a striped bar, and the alert "Not enough
    credits: ..." shows once, with one sound. The job crawls on the free income and runs at full speed after the next
    delivery. When the state comes back after a delivery, nothing sounds again inside 90 s (PAY1, HUD5, ALR2, INC1).
12. B drags a box round its units, presses A and left-clicks near the base of A (H6). The attack cursor shows, then a
    cross marker at once. The units move, and they fire by themselves at enemies in range (TGT1, MC ORD4).
13. In the fight: muzzle flashes, tracers, full and reduced hits, and health bars on damaged units. A weapon sound near
    the camera is louder than a far one. A gets "Base under attack. Click to see it." with a minimap ping; a click on
    the line moves the camera to the base. When B's launchers hit A's Ore Hauler, A gets "Ore Hauler under attack.
    Click to see it." (FX1, AUD2, ALR2).
14. An enemy turret turns to its target and fires (FX1).
15. Units die with a death effect, and they leave the selection and their groups. If the Ore Hauler of A dies, A gets
    "Ore Hauler destroyed. Train a new one at the Command Centre (O)." and hint X1. A partly refused order shows "3 of 8
    units: Cannot reach that place" with one error sound; a click on the line selects the failed units (FX1, MCX1,
    ALR2, GOAL1).
16. The Engineer of A right-clicks a damaged own building. The repair cursor and the repair effect show, the health
    bar rises, and the credits go down. At 0 credits the repair stops (SMT1, REP1).
17. A Command Centre is destroyed. Within 1 s, both windows show "Victory" or "Defeat". After 3 s, the result screen
    shows the reason ("Victory. You destroyed the enemy Command Centre." or "Defeat. Your Command Centre was
    destroyed."), the match time, both slots, the outcomes and the statistics. "Leave" opens the shell screen "The
    match is over. This server starts a new match. Reload this page to join it. If the page shows an error, wait a few
    seconds and reload again." (MAT1, RES1).
18. In a second match (instruction of section 13): A opens the menu and selects "Surrender". The text "Surrender and
    end the match? You will be defeated." shows. Cancel changes nothing. Confirm defeats A; window A shows "Defeat. You
    surrendered." and window B shows "Victory. Player 1 surrendered." (SUR1).
19. In a third match (instruction of section 13; decision D-H): B clicks the reload button of the browser in the same
    tab. When the server session of B ends, A shows the alert "Player 2 lost connection." and the banner "Waiting 1:59
    for Player 2 to return.", which counts down in match time. B reloads within 2:00, rejoins its slot, and A shows
    "Player 2 is back."; the match goes on. Later B closes its tab and does not come back: after 2:00, B is defeated,
    and A shows "Victory. Player 2 left the match and did not return within 2:00." (MAT1, ALR2, MC SLOT1).
