DONE items=10/10

# Repair pass on the 0.8 amendment (technical-director, build mode)

Files changed: `RTS-Engine-Project-Specifications.md`, `CLAUDE.md`. No other repository file. S = spec, C = CLAUDE.md, new lines.

## Per item

1. B1 mixed status. D65 S:94 "Decided line (G6 B); design Proposed (G16, G11)"; D67 S:96 "(G5 a)"; D73 S:102 "(G7 yes)". S:105 explains "Decided line". S:187 keeps "applies" (now "(Decided, D67)", active voice). Section 15 HUD S:756 and Order model S:757 now say "Decided line, design open". C:172-180: the bullet says "design Proposed", and a new paragraph C:178 lists the three decided lines as "not open; assume them".
2. B2 / G11 / G12. S:703 player-check paragraph: the `player` agent and a person run the check; a person signs off. C:232-235 new "Client milestones (G11, G12)" bullet. C has no "no playable loop" rule (grep: 0 hits).
3. B3. S:703: client milestones have a check a person can fail from the pixels with mouse and keyboard; QA0 and MS have none; for M6 and M8 to M13 the producer writes the check before the gate. M7 S:731 rewritten (red when a budget is over). "As before" count: 0. Evidence cells say "The producer sets it in the <x> spec".
4. MB S:729: blind playtest by a person plus no engine edit.
5. QA0 S:721 and MS S:728 start with "No client, no player check."
6. Definitions S:705-712, source gap register section 5 items 1 to 3. S:113 adds "Section 14 defines 'player-facing mechanics'" (the G1 sentence stays verbatim).
7. G16 link on D68-D74 (S:97-103); D70 Second `game-designer` restored. Every bare "register" is now "decision register" or "gap register" (grep list: S:26, 105, 701, 705, 709, 711, 725, 731, 758, 809, 867). D37 note S:66. Invariant 15 S:581; S:267 cites it; M11 S:735 names the replay test.
8. MP2 S:724: bot builds, expands, attacks at least once; the cell names the gap with Section 17.
9. H3 items: MC S:722 (HUD items, minimap, gather, machine-problem alerts); MP1 S:723 (automatic targeting, attack/hit/death effects, cancel and refund); Section 15 S:761 "Genre breadth beyond 16.2" with owner seats.
10. M5 S:727; 16.2 S:803 adds stop and hold; Section 15 Solo play S:759, WebGL fallback S:760 (`web-platform-engineer`); MP1 S:723 "both build, power, and fight".

Also: L1 (S:70 "Revised in 0.8", S:267 split, S:809 split), L2 (D74 says "player input map (D66)", smart command defined S:105), L3 (C:141-142).

## What I ran

| command | claim tested | refuting result | observed |
| --- | --- | --- | --- |
| grep -c "should be possible" | none left | >0 | 0 |
| grep -c "desktop-first" | none left | >0 | 0 |
| awk section 2, grep -c `^\| D[0-9]+` | 74 rows | !=74 | 74 |
| grep -c "As before" | item 3 | >0 | 0 |
| grep `\bregister\b` | item 7, one meaning | a bare use | none |
| python table cell check | tables intact | mismatch | ok |
| `core_map2.py` (player probe, new line ranges, adds section 15) at `td2/` | item 9: every CORE item has a row or owner | any NO HIT | 0 NO HIT |
| grep "playable loop" C | item 2 | n/a | 0 hits |
| `bash docs/agents/check.sh` | roster check passes | non-zero | exit 0 |

## NOT TRACED

- Owner seats in S:761 are my assignment (no register row names them).
- The "smart command" example (S:105) and the QA0 fixture wording come from the gap register text and my gap table, not from qa-lead's files.
- Medium H6 (MS measured quantities) is not fixed.
- I did not count words per sentence in the new text.
- No re-gate ran.
