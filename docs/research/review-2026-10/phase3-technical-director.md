VERDICT=pass-with-changes FINDINGS=13 HIGH=3 RUNS=6 NOT_CHECKED=3

# Phase 3 review: technical-director on `docs/research/review-2026-10/00-gap-register.md`

Mode: review-spec. R = register line. S = spec v0.7 line.

## Findings

### Blocking (High)

1. **R:101 closes an Open item in a "text fix".** "fix S:352 (Lightyear is selected and pinned)". S:352 says
   "First prototype recommendation, not selected". The networking library is Open: D18 S:41 "Networking library,
   transport ... remain open", S:247 "No transport or crate has been selected", S:704. The code pins lightyear and
   the M1 spike kept it, but the register status stays Open until someone decides it. Fix: move this item to section
   3 as a new G13 ("record lightyear and WebSocket as selected for the prototype line, or keep Open"). Do not list it
   as a text fix.

2. **R:54 (G1) conflicts with Decided items in silence.** "A mod must be able to reproduce the player-facing
   mechanics of every reference RTS". Two reference games hold mechanics that Decided items exclude: Tiberian Sun
   subterranean units (D10 S:33, 3.2 S:106) and Red Alert 2 bridges with ships under them (3.2 S:113: "outside this
   baseline"; D09 S:32). D06 S:29 (Baseline, "not arbitrary game genres") also bounds the WC3 custom modes. Fix: the
   G1 option text names the exceptions ("within D09, D10 and 3.2"), or G1 states that adopting the sentence as
   written reopens D10. The CEO must see the conflict before choosing.

3. **R:41 (F8) states as fact what the seat marked NOT TRACED.** "Chrome does NOT reserve Ctrl+1..9 (yardstick
   corrected)". The web-platform seat read the Chromium source (`gap-web-platform-engineer.md:59`), but its own
   NOT TRACED list says "Real OS key input for Ctrl+1..9 in Chrome (source says not reserved; CDP probe bypasses
   accelerators)" (:70). Firefox and Safari are NOT TRACED (:42, :69). The MC control-group key design will build on
   this sentence. Fix: "Chromium source does not list Ctrl+1..9 as reserved; real-key test and Firefox/Safari NOT
   TRACED; MC starts with that probe."

### Non-blocking

4. **R:52-65: G6 and G7 belong to the team.** Group-order semantics (G6) refines a Proposed sentence (S:342) and is
   an engine mechanism. The order envelope (G7) refines D15 (Baseline) and S:358 (Baseline). Baseline refinements are
   team calls per S:14. Fix: record them as D65 and D73 decisions by the team, and keep them in section 3 only for the
   CEO to see (they cost a protocol bump).
5. **R:61 (G8) cites S:189 but not S:759.** S:759 (Decided): "Libraries provide the decision machinery". A Rust
   bot framework inside the engine moves the machinery out of libraries. Fix: add "touches D48/S:759" to the G8 row.
6. **R:62 (G9) lacks the client boundary.** A command-log replay must be re-simulated. `turts_client` never depends on
   `turts_sim` (M1 design :94; CLAUDE.md crate table). Fix: G9 option text says the re-sim runs on a host and
   streams as a spectator feed, or says that the option changes the T1 rule. The lean "add the determinism
   requirement" touches the Decided paragraph S:243. State that, and say it is a same-binary server rule only.
7. **R:55 (G2) "server-in-process":** in the browser, the client cannot hold the sim (same T1 rule; `turts_server`
   is native only). Fix: the option names the place (a separate worker module) and the T1 rule change.
8. **R:98-99 and R:103-104: D65-D73 are listed by name only.** The status and seat of each row are missing, even
   though R:99 says "Each with a status and a seat". The cross-links are missing too: D65 = G6, D67 = G5, D71 = G4,
   D72 relates to G2, D73 = G7, and D65-D67 need G11. The client architecture decision has no ID; it must be a row
   (D74, Baseline). It must also carry its test: a T1 forbidden-edge rule (no renderer in the interaction core).
   Fix: copy the table from `gap-technical-director.md` (lens answer 1) into section 5.
