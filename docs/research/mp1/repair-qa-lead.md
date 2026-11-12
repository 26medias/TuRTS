VERDICT=REPAIRED
## Replace: `## 1. Goal and exit`
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
  green; and the `player` agent plays the loop from the goals text of section 13 to the result screen.
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
---
## Replace: `## 2. Decisions for the CEO before gate 1 closes`
## 2. Decisions for the CEO

Each line quotes the specification line that it touches and the status of that line. "Choice" is the
recommendation of the team or the call of the orchestrator after gate 1; the CEO can overrule each one. The
design below is written on the choice. "Changes" names, for each other option, every section, row of section
12, Yardstick coverage item and Player path step (numbered as in revision 3) that changes [rigour 9]. A line
marked "needs decision" has no choice yet.

| Id | Question | Specification line and status | Options | Choice | Changes for each other option |
| --- | --- | --- | --- | --- | --- |
| P1 | Real models in MP1? | S:247 (Baseline): "glTF is the preferred interchange candidate; exact supported features" stay to be selected; S:750 (Open): importer features | (a) simple shapes; (b) one static model and one texture set for each unit and building, from the package, with the shape as fallback; (c) wait for M13 | (b), with a cap of 9 models, in stage 2; stage 1 uses shapes (RE part 1) | (a) or (c): sections 4 (X16), 8.1, 8.5, 11 (minus 8 to 16 runs); rows MDL1 out, VIS3 primitives only; coverage MD17 and AN07 to `later:M13`; Player path: no step |
| D-M | Model caps (RE:24; mod M9) | S:247 (Baseline): the supported features stay to be selected | (a) the caps of section 8.1 now: 8,000 triangles for one model, 4 MiB for one model, 16 MiB for all models, at most 2 materials; (b) the X16 task sets the caps after the measurement of section 8.5 | needs decision (CEO; section 8.1 names it D-M). qa-lead recommendation: (a) as an upper bound that the 8.5 measurement can only lower, because the MDL1 red packages need fixed numbers | (b): MDL1 gets its cap cases in stage 2; section 8.1 numbers become provisional. No coverage item; no Player path step |
| P2 | Lobby | S:723 (Proposed until gate 1): "a minimal two-slot lobby (team, colour, ready)"; S:335 (Decided): readiness is a suggestion only | (a) two slots, ready and colour choice, team = slot; (b) two slots, ready only, team = slot; (c) team and colour choice | (a). Revision 3 wrote option (a) as "ready only", but its recommendation kept colour choice (LOB2, L4, TM03); the option text now says what the design builds, so every "P2 (a)" in the design keeps its meaning [rigour 9]. Stage 2 | (b): row LOB2 out; Player path L3 (default colours stay), L4 out; coverage TM03 to `done:` with the MC VIS1 slot colours. (c): sections 6.6, 6.7, 7.6; rows LOB1, MAT1 team cases; coverage TM01, TM02, MF10 stay `later` by R4 (MF02 needs MD12, `later:M9`) |
| P3 | Scope cuts if the schedule slips | S:798 (Decided, 16.2 rules): "Engineer: Repairs buildings and vehicles"; S:723 (Proposed): repair, rally, charge-over-time payment, cancel | (a) keep all; (b) cut rally; (c) also cut cancel of construction; (d) also cut repair: this changes the Decided S:798 and needs the CEO [rigour 2]. Charge over time is not a cut: it is decision P4 | (a). If the schedule slips: (b), then (c). (d) only by a CEO decision | (b): sections 4 (X9), 6.5; rows RLY1 out, SMT1 rally part out; coverage OR15, PR06 to `later:MP2`; Player path step 8 out. (c): X4b; row CON3 cancel part out (the destroyed-site part stays); coverage CO08 to `later:MP2`. (d): X8; rows REP1 out, SMT1 repair part out; Player path step 16 out; the specification line S:798 changes |
| P4 | Payment | S:801 (Decided, 16.2 rules): "Construction: Pay credits, select a valid site, then construction progresses over time"; S:434 (Baseline): "consume over time" is a valid policy | (a) pay at the order; (b) pay over time while the job runs | (b) (D-A, orchestrator). This changes the Decided order of S:801, so the CEO decides, and the specification line changes to "Select a valid site; construction progresses over time and charges credits as it progresses" [rigour 2] | (a): sections 4 (X7), 6.4, 7.3; rows PAY1, PAY2 (refund base becomes the paid cost), CON1 (charge clause), CON3, HUD5 ("On hold: no credits"), HUD6 ("paid while it builds"), ALR2 (no-credits state); coverage PR03, EC10, PR02, CO08 notes; Player path steps 3, 4, 9, 11 |
| D-A | Charge model of P4 (b) | S:434 (Baseline): "Cancellation, refunds, interrupted production ... must be explicit" | (a) each tick charges the difference of `floor(cost x progress)`, with a u128 intermediate; ledger events coalesced: one event when the job completes, is cancelled or stalls; (b) charge in progress steps of a mod-data size (TD:199-200); (c) as (a) with one ledger event for each tick | (a) (orchestrator) [perf M3] | (b): rows PAY1 (totals at step bounds), PAY2, HUD5 (the credits fall in steps); Player path step 3. (c): row PAY1 event-count case out; the ledger grows to about 400 events for a 20 s tank (perf M3). No coverage item |
| P5 | Sound | D69 (Proposed): audio | none; a minimal set (fire, hit, death, built, low power, ready, error); voices | the minimal set, in stage 2, with the engine part of X14 (an effect names a sound; the engine plays it at the place, with a cap) | none: sections 4 (X14 sound part, X17), 8.4; row AUD2 out; coverage AU03, AU07 to `later:M13`; Player path steps 3, 13, 15 lose their sound clauses. Voices: X17 grows; no row change |
| P6 | Names, tone, art direction | S:807 (16.2, Decided section): "Names/art ... remain to be specified during implementation" | as revision 3 | Faction "Lantern Company" (working name), a mirror match over a claim; grounded near future, no gore; low-poly, clear top-down silhouettes, team colour on large surfaces; plain function names for units | Another choice: section 5 names; the words of Player path steps 2, 6, 7 and of section 13. No row; no coverage item |
| P7 | The two people of the play test | S:703 (Decided, G12): a person signs off | two people; or the CEO and one more person | two people, one of them not the CEO, so one tester is blind | The CEO as a tester: section 13 and row PT3 (one tester is not blind); exit item 2 |
| P8 | A lost connection | D28 (Decided): "Do not impose ... disconnect gameplay through the engine"; S:327 (Baseline): everyone disconnects is server policy | (a) the match waits for ever; (b) a slot that stays disconnected for N seconds is defeated, by a victory clause that the mod declares; with no clause, nothing happens [mod L11] | (b), N = 120 s in the `frontier` victory data, counted in ticks (D-D), in stage 2 | (a): sections 6.7, 4 (X11); row MAT1 disconnect cases out; coverage MF14 to `later:M10`; Player path step 19 out |
| P9 | Units and buildings in the way | none (MC ruling O14 records the limit) | (a) a free approach point in the engine and a placement gap as mod data; (b) the M4 route step in MP1 | (a), in stage 1 | (b): sections 6.9, 11 (400 to 700 lines, 5 to 7 files, estimate of the probe); row OBS1 becomes a route row; coverage PF04 stays `in`. No Player path step |
| D-B | Claim boundary | D25 (Decided): "A claimed player slot ... cannot be reassigned"; S:308 (Baseline): "the exact claim boundary" needs design; S:350 (Baseline): open to claimed only by atomic admission | (1) a lobby seat is a reservation that the room can release; the match start claims every seated slot in one step, and D25 holds from that tick; a package with no start policy starts at once (`Immediate`), and the claim stays at Hello, as today (`crates/turts_server/src/session.rs:152`); (2) the claim stays at Hello, and a leave closes the slot | (1) (orchestrator) [rigour 3, backend H2, net 1] | (2): sections 6.7, 7.6; rows LOB1 (the leave closes the room), LOB2; coverage MF07 note; Player path L2 to L5 (a leave ends the lobby) |
| D-C | One match for each process | S:296 (recommendation): "Separate worker processes per active match are a recommendation" | (a) the server runs Boot, Lobby, Countdown, Running, Ended, then exits with 0; a supervisor starts the next process; (b) the process resets the world in place (about 30 resources, `crates/turts_sim/src/plugin.rs:60-90`) | (a) (orchestrator); no world reset in place [backend H1, net 12] | (b): sections 4 (X11b), 6.7; a new reset row; rows MAT1, RES1; Player path steps 17, 18, 19 (a next match with no reload) |
| D-D | Disconnect, reconnect, surrender | S:267 and S:392 (Decided, G9): replays are a command log; D28 (Decided) | (a) the room writes each as a sim input into the command log at a tick; the sim counts the 120 s in ticks; the rule is a clause of the mod victory data; (b) a server timer on the wall clock | (a) (orchestrator). (b) breaks invariant 15 (S:581), so it is not a valid option [net 2, backend H4] | (b): row MAT1 loses its replay case. No coverage item; no Player path step |
| D-E | Where construction progress lives | S:801 (Decided); S:434 (Baseline) | (a) the structures queue of the constructor holds the order until the player places it; then the site entity owns the progress, the charge and the job; `constructor` is a work capability beside `producer`; (b) the progress stays in the producer queue, and the site is a marker | (a) (orchestrator) [rigour 1, mod H2] | (b): sections 6.1, 7.3; rows CON1 (its red becomes the green), PRD1; coverage OR16, CO04, CO07 notes; Player path step 7 |
| D-E2 | A cap on open sites for each slot | none (net 5: a site at progress 0 is free) | (a) a cap as mod data; (b) no cap; (c) an engine constant | (a) (orchestrator). The `frontier` value is content (GD:141 proposes 1; S:807 keeps numbers open) | (b): row CON1 cap case out; the free-site grief of net 5 stays. (c): a private engine number. No coverage item; no Player path step |
| D-F | The harvester at the start | S:805 (Decided): "One command center, one harvester, and enough credits to establish power" | (a) a map placement can carry a start order (an engine mechanism, 6.10 row M3); the `frontier` harvester gathers from tick 0; the first hint names the power station; (b) no mechanism; the first hint is "Send your Ore Hauler to the ore" | (a) (orchestrator) [player H1] | (b): section 4 (X24); row STO1 out; GOAL1 first hint; coverage MF06 note; Player path steps 2 and 6 change order |
| D-G | A unit that is attacked | S:803 (Decided): "Move, attack, attack-move, stop, hold, and automatic targeting" | (a) an idle unit targets its attacker inside its leash; a holding unit fires back only inside its weapon range and never moves; (b) a holding unit chases inside its leash (revision 3) | (a) (orchestrator) [rigour 5] | (b): rows TGT1 (hold never moves) and TGT2 contradict; coverage OR08, OR26 notes; Player path step 12 |
| D-H | Where the browser keeps the player id | D52 (Decided): one controlling session for each claimed slot | (a) sessionStorage, as today (`crates/turts_client_web/src/lib.rs:239-251`); step 19 says "B reloads the page in the same tab"; (b) localStorage; a second tab becomes a D52 session replacement | (a) (orchestrator) [net 3] | (b): MC row SLOT1 (clause copied-tab-terminal-refusal) changes; Player path step 19 can say "closes its window". No coverage item |
| D-I | The public mod API 0.2.4 | D15 (Baseline): a versioned mod API | (a) a full table in the design: for each item the export group or file, field, type, default when absent, validation, error text and version-table row; presence means "written in the source", read from the raw file; (b) lane 0 defines the fields | (a) (orchestrator) [mod H1, mod M4] | (b): row VAL1 has no list of red packages before lane 0. Coverage MD01 note |
| D-J | Performance rules | S 13 (Baseline); budgets Open (S 15) | (a) the UnitId index and the debug-state gate in lane 0; acquisition from a grid for each relation, only when the weapon is ready and has no valid target, at most every K ticks, staggered by unit id; wire records with a start tick and a rate, sent on change; slice part A3 before content with a footprint above the mobile radius; (b) leave them to later milestones | (a) (orchestrator) [perf H1 to H4] | (b): sections 4 (lane 0), 10, 11 (minus 4 to 7 runs); rows TGT1 (query counter), DBG1, DES1 (index), OBS1 (A3); coverage CB03 note |
| D-K | A rematch | S:296: "A room may exist ... afterward for results or rematches" | (a) no rematch button in MP1; the result screen names the reason of the result; "Leave" says how to start the next match; (b) a rematch button (the room outlives the match, against D-C) | (a) (orchestrator) [player 9] | (b): sections 4 (X13), 6.7; rows RES1, LOB1; coverage MF12 note; Player path step 17 |
| D-L | A free income of the Command Centre | S:799 (Decided, 16.2 rules): "Harvester collects ore and delivers it to the command center for credits"; S:807: these policies are mod-owned | (a) an engine capability `income` as mod data; `frontier` gives 3 credits each second while the Command Centre is complete (section 5, row INC1); (b) no income; the floor comes only from a cheaper hauler | (a) (part-3 repair; the call is NOT TRACED in the orchestrator's brief). qa-lead: it adds a second credit source beside the Decided harvesting line; it does not contradict that line, but the CEO should see it [player 5] | (b): section 5 table, row INC1 out; Player path steps that count credits (3, 6, 11) lose the income; the 166 s floor of section 5 goes. No coverage item |
| D-N | The VIS3 floor before the play test | S 15 (Open): numeric budgets | (a) p95 at or below 50 ms and p50 at or below 33 ms at 400 units on the play-test machine, as a start condition of PT3 (Proposed, not a budget); (b) no floor | needs decision (section 8.5 names it D-N) [perf L2] | (b): row VIS3 and PT3 lose the start condition. No coverage item; no Player path step |
| Q1 | Named producer queues | none (6.1; GD:272 asked for `lanes`) | (a) no named producer queues in 0.2.4: the command centre has the producer queue and the constructor queue (D-E); (b) named queues for each producer | (a) (part-2 repair, 6.1) | (b): sections 6.1, 6.10, 7.3; rows PRD1, VAL1; coverage PR04 note |
| S1 | Specification text edits after MP1 | S:723 and S:809 ("three" M3 rules; MC lifted one); S:801 (D-A); S:727 ("Needs teams (MP1)") | edit the lines; or keep them | edit: S:723 and S:809 say "two"; S:801 as in P4; S:727 says what MP1 gives (two teams of one, the `Ally` and `Enemy` relations) | No row; no coverage item; no Player path step |
| 6.7.14 | Four server policy points of section 6.7 | see 6.7.14 | see 6.7.14 (the `all_disconnected` default; `AllSeated`; when Ended exits; the disconnect notice timeout) | the recommendations of 6.7.14 | the changes that 6.7.14 names |
| N1 | The bare-id rule of the no-content scan | none (support code: `crates/turts_checks/tests/support/mod.rs:449-456`) | (a) match the qualified id and the bare local id for every kind, as today; a collision with an engine word names the definition path; the repository package renames, or the scan gets an exception row with a reason and `identifiers_only: true`; (b) match only qualified ids for the new 0.2.4 kinds | (a) (qa-lead): a leak in engine code is mostly a bare word in a string literal (for example `"power"`), and (b) cannot see it. An exception with `identifiers_only: true` keeps the literal red [mod H3] | (b): row NC1 bare-id plants go green. No coverage item; no Player path step |
| Y1 | The yardstick edge MP07 to MP01 | QA0 K6 (CEO): only the CEO changes a dependency edge (`docs/research/review-2026-10/inventory-engine.md:5-9`) | (a) keep the edge: MP07 and, by R4, MF07 are `later:M4`; (b) remove the edge, as K20 did for MP09: the client downloads the locked package with its maps since M1 | needs decision; the design applies (a); recommendation (b) [rigour 8] | (b): coverage MP07 to `done:crates/turts_server/tests/acceptance.rs` (test `t2a_package_parity`), MF07 to `in` LOB1; the inventory gets a K line. No row; no Player path step |
---
## Replace: `## 4. Scope`
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
| X4 | Construction (D-E): `constructor` is a work capability beside `producer`; a `Construct` order with content and a point; the structures queue holds the order until placement; a site check; a site entity that owns its progress, charge and job and can be attacked; the limits `concurrent_sites` for each constructor and `open_sites_per_slot` for each slot as mod data; no named producer queues (Q1); a footprint that blocks | E | 1 |
| X4b | Cancel of construction with refund of what the site charged | E | 2 |
| X5 | Placement mode and ghost | UI + E (interaction core) | 1 |
| X6 | Power: supply and demand for each slot, a shortage rule for each capability | E + C + UI | 1 |
| X7 | Charge over time (D-A): each tick charges the difference of `floor(cost x progress)` with a u128 intermediate; the stage "waiting for funds"; coalesced ledger events | E | 1 |
| X8 | Repair: a `repairer` capability and a `Repair` behaviour, charged over time | E + C | 2 |
| X9 | Rally: `SetRally` on a producer; the engine sends the follow order through `apply_order` with the source `Engine`: no seq, no token, no `OrderResult` (net 8) | E + UI | 2 |
| X10 | Relations: `Ally` and `Enemy` beside the present slot filter; MP1 uses two teams of one | E | 1 |
| X11 | Match lifecycle: a declarative victory rule as mod data and an outcome for each slot | E + C | 1 |
| X11c | Surrender, disconnect and reconnect as sim inputs in the command log; the disconnect rule as a clause of the mod victory data, counted in ticks (D-D, P8) | E + C | 2 |
| X11b | One match for each server process (D-C): Boot, Running, Ended, exit 0 (stage 1); Lobby and Countdown (stage 2, with X12); a client that connects after the end: the owner gets the result, a new id gets "The match is over" (LEAVE1) | E | 1, 2 |
| X12 | Lobby for two slots (D-B): lobby seats are reservations; the match start claims every seated slot in one step; the lobby states Lobby and Countdown; ready and colour (P2 a); the browser keeps the player id in sessionStorage from the lobby seat (D-H) | UI + E (server room) | 2 |
| X13 | Result screen, owned by the mod, from engine sources; it names the reason of the result; no rematch; "Leave" says how to start the next match (D-K) | UI | 1 |
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
| X21 | The play test: the `player` agent on the stage-1 loop (stage 1); two people (stage 2) | I | 1, 2 |
| X22 | Final regression (REG2) and CI; REG2 also runs at the end of stage 1 | I | 1, 2 |
| X23 | Architecture audit before the merge to main | I | 2 |

The state alerts of ALR2 are in stage 1, though the stage order does not name them: the CEO play of MC failed
on an alert that repeats (`mods/forge/interface/hud.ron:130-136`), and stage 1 has the low-power and
no-credits states.

Not in MP1: a bot (MP2), fog of war (M5), terrain (M4), routes round obstacles beyond section 6.9,
skeletal animation (M13), more than two slots, rooms (M9), scripting (MS), a rematch (D-K), a world reset in
place (D-C), team choice (P2), a worker that walks to its site (D-E).
---
## Replace: `## 11. Budget`
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
---
## Replace: `## 12. Acceptance rows`
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
ready. The rows PAY1, PAY2, CON3, DES1 and REP1 need
`turts_audit`, which slice part P1 adds (it is not in today's tree).

Stage 1 rows: NC1, VAL1, IMM1, DMG1, TGT1, TGT2, CON1, CON2, PWR1, PAY1, INC1, DES1, OBS1, STO1, DBG1, ORDW1,
MAT1 (victory, defeat and draw), RES1, LEAVE1 (the exit after the end and the restart), GOAL1, GHO1, PRD1,
HUD5, KEY1, ALR2, FX1 (the wire event, the flash and the death effect), REG2. Stage 2 rows: CON3, PAY2, REP1,
RLY1, LOB1, LOB2, SUR1, MAT1 (surrender and disconnect), LEAVE1 (the lobby seat of the next match), HUD6,
SMT1, AUD2, MDL1, VIS3, MCX1, NOGPU1, FX1 (the rest), PT3 (people), and REG2 again.

| Row | Check | Red build | Runs |
| --- | --- | --- | --- |
| NC1 | The no-content scan (`crates/turts_checks/tests/no_content.rs`, `real_source_scan_is_clean`, :166) derives a token for every content kind of API 0.2.4: the kinds of today's list (`crates/turts_checks/tests/support/mod.rs:86-99` and the loops to :267) and each new kind of the 0.2.4 table (D-I), for example damage types, armour classes, capacity resources, queue ids, victory clauses, start policies, hint lines, effects and models, and the interface kinds (alerts and widgets) that today's list does not hold. For each kind the scan keeps the qualified id and the bare local id (`add_id`, support/mod.rs:449-456; decision N1). A bare id that equals an engine word is red and names the package, the definition path and the field; the repository package renames the id, or the scan gets an exception row (the table at support/mod.rs:272-292) with a reason and `identifiers_only: true`, so that a string literal of the word stays red. The modder guide states this rule; a mod outside `mods/` is never scanned [mod H3]. No 0.2.4 engine field name equals a `frontier` local id (for example a field `power` while `frontier` has a resource `power`) | For each new kind, one planted string literal in a scanned crate (for example `"frontier:cannon"` as a damage type and `"structures"` as a queue id, in `crates/turts_net_protocol/src/lib.rs`): a derivation that misses the kind gives 0 hits for its plant, and the row is red on that plant. The gap is real today (qa-lead run on d729a97; the code of 1b15402 is the same): the literal `"muster_unit_damaged"` (an alert id, `mods/muster/interface/hud.ron:19`) gives 0 hits and exit 0, while `"muster:trooper"` gives 48 hits and exit 101 | headless |
| VAL1 | The two M3 rules are lifted (`crates/turts_mod_runtime/src/content_m3_validation.rs:374-376` and `:420-424`; 6.10 rows K7, K8), and the work-capability exemption of `constructor` beside `producer` (`:418`, D-E) has its row. Each row of the 0.2.4 table (6.10, D-I) has its red packages, by its detection kind: for a RAW row, a `^0.2.3` package that writes the field, and a second one that writes it with an empty value, an explicit `None` (for example `armour: None`) or an explicit `[]` (for example `orders: []`) [mod M4]; for a VAR row, a `^0.2.3` package with the variant; for a COMBO row, a `^0.2.3` package with the combination; for an ED row, a `^0.2.3` and `^0.2.4` pair that shows the two behaviours. Each refusal has the form `` `<row name>` needs engine api 0.2.4; `^0.2.3` admits 0.2.3 `` (`crates/turts_mod_runtime/src/version_table.rs:101-108`). A reserved variant (`NeedsBuilder`, `counts: Queued`, `BlockStart`) is refused with "reserved for a later engine API". A package with a constructor and no `open_sites_per_slot` is refused (6.10 row K6). An unknown reason key or cursor key is refused with a `did you mean` hint [mod M8] | (1) A table row missing: its red package packs with exit 0. (2) Presence read from the parsed value, the form of today (`orders_present: !orders.is_empty()`, `crates/turts_mod_runtime/src/content_mc_validation.rs:65`): the explicit-empty packages pack with exit 0; this red exists today (`^0.2.2` with `orders: []` packs with exit 0, mod-builder probe at gate 1). (3) A reserved variant accepted. (4) Key check off: `queue_ful` and `plcae` pack with exit 0 (today's behaviour, mod-builder probe) | headless |
| DMG1 | Damage is `hit = max(minimum, floor(base x num / den) saturating_sub flat)` with a u128 intermediate, `minimum` on the damage table, `flat` on the unit armour, and the cell (damage type, armour class) of the table (6.3); no float takes part. A test package, not `frontier`, holds two cells where the integer form and a float form differ: damage 100 with 29/100 gives 29, and damage 50 with 106/100 gives 53; and a 2 x 2 table whose transpose differs: (bullet, light) 100/100, (bullet, heavy) 10/100, (rocket, light) 25/100, (rocket, heavy) 150/100; each shot asserts its exact damage. A package with no damage table keeps the damage of today (`hit = base`; REG2 and the M3 fault set show it). Each balance ratio of section 5 is a headless duel with the `frontier` numbers; a duel that fails is a content defect and a retune (S:807 keeps the numbers Open), not a red of the engine; the duel results go to `docs/measurements/mp1.md` [rigour 6] | (1) A float multiplier (`damage as f64 * (num as f64 / den as f64)`): the cell 100 x 29/100 gives 28; with f32 the cell 50 x 106/100 gives 52. The qa-lead probe found 146 differing cells among 80,000 (damage 1 to 400, ratio n/100 for n 1 to 200) and no cell that differs for both f64 and f32, so the row needs both cells. (2) The table read with its axes swapped: the shot (bullet, heavy) gives 25, not 10. (3) `flat` taken before the ratio: the case with flat 5 on 100 x 29/100 gives 27, not 24 | headless |
| TGT1 | An idle unit with `acquire` (6.3; on the unit, 6.10 row U4) and a ready weapon acquires the nearest enemy (distance, then unit id) inside `acquire.range` within K = `ACQUIRE_PERIOD_TICKS` (4 at the MP1 tick rate) ticks after the enemy enters the range (D-J); it chases while its distance from its idle point is at most `leash`, then goes back to the idle point. An ordered target comes first. A holding unit never moves: its position bits are equal on every tick of the scenario; it fires only at an enemy inside its weapon range. A query runs only when the weapon is ready and has no valid target, and a unit queries only in a tick t with `(t + unit_id) mod K = 0`: a query counter (the `cfg(test)` counter pattern of `crates/turts_sim/src/movement.rs:13-21`) shows 0 queries for a unit that keeps a valid target, and no unit queries twice in K consecutive ticks [perf H3]. Candidates come only from the grids of `Enemy` teams: a `turts_rules` unit test with a teams map of two slots for each team shows that an ally inside the range is never a candidate. A unit without `acquire` keeps the behaviour of today (REG2) | (1) Hold handled as idle: the holding unit chases inside its leash, and the position check fails at the first tick that it moves (`unit U on hold moved at tick T`). (2) A query in each tick, or before the readiness check as today (`choose_target` before the cooldown test, `crates/turts_sim/src/economy/combat.rs:201`, `:229`): the counter shows a query in each tick. (3) The leash ignored: the unit follows past the leash. (4) A grid built with the relation `Other`: the ally of the unit test becomes a candidate | headless |
| CON1 | The structures queue of the constructor holds each `Construct` order, never progress or charge (D-E, 6.1). A placement is refused for each cause with its reason key of 6.1, one case each, and the first failing check gives the cause: outside the map, out of the build area, an overlap with any footprint (a unit included), a gap below `placement.gap` to a static footprint, inside `placement.node_clearance` of a node, the slot at `open_sites_per_slot`, and a full queue [player H4, net 5]. A valid placement creates a site entity in the tick of the order. From that tick the site owns the progress, the charge (PAY1) and the job, and the constructor counts only its open sites. With `concurrent_sites` 2, two sites of one constructor progress in the same ticks; a third order waits in the queue, and the engine places its site (an engine order, 6.5) in the tick after one of the two ends. The site completes, and the building has full health [rigour 1, mod H2] | (1) Progress advances in the constructor's queue after placement (the site is only a marker): after the placement tick the queue holds a job with progress, and the second site stays at progress 0 while the first progresses (`site S2 progress 0 at tick T`). (2) For each cause, the check of that cause off: its case is accepted. (3) The check order changed: the case with two failing causes gives the second cause. (4) The site check off: every case is accepted | headless |
| CON2 | For a scene with no motion and no new site between the client frame and the server tick, the client verdict from its replica equals the server verdict for 1,000 seeded points [net 9]. The seeds put at least 20 points in each band where a wrong rule changes the verdict: a gap from 1.8 to 2.0 from a footprint, over a unit, inside the node clearance, and at the edge of the build area. The test counts the points in each band and fails when a band has fewer than 20, so that a new seed cannot make the row unable to go red. The client gets its verdict from `turts_rules` (section 9); `scripts/boundary.sh` refuses a placement function in `turts_client` other than the call | (1) The client uses its own copy of the rule with the gap 1.8: the band points differ (`point P: client valid, server refused`). (2) The client leaves out the unit blocker class: the points over a unit differ. (3) A copy of the placement function in `turts_client`: the boundary check is red | headless |
| CON3 | `CancelConstruction` on a site refunds `floor(charged x num / den)` with `cancel_refunds.running` (6.4). The case cancels at progress 37/100 of a cost of 500 with the ratio 3/4: the charge is 185 and the refund is 138. A cancel of a held order refunds 0. A destroyed site refunds by `cancel_refunds.destroyed`, which is 0/1 when absent: one case with it absent (refund 0) and one with 1/2 (refund 92). After the cancel and after the destruction, the claims are released in the same tick: a new site on the point passes the site check, and the open-site count goes down by 1 (a placement at the cap is then accepted). The ledger (`turts_audit`) balances after each case | (1) Refund of the full cost: 375 or 500, not 138. (2) The running ratio used for a destruction: 138 for the destroyed site, not 0 or 92. (3) The open-site count kept: the next placement is refused with the cap reason | headless |
| PWR1 | Supply (`Supply(scope: Slot, counts: Completed)`, 6.2) counts units that exist and are not sites: `provides` and `consumes`, once for each tick at the start of the economy phase. In a shortage each capability applies its own `when_short`: `Rate(1/2)` on the producer and the constructor, `Disable` on the turret weapon (it neither fires nor acquires); a capability with no entry (the rifleman weapon, repair) is not affected. Recovery restores both at the first computation after the supply returns. A weapon that the player disabled stays disabled after recovery (a shortage is not `SetEnabled`). The rule reads only mod data: a second test package names its supply resource `zeta:steam` and puts `Rate(1/3)` on a different capability, and the same checks pass | (1) A check keyed on the `frontier` resource id or on the word power: the `zeta:steam` package shows no slowdown (`rate 1/1 under shortage`); NC1 is also red on the literal. (2) Sites counted: a station that is still a site adds supply. (3) The shortage implemented with `SetEnabled`: the weapon that the player disabled is enabled after recovery. (4) `Disable` that stops fire but not acquisition: the disabled turret records a target | headless |
| PAY1 | Charge over time (P4, D-A, 6.4): a job has `done` and `total = duration_ticks x Q`; the total charged at `done` is `floor(cost x done / total)` for each cost entry, with a u128 intermediate; each tick charges the difference. At completion the total equals the cost; the balance never goes below 0. A tick that cannot pay every difference charges nothing and does not advance (stage `WaitingForFunds(resource)`, the first resource in cost order that cannot pay), and the job resumes in the tick after the next delivery. The ledger has one `Charged` event for each job end (completion, cancel, entry into `WaitingForFunds`) with the sum since the last event, not one for each tick. Cases: a cost that the duration does not divide (500 over 240 ticks); a stall at 0 credits; two cost entries where only the second cannot pay; a product `cost x done` above the u64 range (cost 2^40, `total` 2^25) [perf M3] | (1) Charge at the order: the balance falls by the cost in the order tick. (2) A fixed amount `cost / duration` in each tick: the total at completion is 480, not 500. (3) A u64 intermediate: the large case overflows (an error or a wrong total). (4) One charge for each cost entry on its own: the first entry is charged while the job does not advance. (5) One ledger event for each tick: 240 events for the job, not 1 | headless |
| REP1 | Repair (6.5): the heal of each tick is `floor(rate x t / tick_hz)` minus the same value at `t - 1`, at most `max - health`; the total charged after H applied points is `floor(H x amount / per_health)`, and each tick charges the difference; a tick that cannot pay heals nothing (stage `WaitingForFunds`); a cost of 0 heals with no charge; nothing is charged at full health. Health changes only inside a transaction: (a) static: the only production writers of the health table in `turts_sim` are `crates/turts_sim/src/boot_m3.rs:110` and `crates/turts_sim/src/economy/transaction_m3.rs:218` (a `turts_checks` test with the syn scanner and its production filter); (b) at run time: an audit rule says that for each unit and tick the health change equals the `Fired` damage plus the repair heal of that tick | (1) A repair system that writes `world.resource_mut::<Healths>()` directly: the static check names the file and line (qa-lead probe: a plant in `economy/combat.rs` gives `RED health writer economy/combat.rs:83`), and the audit rule finds a health change with no event. (2) Repair at 0 credits: health rises while the balance is 0. (3) A charge at full health, or a charge for the requested heal and not the applied heal | headless |
| RLY1 | A new unit follows the rally through `apply_order` (`crates/turts_sim/src/orders.rs:2610`) with the source `Engine` [net 8]. The follow order is the smart rule of the rally target (`orders/rules.ron` `rally`, else `smart`, 6.5): a rally on ground gives the mod's ground order, a rally on a node gives a hauler its gather order. The order log (`--order-log`) records the order with the source `Engine` and no seq; it costs no token and gives no `OrderResult`; a refusal shows in `last_failure`. Two runs of the same `--command-script` give identical order logs and economy logs (`cmp`), and the economy-log replay validator (`validate_records`, `crates/turts_server/src/economylog.rs:371`) passes | A fake player envelope: the server injects the follow order as a player envelope with a seq of the slot. The order log shows the source `Player`; the client report has an `OrderResult` for a seq that the client never sent; the token count of the slot falls in the spawn tick | headless |
| MAT1 | Outcomes (X11, D-D), package `standoff` (limit 100 ticks at 20 Hz). Scripts, `--start-now --command-script`: (a) attack orders kill B's key unit at tick T: A `Victory`/`OpponentsDefeated`, B `Defeat`/`KeyUnitsLost`, `match_over_tick` T, and the tick log has no tick after T; (b) both key units die in one tick: `Draw` for both; (c) `connection: false` for both slots at tick 10: `Draw`/`Disconnected` at tick 110; at ticks 10 and 20: slot 0 `Defeat` at 110 and slot 1 `Victory` at 110; (d) slot 0 false at 10 and true at 60: no outcome at 110. Two headless clients: (e) B `surrender` at tick 40: both reports have the outcome with the reason `Surrendered` and the same `match_over_tick`, and B's key unit is alive at the end; (f) B killed: the input log has `connection: false` for slot 1 at tick D, at most 11 s after the kill, and B gets `Defeat`/`Disconnected` at D + 100; (g) B killed, then B with `--rejoin` before the limit: `connection: true`, and no outcome by D + 200; (h) the `--input-log` of (e) and of (f), replayed with `--start-now --command-script`, gives the same outcomes and `match_over_tick`; (i) `muster` (no victory data), one client killed: no outcome after 300 ticks; (j) after the end, a new id gets `The match is over`, and B (an owner that did not leave) gets `welcome` and the same `match_result`. A sim test with three slots: an order from a defeated slot gets `slot_defeated` | the client works out the outcome from the unit list ((e) and (f): `outcome` is null, because B's key unit is alive; in case (a) the two agree, so only (e) and (f) can show this red); the disconnect count is a wall-clock timer in the server ((h): the replay has no connections, so it gives no outcome); a defeat with no clause ((i), D28); a draw rule that gives `Victory` to the lower slot index ((b), (c)) | headless, two clients, and scripts |
| LOB1 | Lobby start and claim boundary (D-B), package `standoff`. Run 1, headless clients A, B, C: A seats: A's first `lobby_states` entry has `you: 0`, seat 1 `Open`, start `Waiting`, and A has no `welcome`; server `--stats` `ticks` stays 0 for 3 s. B seats: both reports show two `Human` seats with different colours. Only A is ready for 3 s: start stays `Waiting`, no `welcome`. B ready: start `Countdown`; B's `lobby_move` in the countdown gets `match_not_started`. Both `welcome` records have `tick` 0, `start_tick` 1, and equal `teams` and `colours`; the first snapshot tick is 0 for both; the tick log starts at tick 1. After the start, C (new id) gets `no open slot`; B killed, then B with `--rejoin` gets slot 1 again. Run 2: B sends `unready` in the countdown: start goes back to `Waiting` and no `welcome` comes; B `ready` again: the match starts. Run 3: A and B seated, B killed: within 11 s A's lobby state shows seat 1 `Open` (the time is recorded as L), and C gets seat 1. Pixels, two windows: the words of L2 to L5, "Match starts in 3", "2", "1", and both windows show "Click to play" | the sim claims at `Hello` (run 3: C gets `no open slot`); the sim ticks in the lobby (the `welcome` ticks are above 0 and differ, as today: 76 and 171 in the gate 1 probe); a start with one ready (run 1); an order admitted before the start (run 1: `lobby_move` gets another result); `unready` does not stop the countdown (run 2) | headless, three clients; pixels, two windows |
| RES1 | Result (D-K). Pixels, two windows: within 1 s of the end tick both windows show the outcome and the reason line (7.6 words, for example "Player 2 surrendered"); within 3 s more, the result screen shows both slots, their outcomes and reasons, and the statistics; the oracle compares each number on the screen with the `match_result` of that client. Wire check (net 10), headless, two clients: in each report `stats_first_seen_tick` equals `match_over_tick`, and each client got exactly one `MatchResult`; an export unit test in `turts_server` builds the per-slot export at each tick of a scripted `standoff` match and finds no statistics before the end tick | the sim view gives the statistics at each tick (headless: `stats_first_seen_tick` is 1; the export test is red at tick 1); a result screen without the reason (pixels); a pixels driver alone cannot show the first red, because a client can hold a value and not show it | headless, two clients; pixels, two windows |
| FX1 | A `Fired{unit, target}` event, with the weapon as an index (net 7, perf M1), reaches each client that may see the shooter or the target (both clients in MP1). In window B an enemy turret turns to its target and its muzzle flash shows; a death shows its effect; a reduced hit (table ratio below 1/2, for example a bullet on a tank) shows `hit_reduced` and a full hit shows `hit` (`turts_debug.effects.recent`). With 400 riflemen that fire, the effect pool stays at 256 and the effect entity count does not grow. After a T8 resync, no effect older than the presentation tick minus K plays | (a) today's event path: the event is staged for the shooter's slot only (`crates/turts_sim/src/economy/combat.rs:393-395`), so window B sees no enemy fire; (b) the pool grows past 256, or one entity spawns for each shot; (c) `hit_reduced` never chosen; (d) the kept deltas of a resync replay their effects: a burst after the resync | headless and pixels |
| HUD5 | Forced states in a scenario, each read as exact text: "Credits 1200"; "Power 40 used / 100 made"; under a shortage "Low power: 130 used / 100 made" with the mark; queue rows "On hold: no credits" with a striped bar, "Slowed: low power", "Waiting to start", "Waiting: exit blocked"; a disabled entry "Queue is full (5/5)". A starved job under the free income (it pays in some ticks and not in others) shows "On hold: no credits" in each of 10 frames sampled over 1 s | (a) the `Supply` fields `Used` and `Made` swapped: "Power 100 used / 40 made"; (b) the row state without its 1 s hold: the words are absent in some of the 10 frames; (c) the job stage `WaitingForFunds` mapped to the low-power words | pixels |
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
| STO1 | A map placement with a start order (`order: Gather`, decision D-F) gathers from tick 0 with no client input: the first delivery comes within 1 s of the first-trip time of the model (29.7 s, `pacing.py new`); the command log records the order at tick 0 with the source `Engine`; a replay of the log gives the same delivery tick; the order has no seq, costs no token and gives no `OrderResult` | (a) the field parsed and not used: no delivery in 60 s; (b) the start order sent as a player envelope: it shows in the admission evidence with a seq | headless |
| GHO1 | P, or a click on the build entry, starts placement; the server order log has no new line. The ghost follows the pointer, and the build-area ring of the own Command Centre shows (radius 12.5). Over a valid point: a solid ghost ring in the slot colour. At one fixed point for each cause on a test map: stripes, the `forbidden` cursor, and the exact text of 7.2 at the pointer: "Outside the map", "Too far from your base", "Something is in the way" (over a unit, and over a building), "Too close to a building" (the gap only: the gap rings of the ghost and of the building show in vermilion), "Too close to ore" (the clearance ring shows). Right click and Esc end placement: no new order line, and the credits do not change. Left click places: a pending site shows in the same frame; after the server result a world progress bar grows on the site until it is complete. Shift and left click places and keeps the ghost. Mismatch case (a scenario unit walks onto the point after the client verdict): the pending site goes, "Something is in the way" shows at the site point, one error sound plays, the credits do not change, and the hint does not advance | (a) the gap folded into the overlap check (the rev-3 text): the gap-only point shows "Something is in the way"; (b) the old ore text "Cannot build on ore"; (c) the world style `build_area_ring` or `gap_ring` not drawn: no ring pixels at radius 12.5, or none round the building at the gap-only point; (d) the smart path runs before the placement mode: a right click in placement sends a move (an order line); (e) the client reads a fixed gap of 0.5, not `placement.gap`: the ghost is valid at the gap-only point and the server refuses | pixels |
| PRD1 | The Command Centre runs one site from its structures queue and one job from its units queue at the same time (decision D-E). A second placed order waits ("Waiting to start") while one site is open (open-site cap 1, mod data) and opens when that site completes. A sixth placed order is refused with `queue_full`. A new unit appears at an exit point of the producer. With every exit point blocked, the complete job waits ("Waiting: exit blocked"), charges nothing more, and the unit appears when an exit point is free | (a) the producer advances only its first queue with an active job: the unit job stops while a site progresses; (b) the open-site cap not counted: two sites progress at once; (c) the exit check ignores static footprints: a unit appears inside the footprint of the Command Centre | headless |
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
---
## Replace: `## 14. Risks and the early tests`
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
| The server detects a disconnect late: it refused the same id until +14.5 s and accepted it at +15.8 s (debug build; backend M7) | MAT1 counts the 120 s from the end of the server session. Test: the same probe on a release build |
| The lobby and screens overrun, as the client shell did in MC (22 runs against 6) | Stage 2. A pixels-only stub: two clients reach the lobby, press ready, the match starts; red if the stub needs more than 4 runs |
| Models cost too much frame time | Stage 2: the 400-unit measurement of section 8.5 |
| Art production has no natural end | The cap of decision P1 and the shape fallback |
| The re-freeze and the audit change have a long tail of moved fault plants | The slice gate runs the fault sets with the replay validator disabled |
| `turts_audit` is not in today's tree (no hit in `crates/` or `Cargo.toml`); PAY1, PAY2, CON3, DES1 and REP1 need it | These rows start after slice part P1 is merged |
| The completeness gate checks form only: a `done:` path to any file that exists passes (`done:Cargo.toml` exits 0, qa-lead run) | The proposal R7b (section 15). Until it lands, qa-lead checks each `done:` token with a script at each gate |
---
## Replace: `## 15. Open points for gate 1`
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
7. S:727 says M5 "Needs teams (MP1)"; with P2 (b) MP1 gives two teams of one, not team choice. The
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
---
## Replace: `## Yardstick coverage`
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
| EC01 | in | PWR1 | Resource types | MC done. MP1: a capacity resource kind for power |
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
---
## Not done

- Runs and evidence of this repair: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/qarep/runs.md` (16 runs); the R7b diff: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/qarep/r7-proposal.diff`. The final check: the worktree design (1b15402) with all blocks of this reply applied gives `--enforce` exit 0 and `--self-test` 21/21.
- Base of the section 12 block: it holds the rows of the other seats verbatim as they are on ba4e689 (repair parts
  1 to 3 applied), with my rows replaced and DBG1 added after OBS1. If a row of another seat changed after
  ba4e689, apply that seat's newer row text after this block.
- Conflict between seats, not mine to fix: 6.1 names the reason keys `site_out_of_area`, `site_too_close`,
  `site_near_resource`, `site_cap`; 7.2 names `site_too_far`, `site_near_building`, `site_near_ore` and has no
  cap key. CON1 uses the causes of 6.1; the two lists must agree before lane 0.
- 6.1 says that an order at the caps waits in the queue, and it also lists the refusal `site_cap`. CON1 tests
  the wait for `concurrent_sites` and the refusal for `open_sites_per_slot`; 6.1 must say which applies when.
- My row PLC1 is dropped: row STO1 (ux-designer seat) now holds the start order. Its red (a), "the field parsed
  and not used", shows only absence. Plants that can happen: the order applied at the first `Welcome` (the first
  gather stage at the Welcome tick); the gather state written outside `apply_order` (no order-log line).
- rigour 10 (when a hint is "done", and the end of the 60 s of PT3): the hint list and PT3 belong to the
  ux-designer seat (GOAL1, PT3, Player path).
- rigour 11 (terms): my sections use "player slot", "lobby seat" (D-B), "site" and "queue" in one meaning each;
  the definitions list for the whole design is not written.
- rigour 12: the D70 status in 7.6, the GLB features in 8.1, the invariant-15 citation and the missing
  `economy/` in the paths of 6.3 and 6.4, and 8.4 against section 16 are in sections that I do not own. My rows
  cite the correct paths. The MD12 conflict and the S:727 note are now in the coverage notes and in section 15.
- player H2, H3, 5, 6, 7, 8, 11: hint list, alert repeat, harvester alert, role line, tooltip time, goals text,
  extra match instructions: ux-designer seat (GOAL1, ALR2, HUD6, section 13).
- mod H1 (section 6.10 table), M5, M6, M7, M9: in sections 6 and 8. VAL1 and NC1 depend on that table and are
  written for it.
- net 1, 2, 10 (rows LOB1, MAT1, RES1), net 6, 7, 11 (6.8 and the client): other seats. Section 2 records the
  decisions behind net 1, 2, 3, 5, 12.
- backend M5 to M8, L9, L10 (wire seat, lobby records, hooks): backend seat, except the boot-time number, which
  exit item 5 now records.
- perf M1, M2, M4, M5, L1, L3 (wire, replica, parse, export, effect pool, memory): sections 6.8, 8 and the
  rows FX1 and VIS3. perf L2 (VIS3 floor) is open point 3 of section 15.
- The browser-row plant of REG2 is NOT RUN; only the native plant ran.
- The static instrument of REP1 and DES1 ran as a scratch script that cuts each file at its first
  `#[cfg(test)]` line. A file with an early test attribute hides code below it; the plan must use the syn
  scanner of `turts_checks` with its production filter.
---
## Decisions for the CEO

1. D-A: the payment order of the Decided line S:801. Options: (a) keep S:801 (pay at the order); (b) pay over
   time while the job runs, and change S:801 to "Select a valid site; construction progresses over time and
   charges credits as it progresses". Recommendation: (b), the orchestrator's call.
2. P3 (d): cutting repair changes the Decided line S:798. Options: never cut repair in MP1; or allow the cut
   as the last item if the schedule slips. Recommendation: never cut it; cut rally, then cancel of
   construction.
3. Y1: the yardstick edge MP07 to MP01. Options: (a) keep it, so MP07 and MF07 are `later:M4` (the text applies
   this); (b) remove it under QA0 K6, as K20 did for MP09, so MP07 is `done:` (`t2a_package_parity`) and MF07
   is `in` LOB1. Recommendation: (b), because the client downloads the locked package with its maps since M1.
4. D-M: the model caps of section 8.1 (8,000 triangles and 4 MiB for one model, 16 MiB for all, 2 materials).
   Options: (a) fix them now as an upper bound; (b) the X16 task sets them after the 8.5 measurement.
   Recommendation: (a), because the MDL1 red packages need fixed numbers, and the measurement can only lower them.
5. D-L: a free income of 3 credits each second for the Command Centre (row INC1). Options: (a) the engine
   capability `income` as mod data; (b) no income. It adds a credit source beside the Decided harvesting line
   S:799 and does not contradict it. Recommendation: (a), with the CEO aware of the addition.
6. D-N: the VIS3 floor before the play test (p95 at or below 50 ms, p50 at or below 33 ms). Options: (a) a
   start condition of PT3 (Proposed, not a budget); (b) no floor. Recommendation: (a).
7. For the orchestrator, not the CEO: proposal R7b for `scripts/qa/completeness.py` (section 15, point 9; diff
   in the scratchpad). Recommendation: apply it with the fill of the MC and fixture cells in one commit.
