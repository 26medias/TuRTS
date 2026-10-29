VERDICT=fail ITEMS=25 MISSING=11 PARTIAL=11 CONFLICT=1 RUNS=14 NOT_TRACED=6

Seat: qa-lead. Areas: 13.8, 16.1, section 14 acceptance evidence. Mode: review-spec (gap audit). What I changed in the repository: none.
S = `RTS-Engine-Project-Specifications.md` v0.7. Probe files: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/qa-probe/`.

## Gap table

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-RO01 | Replay recording | partial | S:64 D41 Decided "Replays ... are secondary"; S:237 "replay/debug tooling" (principle); S:710 replay Open | mechanism | none | M4 (session record as test evidence, see qa-lead-4); full replay with M11 | S:364 checkpoint + state stream (D18 forbids input-only replay) | No. `crates/turts_server/src/economylog.rs:3` JSONL sink is additive | Under D18 a replay is a recorded state stream, not a re-simulated input log (S:364). |
| A-RO02 | Replay controls (speed, seek, vision) | missing | "seek", "playback", "replay control": S:171 animation, S:364 viewer catch-up; no replay UI | mechanism | none | after M11 | A-RO01, S:364 checkpoints | No | Seek needs checkpoints; M11 builds them. |
| A-RO05 | Replay version safety | partial | D35 locked hashes; S:64 migration secondary | mechanism | none | with A-RO01 | lockfile hash (built, M1 T2d) | No | Reuse the lockfile hash in the replay header. |
| A-MD15 | Fast iteration (hot reload / quick restart) | missing | "hot reload", "reload", "quick restart": 0 hits | mechanism | none | when an agent need shows (D43), latest before 16.2 | package loader `turts_pack::load_package` | NOT TRACED | 16.1 tool; record the need when it appears. |
| A-MD18 | Validation messages (file, field, cause) | present | S:209, S:229 Baseline; built: M1 T2b/T2c (`docs/superpowers/specs/2026-10-26-m1-foundation-design.md:319-320`) | mechanism | M1 | M1 | none | No | Instrument went red at M1 verify (memory m1-verify). |
| A-MP02 | Map editor | conflict | S:720 Decided "Do not precommit to a large manual editor suite" vs CORE for a moddable engine | mechanism | none | needs decision | MP01 map format | No | See "Conflicts". Text format + validator satisfies D43 first. |
| A-PE01 | Order responsiveness (click to feedback) | partial | S:243 "immediate input acknowledgement" (Baseline item); no milestone measures it | mechanism | none | M4 record, M7 judge | IN01, OR23 | No | Record click-to-ack latency; budget stays Open (S:700). |
| A-PE02 | Stable frame rate | partial | S:650, S:662; thresholds Open | mechanism | M1 records browser frame p99 (`docs/measurements/m1.md`) | M7, M13 | AN11 | No | Recorded, not judged. Correct per status. |
| A-PE03 | Unit count | present | D57 S:80, D58 S:81, S:578-586 | mechanism | none run on the allocation | M7, M12 | GCP VM (D59) | No | `docs/measurements/m1.md:287-288`: no run on 4 vCPU/8 GB yet. |
| A-PE04 | Large selection speed | missing | "select" only library/dependency senses (inventory-player.md:361) | mechanism | none | M7 (1k-5k units), M12 | PC02 | Yes: `crates/turts_client/src/economy_ui.rs:31` holds one id | Not measurable until box-select exists. |
| A-PE05 | Load time | partial | S:284 setup phase; M1 records download size only | mechanism | M1 (size) | M8 | 6.5 acquisition | No | Add time-to-playable to the M8 evidence. |
| A-PE06 | Late-game slowdown, graceful degradation | missing | "degrad": 0 hits | mechanism | none | M12 | 6.9 budgets | No | M12 "bounded work" (S:687) is closest; say what the player sees. |
| A-PE07 | Latency hiding | partial | S:243 Baseline; M1 T5 interpolation | mechanism | M1 | M4 onward with PE01 | AN08 | No | Interpolation exists; order feedback does not. |
| B-71 | Replays and observer mode | partial | as A-RO01; S:360-368 spectators | mechanism | none | M11 | A-RO01 | No | Same gap from the player side. |
| qa-lead-1 | Section 14 has no player-control acceptance claim | missing | Run 1: 0 of 13 rows (S:676-688) name select, input, click, hotkey, mouse, keyboard. Hits are noise: S:676 "selected packaging", S:678 "Inputs/outputs", S:680 "player sees" (secrecy), S:688 "picking" (render check) | both | none | every milestone with a browser client, from M4 | Yardstick A2-A6 | No code; docs only | The Demonstration column also has 0 hits. |
| qa-lead-2 | Browser tests cannot fail on discoverability | missing | `tests/browser/m2.js:151-161` reads the `turts_ui` bridge to find each control, then clicks it; `tests/browser/m3-human.js:27,35,43`; `tests/browser/t7.js:213` moves through `window.turts_move` | mechanism (instrument) | none | M4 | qa-lead-3 | Yes, test side only | m3-human passed (R:13) on the build the CEO could not use (R:3). The instrument was green on the failing build. |
| qa-lead-3 | Scenario runner works below the input layer | partial | `crates/turts_client/src/scenario.rs:1-4` sends `PlayerCommand` envelopes; `crates/turts_client/src/input.rs` has no drag, box, shift, ctrl or KeyCode | mechanism | M2 (protocol traces) | M4 | client input layer | Additive | A build with no input layer passes every headless scenario. Add a pointer/key event script. |
| qa-lead-4 | No session record of a playtest | missing | `--command-trace` is input-only, protocol level; no recorder of a human/agent session found ("--record", "record_trace" only marks trace steps, `scenario.rs:220`) | mechanism | none | M4 | qa-lead-3, A-RO01 | Additive | A failed playtest cannot become a regression test without client input log + server command log + state stream. |
| qa-lead-5 | Automated performance regressions | partial | S:568 "Keep automated performance regressions"; `.github/workflows/ci.yml` step names (lines 24-142) have no perf step; grep "bench|p99|baseline|perf" in ci.yml: 0 | mechanism | none | M4 record into a committed baseline, M7 judge | stable runner or GCP VM | No | Baseline lives nowhere; nobody sees a red. |
| qa-lead-6 | Machine-readable API/package schemas (16.1) | partial | S:722 Proposed; no `schemars`/`JsonSchema`/`--schema` in `crates/` | mechanism | none | when mod-builder or agent need appears (D43) | API 0.2 schema types | No | Proposed only; correct to wait for a need. |
| qa-lead-7 | Blind playtest protocol | missing | "playtest", "usability", "human evaluat": 0 hits in S. M4 F9 (`2026-10-28-m4-terrain-design.md:345-348`) is the first one | both | M4 (draft) | M4 | qa-lead-2, -4 | No | F9 as drafted cannot be shown red; see Lens answer. |
| qa-lead-8 | Gate lens rules exclude the player and the input owner | missing | design `2026-10-25-studio-agent-team-design.md:123` "Not picked for an artefact with no playable loop"; `:228` player only for S 3, 6.6, 16.2, 17; `:234-238` engine lens from Demonstration column; `.claude/agents/player.md:99,103` "write needs-decision and stop" | process | none | now | none | Agent files and CLAUDE.md, not code | Selection is derived from the spec text, so a hole in the spec is a hole in the panel. |
| qa-lead-9 | 13.8 has no player-facing scenario | missing | S:648-658: nine rows, all cost or correctness | mechanism | none | M7, M12, M13 | PE01, PE04 | No | Add "box-select N units and order them" and "input-to-ack latency under load". |
| qa-lead-10 | The one "human-facing" test is not in CI | missing | `m3-human.js` has no hit in `.github/workflows/ci.yml` or `scripts/*.sh`; only `m3.js` (`scripts/m3-browser.sh:82`) | mechanism | none | M4 | none | Yes, CI only | A hand-run test is not a regression guard. |
| qa-lead-11 | D57/D58 benchmarks never run; D59 status | partial | S:82 D59 "access pending"; CLAUDE.md says use the default gcloud config | mechanism | none | M7 | GCP access | No | Whether access exists now: NOT TRACED. Possible status drift. |

## Conflicts with the goal

- S:720 (16.1, Decided) "Do not precommit to a large manual editor suite" against A-MP02 (CORE for a moddable engine). Needs decision. Options: (a) keep 16.1; a text map format plus a validator and a preview tool count as the editor until a need shows (D43); (b) add a map editor milestone. Evidence: no map tool exists; maps are RON files (`mods/`).
- S:64 D41 (Decided) "Replays ... are secondary". Tension with A-RO01 (COMMON), not a block. It lowers priority only.
- S:41 D18 (Decided) is a constraint, not a conflict: replay and session records must be state streams (S:364).
- No Decided item blocks player-control acceptance evidence.

## Lens answer

### What each gate was asked

Gate records: M1 G1/G1b (rigour, mod-builder, technical-director, network, mod-runtime; web-platform at re-gate: `vault/Work Log/2026-10-26.md:28,30`); M1 G2/G2b (qa-lead, rigour, technical-director, producer, network: `:33,35`); M2 G1 (six seats: `docs/research/m2/design-reviews.md:42-49`); M2 G2 (six seats: `docs/research/m2/plan-reviews.md:64-71`); M3 G1 and G2 (qa-lead, technical-director: `vault/Decisions/2026-10-27 M3 scope and orchestrator calls.md:16,29`). Nine seats sat. `player`, `bevy-rendering-engineer` (owner of input and UI) and `game-designer` (owner of the first playable loop) sat at zero gates.

| Lens | Question asked (source) | Compares with |
| --- | --- | --- |
| Rigour | contradictions, undefined terms, untestable claims, status drift (`.claude/agents/spec-rigour-reviewer.md:12`); "Section 14 is the model" (`:67-68`) | the spec itself |
| Engine | correctness of the subsystems in the Demonstration column (design `:234-238`) | the Demonstration column |
| Boundary | order "derived from the demonstrations" (`.claude/agents/producer.md:43`); M2 G2 "against the approved economy-only design" (`plan-reviews.md:8-10`) | the approved design |
| Consumer | mod-builder: can a modder build it from the public docs (design `:122`) | the public API |
| QA (me) | "Turn each row into a scenario that can fail" (`.claude/agents/qa-lead.md:55-63`) | the section 14 claims as given |
| Player | never asked: rule `:123`, `player.md:99` | - |

Every question took the spec's own list as the bar. My question, "can this test go red", tests a claim. It never tests whether the claim list is complete. I passed T7 after the M1 verify, but T7 drives the unit through `window.turts_move` (`t7.js:213`). I recorded that T7 stayed green with no renderer (memory m1-verify). I did not ask what the player can do.

### The question that would have found it

"Name the wrong build: today's client. Which acceptance test goes red because it lacks a yardstick CORE item?" For PC02 box-select, OR24 group orders, control groups and hotkeys the answer was "none" at every gate. A second form: "Write the player path for this demonstration, step by step with mouse and keyboard, and name the test that drives each step through real input." M1 to M3 have no such section (Run 10).

### Proposed completeness gate (instrument; each part can go red)

1. **C1 Coverage matrix** (machine check, `qa-probe/completeness_gate.py`). Every milestone spec carries `## Yardstick coverage`: one row per CORE item (181), disposition `in | later:Mn | mod | excluded:S:line`, and a test id. Rules: R1 every CORE item has one row; R2 an `in` row names a test that is in the acceptance section; R3 a `## Player path` section with numbered steps; R4 an `in` item has its CORE dependencies `in` at the same or an earlier milestone (catches wrong-milestone). Observed: red on all four current specs (M1-M4); green on a fixture; red on each of five mutations (drop PC02; OR24 with no test; no player path; CA01 deferred; PC02 bound to H9). Wrong builds: a spec with a missing row, an untested `in` row, a dependency scheduled later.
2. **C2 Driver/oracle split** (machine check, `qa-probe/hook_ban.py`). A player-path test has two processes: a driver that may use only pointer events, key events and screenshots; an oracle that may read bridges and server logs. Observed: red on `m3-human.js` (4 bridge uses) and `t7.js`; green on a pointer-only fixture. The probe also flags oracle reads; the real check must scan only the driver file. Wrong build: a driver that calls `turts_move` or reads `turts_ui` to find a control.
3. **C3 Blind playtest.** A tester (player agent or a person) gets the build, the in-game guide, and screen pixels. No bridge, no source, no design document. Task list = the Player path. Any "could not find object, action or reason" blocks. Before trust, run it once on a planted build (labels removed, or drag disabled) and record the red. Run it twice for flakiness. Store the session record (qa-lead-4) as the evidence.
4. **C4 Panel rule.** Every milestone with a browser client adds `player` and `bevy-rendering-engineer` lenses. `player.md:99` must file "no playable loop" as a High finding, not stop.

### Player-control acceptance evidence per future milestone (direction)

- M4: PC01, PC02, OR24, IN01 height pick. Put H9 into section 11 (now H1-H9 are not referenced by F1-F10; Run 13). Plants: box selects enemy units; `MoveTo` carries only the first id; pick on plane y=0. F9 through C2 and C3; remove `window.turts_move` from the build F9 plays, or prove it unused (it is global in every build: `crates/turts_client_web/src/lib.rs:78-79`). Record PE01.
- M5: fog states and the scanner reached through the player path; plant: client shows a deposit the server did not send.
- M6: overlay toggle by key; a crossing visible on screen.
- M7: control groups, hotkeys, stop/hold, attack-move (S:747); PE04 box-select at 1,000 and 5,000 units; PE01 under load. Recorded; budgets Open.
- First playable default mod (S:694, 16.2): the 16.2 adaptation of the yardstick B first-ten-minutes script, blind playtest against the bot, session record kept.
- M9 to M11: room create, slot claim and spectate through the UI, not through test flags.
- M12: input under load; M13: picking with animated crowds tied to PC01.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| Python regex over S:676-688 cells | S:674-688 has a player-control acceptance claim | any real control term | 0; 4 noise hits |
| Python over acceptance sections of M1-M4 designs | M1-M3 acceptance names a player path | a hit in M1 (311-332), M3 (635-703) | M1 0, M3 0, M2 1 ("real controls", E9), M4 F9 |
| spec greps: replay, hot reload, degrad, playtest, schema | S covers RO01/RO02/MD15/PE06/playtests | design text | see table rows |
| `grep -c` turts_move vs page.mouse in tests/browser | browser tests drive real input | t7 with mouse calls | t7: 2 turts_move, 0 mouse |
| grep `turts_move` with context in turts_client_web | hook is test-only (cfg gated) | a cfg gate | none; `lib.rs:78-79` |
| grep drag/box/shift/KeyCode in input.rs, economy_ui.rs | client has multi-select | any hit | 0; `Selection(Option<u32>)` |
| read scenario.rs:1-30 | scenarios exercise input | pointer/key events | PlayerCommand only |
| grep perf/m3-human in ci.yml, scripts | perf regression and human test in CI | a step | none |
| grep schemars/JsonSchema in crates | 16.1 schema export exists | a hit | 0 |
| `completeness_gate.py` on M1-M4 specs | C1 goes red on today's specs | exit 0 | exit 1, R1+R3 per spec |
| gate on fixture + 5 mutations (`run_mutations.txt`) | C1 is green on a valid spec and red on each defect | green on a mutation | green 1, red 5/5 |
| `hook_ban.py` on m3-human.js, t7.js, fixture | C2 separates bridge drivers from pointer drivers | m3-human green | red 4, red 4, green 0 |
| grep H1-H9 in M4 section 11 | M4 acceptance includes drag-select proof H9 | an H ref in F1-F10 | 0 |
| grep lens lists in gate records | player/input owner sat at a gate | a hit | none |

## NOT TRACED

1. The verbatim prompts sent to each gate. Only lens lists and the "Minimum prompt" rule (design `:242-243`) are in the repo.
2. Whether `m3-human.js` passes on HEAD 5461486 (not run; R:13 says it passed on 1354e33).
3. Whether the player agent can drive a real pointer in the browser with today's tools (needed for C3).
4. GCP access state versus D59 S:82.
5. Whether the yardstick dependency OR24 -> OR11 (order queue) binds M4's group `MoveTo`.
6. Coverage of the 13.8 rows by milestones other than M4.
