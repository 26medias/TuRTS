VERDICT=fail BLOCKING=3 HIGH=6 RUNS=11

# Gate 1, rigour lens: spec amendment 0.7 to 0.8 and CLAUDE.md

Seat: spec-rigour-reviewer. Mode: review-spec. S = `RTS-Engine-Project-Specifications.md` (working tree, 856 lines).
R = `docs/research/review-2026-10/00-gap-register.md`. C = `CLAUDE.md` (working tree).

## Blocking findings

B1. High. S:94, S:96, S:102, S:187, S:744, S:745, C:174. G5, G6, and G7 have the wrong status, and the document contradicts itself.
- R:73: the CEO adopted the team lean for G2 to G16. That gives G5 = (a), G6 = B, and G7 = yes (R:60-62).
- The amendment records these lines as Proposed (D65 "Proposed (G6, G11)", D67 "Proposed (G5, G11)", D73 "Proposed (G7)").
- Section 15 lists them under "What must be decided" (S:744 "line (a) of G5 (D67, Proposed)"; S:745).
- S:187 says "Line (a) of G5 applies", which is a decided statement.
- C:165 says "Do not assume any of these", and C:174 lists the control model and the order envelope.
- Failure: a gate-1 reviewer of the MC spec can reopen per-unit acceptance or line (a) and cite S:745 and C:165. The CEO decided both.
- Needs decision on the form. The source of the conflict is G16 (R:70), which makes the rows Proposed.
    - Option A: use a mixed status in the D18 pattern, for example "Decided line (G6, option B); design Proposed (G16)". Remove the decided part from Section 15 and from C:174.
    - Option B: keep the rows Proposed, and record that the CEO decided only the row status. Then remove "applies" (S:187) and "option B" (S:94).
- The evidence favours option A: R:73 adopts every lean.

B2. High. S:700, S:705 column header, S:710-S:723. G12 is missing. It did not land anywhere in S or C.
- G12 (R:67) says "both; a person signs off".
- Line S:852 says that G11 and G12 are "recorded in the register". The spec does not use them.
- "Acceptance a player can fail" does not say which player: a person or the `player` agent seat. In S, "player" means a person (S:21). In C, `player` is an agent seat.
- grep "signs off|a person" in S: no hit in Section 14.
- Failure: the MC or MP1 gate can pass on a run by the `player` agent alone. G12 does not allow that.
- Fix: add one sentence to the S:700 intro: "A person runs the player check and signs it off; the `player` agent also runs it (G12)."

B3. High. S:700 against S:718, S:719, S:724, and S:725. The intro makes a new claim that is false, and the instrument cannot go red.
- S:700 says "Each milestone with a client has an acceptance check that a player can fail."
- M7 (S:719) has player input latency, so it has a client. Its check is "p99 numbers recorded", which passes when any number exists.
- M13 (S:725) is a browser client. M6 (S:718) has an underground overlay. M12 (S:724) has a client. Each of the three says "As before (register section 4); set by the Mx spec".
- "As before" has no referent in S, because the old table had no such column (git HEAD S:672-688). In R:102 and R:105, "as before" means no player check.
- "Set by the <x> spec" (S:709-S:723) says when the check is set, but not who sets it.
- Fix:
    - Give M7 a pass/fail player criterion, or remove M7 from the claim.
    - Replace "As before" with "None yet; the Mx spec author (producer) writes it before gate 1".
    - Name the author for every "set by" cell.

## Non-blocking findings

H1. High. S:97, S:98, S:99, S:103. D68, D69, D70, and D74 have no G link. R:127 requires a G link for each row. G16 approves all ten rows. Add "G16" to the status cell of each row. Also, the Second of D70 (`game-designer`, `gap-technical-director.md` table) is dropped and no reason is given.

H2. High. S:709-S:713. The acceptance cells use terms that S does not define: "completeness gate", "pixels-only test driver", "driver/oracle split", "QA0 fixture", "blind playtest", "the two `turts-pack` validation defects", and "arrive as a group". grep in S finds them only in these rows. Their definitions are in R section 5 items 1-3 (R:114-120) and R:47-48, and S does not cite those lines. "Arrive as a group" has no threshold. Fix: cite R section 5 items 1 to 3 in the S:700 intro. Add a measurable criterion to MC, for example "no two units overlap at rest".