9. **R:73-76: sequence risk on group movement.** B-7 (units do not stack) is CORE (`inventory-player.md` item 7). MP
   (row 2) has a player-failable acceptance, but M4b (row 4) brings destination spreading later. The register records
   40 units stacking at one point (F4). Fix: move destination spreading (GM01) into MC or MP. Full avoidance can stay
   in M4b.
10. **R:74 vs R:81: duplicate scope.** "per-slot settings" and results/victory screen sit in MP and again in M9.
   Fix: name one home.
11. **R:75 and R:73 hold design, not direction.** "chunk/nav state keyed by map id, group id carried into the search
   queue" and "(world probe says keep picking by footprint)" are implementation choices. Fix: reduce each to the
   defect it fixes.
12. **R:120 vs R:34.** R:120 says "hotkey (1)". My run on S gives 0 hits for hotkey (also `grep-terms.txt`). The
   single "rally" hit is a substring match, not a rally point. Fix: correct the count.
13. **R:23-24: the claim about milestones is too strong.** It says teams and match flow have "no milestone at all".
   But M9 and M10 (S:684-685) cover rooms, start policies and slots. Fix: say "no milestone for the client side of
   match flow (outcome, defeat, score)".

## Answers

1. The register states F1-F7, F9 and F10 correctly against my evidence (F4 matches `command.rs:19,274-282`; F2
   matches my bevy_audio run). F8 overstates its evidence (finding 3).
2. Missing from the CEO list: G13 networking selection (finding 1); G1's conflict with D10 and 3.2 (finding 2);
   the token budget for the correction (115-225 runs, R:84) as part of G10, because the CEO paused M2 for token
   spend. The home for 16.2 was a CEO question in the M2 approval; say that G10 answers it. The team, not the
   CEO, should decide G6 and G7 (finding 4).
3. The sequence breaks no Decided item. It has one CORE dependency risk (finding 9) and one duplicate (finding 10).
   MC orders before MP, MP teams before M5 fog, and identity before reconnect all hold.
4. D65-D73 are present by name, but they have no status, no seat and no cross-link to G (finding 8).
5. Design leaks at R:73 and R:75 (finding 11), and in G9's lean (finding 6).

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `sed -n 352p;358p;790p` on S | R:101 says S:352 records Lightyear as selected; R:36 cites S:358 for mod orders | S:352 says "selected"; S:358 unrelated | S:352 "not selected"; S:358 is the mod protocol ("must not require recompiling the engine client"), so the cite holds |
| `rtk proxy grep -c -i` rally, hotkey, hold position, click, health bar, control group, stop on S | R:34 "0 hits each"; R:120 "hotkey (1)" | non-zero for R:34 terms; hotkey 0 refutes R:120 | hotkey 0, click 0, stop 0, others 0; rally 1 (S:366, not a rally point) |
| `rtk proxy grep` Ctrl/reserved in `gap-web-platform-engineer.md` | R:41 Chrome does not reserve Ctrl+1..9 as a run fact | seat marks it NOT TRACED | :70 NOT TRACED for real key input; :42, :69 Firefox/Safari NOT TRACED |
| `rtk proxy grep` determinism/replay in perf, sim-core, network seat files | R:62 G9 options complete | seat names the host/client place of re-sim | perf :56-61 gives options but no place for re-sim |
| Read R:1-128 | all register claims reviewed | - | 128 lines read |
| Re-read my `gap-technical-director.md` lens table | R:98-99 carries D65-D73 statuses and seats | statuses present in R | names only |

## Not checked

- The seat probe numbers (latency model, 780 overlaps, 620x replay ratio): other seats own them. The register marks
  them NOT TRACED (R:122).
- F5, F6, F7 code lines: I did not re-open the code (the brief said "do not re-explore").
- RA2 bridge and TS subterranean facts are genre knowledge, NOT TRACED against a primary source.

## What I changed

none (repository). Scratchpad file only.
