DONE items=12/12 open=5

# Spec amendment 0.7 -> 0.8 (technical-director, build mode)

Files changed: `RTS-Engine-Project-Specifications.md` (856 lines, was 809), `CLAUDE.md`. No other repository file.
Line numbers below are the new lines in the spec (S:n) and in CLAUDE.md (C:n).

## What I changed, per item

1. Header and history. S:3 version 0.8, October 29, 2026. S:852 revision entry 0.8 names the review
   (`docs/research/review-2026-10/00-gap-register.md`) and G1 to G16. S:856 footer adds a 0.8 sentence.
2. G1. S:113 goal sentence, verbatim from register G1 row (00-gap-register.md:56), marked
   "Decided (0.8, G1, CEO 2026-10-29)", plus the list of reference RTS games from register line 6-8.
   Twelve lower-bar sentences (rigour report gap-spec-rigour-reviewer.md:58-70):
   - S:7 adds "mouse-and-keyboard player control" to the feature list.
   - S:9 "can span traditional RTS mechanics" -> "must span the player-facing mechanics"; adds Supreme Commander (rigour-12).
   - S:24 adds "It does set the bar for the result: the goal in Section 3.1."
   - S:115 both "should be possible" -> "A mod must be able to build ...".
   - S:119 adds "player input, and usability validation".
   - S:121 "expected" -> "The default game uses a classic RTS camera with the camera controls of D68 (Proposed)".
   - S:131 adds "This does not lower the bar of Section 3.1 ...".
   - S:700 milestone intro: "implementation risk and coverage of the goal"; each client milestone has a player check.
   - S:702 and S:706 M1 row: kept as done; new column says "None (done before 0.8)".
   - S:731 additional validation: bot match moved out (it is MP2).
   - S:835 Section 18 first sentence rewritten: names the missing player control and D65 to D74.
   - S:837 area: "implement the public APIs against the first mod" -> against 16.2 until a person can play it.
3. Terms. S:18-22 define presentation, player input (and "recipe inputs"), player interface (and "protocol",
   "public mod API"). Fixed uses: D07 S:36 recipe inputs; D19 S:48 "index protocol"; S:117 "public mod API";
   S:142 module renamed "Presentation and player interface"; S:161, S:163, S:438, S:708 recipe inputs;
   S:184 row "Player interface" plus S:187 note; S:265 "player-input acknowledgement"; S:366 "observation and
   command facilities"; S:694 "engine-owned APIs".
4. S:835: "desktop-first delivery" removed; replaced by "browser-first delivery with the desktop executable
   secondary (D02, D54)". History 0.5 (S:847 area) "desktop-first device scope" -> "desktop-browser-first
   device scope (D54)" so the grep is 0; meaning unchanged.
5. G13. D18 S:47; 6.2 S:265 and S:271; 6.10 S:376-380 (table status "Selected", Decided paragraph citing
   `docs/measurements/m1-network-spike.md`: P1 to P8 pass, :111 "lightyear is kept"); 15 S:741.
6. D41 S:70 reopened, "server-in-process" form, `turts_client` never depends on `turts_sim`; 6.3 S:277 matches.
   D52 S:81 note (G3). D43 S:72 note (G4); 16.1 S:761 matches.
7. D65 to D74 S:94-103, all Proposed, each with seat and G link per register 00-gap-register.md:123-128 and
   gap-technical-director.md:110-120. D74 seat: technical-director (register section 5 item 7). S:105 note.
8. G8: D48 S:77 note; 17.1 S:804 Decided paragraph, owner ai-engineer, milestone MP2.
   G9: 6.2 S:267 same-build determinism, owner simulation-core-engineer; 6.11 S:392 replay statement.
9. Section 14 S:698-725: 20 rows (M1-M3 Done, then QA0, MC, MP1, MP2, M4, M4b, M5, MS, MB, M6, M7, M8-M13),
   new column "Acceptance a player can fail" copied from register section 4. G15 in MP1 row S:711. No budget numbers.
10. Section 15 S:742 Scripts (Open until MS, G14); S:743 Player input (D66); S:744 HUD (D67); S:745 Order model
    (D65, D73); S:746 Fog of war (no register row). Also S:738 Presentation (D68, D69), S:751 Persistence,
    S:752 Tools.
11. 16.2 S:794 note: MP1 lifts the three rules at content_m3_validation.rs:375, :382, :427 (read) and adds
    construction, power, damage, repair, match lifecycle.
12. CLAUDE.md, the eight stale lines of gap-spec-rigour-reviewer.md:82-89: C:7 (M1 to M3 done), C:8 (0.8),
    C:9-13 (m2.md, m3.md), C:35-38 (goal), C:49-50 (D01 to D74), C:83 "The crate layout", crate rows C:148
    turts_sim and C:153 turts_client (verified with `ls crates/turts_sim/src crates/turts_client/src`),
    C:166-175 open list (networking library closed, transport open, presentation, player control), C:179-181
    roadmap new order, C:231-232 bevy pin (`Cargo.toml:19` `=0.20.0`). Consequence edits: C:81 module name,
    C:188 D43 note.

## What I ran

| command | claim tested | refuting result | observed |
| --- | --- | --- | --- |
| `rtk proxy grep -n -c -E "should be possible"` spec | item 2 removes all lower-bar "should be possible" | count > 0 | 0 |
| `rtk proxy grep -n -E "desktop-first"` spec | item 4 removed | any hit | 0 (after the history and 0.8 entry rewording) |
| awk section 2 + `grep -c '^\| D[0-9]+'`; sort gap check | register has 74 rows, no gap | count != 74 or a gap | 74, no gap |
| term sweep `\b(inputs?\|interfaces?)\b` | one meaning each | a factory "input" or protocol "interface" left | none left; all hits player input, recipe input, or player interface |
| python table cell-count check | no broken table after edits | mismatch | none |
| `sed` content_m3_validation.rs:370-430 | 16.2 note quotes the three rules | other text | :375, :382, :427 match |
| `rtk proxy grep` spike doc | G13 evidence says lightyear kept | no such line | m1-network-spike.md:111 "lightyear is kept"; :30-37 P1-P8 pass |
| `rtk proxy grep` CLAUDE.md/D59 in check.sh, scripts, ci.yml; `bash docs/agents/check.sh` | no tool parses the edited CLAUDE.md lines | a hit or non-zero exit | 0 hits; exit 0 |
| `ls .claude/agents/` | seats named in rows exist | missing name | all exist except `ux-designer` (G11 adds it) |

## Open questions for the orchestrator

1. Networking row (S:741): I closed the library only. G13 names the library; transport (WebSocket in use),
   schemas, and update policies stay open. If the CEO meant transport too, it needs a decision.
2. Fog of war (S:746) has no register row in D65 to D74. Needs decision: a new D75, or keep it under 4.3.
3. Milestones M6, M8 to M13: the register gives "as before" or "each has a player check" with no text. The
   rows say "set by the M<n> spec". The producer must write the check before each gate.
4. "Acceptance evidence" for QA0-MB new rows: "Set by the <milestone> spec" (I did not invent evidence).
5. D74 seat (technical-director) and the G9 determinism owner (simulation-core-engineer) and G8 owner
   (ai-engineer) are my assignments from the gap reports, not from a register row. Confirm.

## NOT TRACED

- Whether the CEO reading of G2 ("server-in-process") is final: register :74-76 says the CEO can correct it.
- No gate review ran on this amendment. It needs one before commit.
- `vault/` was not updated (task said do not edit other files).