H3. High. S:267. The G9 same-build determinism requirement is Decided, but no instrument tests it. Section 12 did not change (the diff has no hunk). No milestone acceptance names it, and M11 (S:723) says only "replays as a command log". The requirement holds now, so the M1-M3 server can already break it, and nothing can show it red. Fix: add an acceptance item, for example "two runs of one build on one command log give the same state hash". Put it in QA0 or M11, or add invariant 15.

H4. Medium. S:26 and S:746 against S:700, S:713, S:718, S:724, S:725, S:794, and S:852. "Register" has two meanings. Section 2 is the "Decision register", and S:746 "No register row yet" uses that meaning. The new rows use "register" for the gap register. S:852 "recorded in the register" can mean either one. Fix: write "gap register (`00-gap-register.md`)".

H5. Medium. S:66 against S:20. D37 (Decided) says "mods control camera, lighting, and presentation". The new definition at S:20 makes "presentation" include audio, overlays, and visual interpolation. That change makes the Decided D37 wider without a note. D37 also keeps "expected" (rigour-10), which is not a status word. Fix: write "presentation settings", as S:121 does, or add a note to D37.

H6. Medium. S:716. The MS acceptance cannot fail. "Browser and native measurements" and "Decision recorded with browser and native measurements" pass on any measurement. Fix: name the measured quantities and the comparison.

L1. Low, STE.
- S:267: "So the server simulation must be deterministic..." has 34 words. Split it after "build".
- S:794 has 32 words.
- S:187 "is refined by" is passive. Rewrite: "D65 to D68 and D74 (Proposed) refine the Player interface row."
- S:70 "Reopened in 0.8 (G2)" sits next to the status "Decided". Rewrite as "Revised in 0.8 (G2)".
- 19 of 623 new sentences have more than 25 words. Most are lists. See `rig/long.txt`.

L2. Low. Two names for one object: "action map" at D74 S:103, and "input map" at D66 S:95 and S:743. "Smart command" (S:94) has no definition.

L3. Low. C:141 still gives only the M1 design as the source of the crate layout. The rows now list M2 and M3 files (C:148, C:153).

## Lens questions, answered

1. G1 to G16:
    - G1: verbatim at S:113. A Python substring test of R:56 against S returned True.
    - G2: S:70 and S:277.
    - G3: S:81.
    - G4: S:72 and S:761.
    - G8: S:77 and S:804.
    - G9: S:267 and S:392.
    - G10: S:700-S:725. The order matches R:93-105.
    - G13: S:47, S:265, S:271, S:376-S:380, and S:741. Cargo.toml:24 has `=0.31.0`.
    - G14: S:742 and S:716.
    - G15: S:711.
    - G16: S:94-S:103.
    - Defects: G5, G6, and G7 have the wrong status (B1). G12 is missing (B2). G11 is not in C, because the C lens rules are unchanged. This is an open question.
    - S:392 adds "publishes the result as a spectator feed". G9 does not say this, but it is not a contradiction of S:390.
2. There are 10 rows, D65 to D74. Every row is Proposed and has a seat. Four rows have no G link (H1). The register has 74 rows in total, with no gap and no duplicate.
3. Lower-bar sentences: the amendment rewrites all twelve, or keeps them with an explicit mark (M1-M3 "None (done before 0.8)"). It adds none. Hedge counts from HEAD to the working tree: should 18 to 16, possible 9 to 7, expected 4 to 3, recommend 9 to 8, may 45 to 44. "Can" rises from 30 to 35. I read each new "can", and none of them lowers the bar.
4. Terms: "input" has 16 hits and "interface" has 7. All are consistent with S:20-22. "Presentation" has one defect (H5).
5. Section 14: see B3, H2, and H6.
6. Section 15 against Section 2: the only defect is B1. Scripts (D34), networking (D18), and persistence (D41) agree.
7. C: the eight lines are fixed. The files exist (`ls docs/measurements`, `crates/turts_sim/src`, `crates/turts_client/src`). `bash docs/agents/check.sh` exit 0. C:165 together with C:174 now forbids an agent to assume a CEO decision (B1).
8. STE: see L1.

## What the document leaves undecided

- The form of the local server (D41, S:70).
- Transport, schemas, and update policies (S:741).
- Fog of war has no register row (S:746).
- The scripting language, until MS.
- The player check for M6 and M8 to M13.
- The authors of every "set by" acceptance.
- Where G11 and G12 land: the CLAUDE.md lens rules or the design doc.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `git diff` of S and C to `diff.txt` | artefact scope | n/a (input) | 135+/75- |
| probe.py R1: count and parse `^\| D\d\d` rows | C:49 "D01 to D74"; each row of D65-74 has Proposed, a seat, and a G link | a count other than 74, a gap, or a missing field | 74, no gap; 4 rows with no G link |
| probe.py R3: substring test of R:56 against S | S:113 has G1 verbatim | False | True |
| probe.py R4: term sweep for input, interface, presentation, register | S:20-22 one meaning each | a hit in another meaning | input and interface clean; presentation (D37) and register defects |
| probe.py R5: hedge word counts, HEAD against working tree | no new lower-bar sentence | a count that rises with a lowering use | only "can" rises; each new hit read, none lowers the bar |
| `rtk proxy grep` solo, local save, standalone | the D41 revision contradicts no other line | an old "no solo" line | none outside S:70, S:277, S:751 |
| `rtk proxy grep` determinis | G9 contradicts no line | a "not required" line in same-build scope | S:261, S:265 cross-platform only; consistent |
| `rtk proxy grep` register; signs off, person, blind playtest, completeness gate | G12 recorded; terms defined | a definition or sign-off line | none (B2, H2, H4) |
| `sed` content_m3_validation.rs:372-430 | S:794 quotes three rules | other text | :375, :382, :427 match |
| `ls` measurements and crate src; Cargo.toml grep | C:9-13, C:148, C:153, C:231 | a missing file or another pin | all present; bevy `=0.20.0`, lightyear `=0.31.0` |
| `bash docs/agents/check.sh` | lens 7 roster check | non-zero exit | exit 0 |

The long-sentence probe output is at `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/amend/rig/long.txt`. The probe script is in the same directory.

## NOT TRACED

- Whether lightyear 0.31 "uses Replicon" (S:376). This is a domain fact.
- The P1-P8 results in `m1-network-spike.md`. I did not read them, and I took the author's citation as given.
- Whether the CEO intended G16 to override G5, G6, and G7 (B1). Only the CEO can answer.
- The Seconds of D65-D69 and D71-D73 against the TD table. I checked only D70.
- The `check_fixtures.sh` run. No agent file changed.

What I changed: none in the repository. I wrote this file and `rig/` in the scratchpad only.

## Re-check after the repair pass (2026-10-29)

VERDICT=pass-with-changes BLOCKING=0 HIGH=0 RUNS=6

Closed:
- B1: D65 at S:94, D67 at S:96, and D73 at S:102 now say "Decided line; design Proposed". S:105 defines that status. S:756 and S:757 agree with it, and so do C:174-180.
- B2: G12 is now at S:703 and C:232-235.
- B3: "As before" now has 0 hits, and S:703 now gives the rule.
- H1: all 10 rows carry G16.
- H2: the terms are now defined at S:705-712.
- H3: invariant 15 is at S:581, and the M11 replay test is at S:735.
- H4: a sweep for "register" without "decision" or "gap" gives 0 hits.
- H5: D37 has a note at S:66.

Remaining. None of these blocks the amendment.
1. Medium, S:850. Section 18 still lists the "player control model" and the "order envelope" as "Proposed and not yet designed". The text does not say that D65, D67, and D73 carry a decided line. Fix: add "(D65, D67, D73: decided line)".
2. Medium, S:703 and S:731. S:703 requires "a check that a person can fail from the pixels". The M7 cell is a budget check, so a person cannot fail it from the pixels. Fix: name M7 as an exception in S:703.
3. Medium, S:728. H6 is still open. The MS acceptance "Browser and native measurements" cannot fail.
4. Low, S:711. "fixture input" uses "input" alone. S:21 does not allow this. Fix: write "fixture data".
5. Low, S:867. "each row with an acceptance a player can fail" is false for QA0, MS, and M1 to M3.

Runs:
- I read the repaired lines.
- A Python sweep checked the bare uses of "register", "input", and "interface", the G16 links, and the invariant count (15).
- A grep checked for "As before" and read the S:850 and S:867 sentences.
- I read S:703-761.
- `bash docs/agents/check.sh` exits with 0.

NOT TRACED: the owner seats at S:761.
