VERDICT=fail BLOCKING=2 HIGH=4 RUNS=7

# Gate 1, player seat: spec amendment 0.7 to 0.8

Mode: review-spec. Artefact: uncommitted `RTS-Engine-Project-Specifications.md` (S). Yardstick: my
`inventory-player.md` (B-n = item n). Spec status is given for each item. What I changed: none.

Short answer. The goal now names the person and the mouse and keyboard. That is the bar I asked for.
MC and MP1 are what I asked for. But the goal promises a person only the default game (a C&C-like).
The one milestone for "the reference games" (MB) is checked by a modder, not by a player. No
milestone covers an AoE-like, a WC3-like, a SupCom-like or a Factorio-like as played.

## Blocking

B1. S:717, MB row. Status: order Decided (G10, S:700); content Proposed.
    What I hit: MB ships a StarCraft-like mod with a client. Its player cell is "Modder builds it
    with no engine edit". I play it and it is unplayable: the row stays green. I cannot fail it
    from the pixels. This is the only row that tests the "reference games" half of the goal (S:113).
    Direction: give MB a check that a person plays the StarCraft-like mod with mouse and keyboard.

B2. S:709 (QA0) and S:716 (MS). Status: order Decided; content Proposed.
    What I hit: the column header is "Acceptance a player can fail" (S:704). The cells say
    "Completeness gate red on M1 to M4" and "Decision recorded with browser and native
    measurements". I cannot fail either from the pixels. S:700 says only "each milestone with a
    client" needs a player check, so these rows may be exempt, but the cell claims a player check.
    Direction: mark these two rows "no client, no player check" so the column does not lie.

## High, not blocking

H1. S:113, goal sentence. Status: Decided (G1).
    The person clause binds only "the default game". "Player-facing mechanics" is not defined
    (Section 1 defines presentation, player input and player interface at S:20-22, not this term).
    I can read "mechanics" as rules only. That was the exact gap of my first verdict ("the server
    could run the rules ... a person could not play"). A spec that meets this goal lets a person
    play a C&C-like (MP1, MP2). It does not make anyone prove a StarCraft-like or an AoE-like
    is playable. Direction: say that "player-facing mechanics" includes the controls and the screens.

H2. S:710, S:713 "Blind playtest". Status: content Proposed.
    The spec never defines a blind playtest. The register row had "(section 5)"
    (`00-gap-register.md:94`); the spec copy dropped it. G12 ("both; a person signs off",
    register:67) is only in the revision history (S:852). So S never says a person plays the MC
    or M4 gate. The M3 test passed on a build the CEO could not play (register:45).
    Direction: put the blind-playtest rule and the person sign-off in Section 14.

H3. CORE items with no owner and no milestone (Q1). Run 2 (`core_map.txt`).
    - B-31 visible attack, hit and death: no row. S:725 "bounded effects" is a cost check.
      MP1 can pass with units that lose health and vanish.
    - B-14 selection panel, B-15 build menu, B-16 tooltip, B-19 production bar, B-21 resource bar:
      only D67 (S:96, Proposed). No milestone row names them. MP1 catches only total absence.
    - B-24 minimap: only "minimap click" (S:710). The minimap itself is not named.
    - Union CORE: B-42 heroes, B-43 spells and target cursor, B-45 strategic zoom, B-49 neutral
      creeps and shops, B-59 blueprints, B-61 avatar WASD, B-62 machine-problem alerts: 0 hits in
      S:113-121, S:94-103, S:709-725.
    Direction: give each of these a row or an owner, or say in 3.1 which come after MB.

H4. S:712 MP2 check with S:798. Status: S:798 Decided; MP2 content Proposed.
    "A person finishes a match against the bot." S:798 says the bot needs "only be competent
    enough for initial testing". A bot that never attacks passes. That is a win with no fight.
    Direction: the check must need the bot to fight back at least once.

## Medium, not blocking

M1. S:711 MP1 check. Two people can finish a match and never run low on power. Then the 16.2
    power rule (S:787) and B-23 (power warning, CORE for this mod) are never seen. B-17 refusal
    and B-18 ghost are also not in the check. Direction: the check must make each 16.2 rule happen once.
M2. S:715 M5 check "Two players see different things". Any difference passes. B-26 black, grey,
    clear and B-27 last-seen are not in the check. Fog arrives at order 6, after MP1 and MP2
    (Decided, G10). So my first skirmish and my first bot match have no scouting and no surprise.
    I do not reopen the order. I say how it feels: like a board game with open cards.
M3. S:718, S:724, S:725: M6, M12, M13 say "As before (register section 4)". Register lines 102
    and 105 say "as before". The 0.7 table had no player column (0.7 S:674). So "as before" is
    empty. M6 has a client (underground overlay). Direction: write "no player check yet" or name one.
M4. Decided items I feel in the first ten minutes (Q3):
    - S:788 (Decided, 16.2): "Move, attack, attack-move, and automatic targeting". No stop, no
      hold (B-9, CORE). MC builds them (S:710). Needs decision: is the 16.2 list complete?
    - S:70, S:277 (Decided; local server form open): solo play has no milestone. MP2 needs a
      server I start myself. I want to click "skirmish" and play.
    - S:739 (Open): WebGPU or WebGL fallback has no milestone. The CEO's normal Chrome showed a
      renderer panic (gap-player player-8). I cannot play a game that does not start.
    - D52 (S:81, Decided, note): no shared slot. Not in my first ten minutes. It stays.
    No Decided or Baseline row blocks B-1 to B-38 outright.

## Clarity (Q4): sentences I read twice

- S:113: "A mod must be able to reproduce the player-facing mechanics of the reference RTS games,
  within the terrain exclusions of 3.2 and D10 (...), and a person must be able to play the default
  game with mouse and keyboard." 60 words, two musts. I read it twice to see the person part is
  only for the default game.
- S:712: "Remembered sightings for bots and humans alike." Fog comes at M5. What do I remember
  when I already see everything?
- S:710: "40 units sent to one point arrive as a group". I do not know what "as a group" looks like.
- S:700: "is Proposed until that spec passes gate 1". Studio words; fine for the studio, not for me.
- S:794: "a unit with health requires removal". Engine words. The next sentence explains it well.
- S:94 says "A group order is accepted or refused per unit ... (G6, option B)" with status
  Proposed. The CEO decided G6 (register:73). I could not tell if per-unit refusal reasons
  (B-17) are decided. S:187 says "Line (a) of G5 applies" while D67 is Proposed: same doubt.

## What the document leaves undecided for a player

- Who plays the gate: agent, person, or both (G12 not in S).
- The form of the local server for solo play (S:70), and its milestone (none).
- Whether the 16.2 order list (S:788) is the full list.
- Which reference games a person must be able to play, other than the default game (S:113).
- Audio (D69, S:98): no milestone row. B-30 is COMMON; silence feels like lag.
- Browser start in a normal browser (S:739 Open): no milestone.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `git diff RTS-Engine-Project-Specifications.md` | the amendment text and scope (S:1-852) | n/a, read the artefact | 106 +, 59 -; read in full |
| `python3 -I core_map.py` (this dir, `core_map.txt`) | S:710-711 and D65-D74 give every CORE B-item an owner or a row | a hit for each CORE item | NO HIT: B-19, B-21, B-42, B-45, B-49, B-59, B-61, B-62, B-37; false hits removed by hand: B-1 S:716 "Selects", B-43 S:117/119 "capability/compatibility", B-31 S:725 cost |
| `rtk proxy grep -n -o -E` 13 terms (blind playtest, signs off, tester, AoE, solo, WebGL, surrender, G12 ...) | S:710 "Blind playtest" is defined in S and a person plays it | a definition line, or "person" in Section 14 | 2 uses (S:710, S:713), 0 definitions; G12 only at S:852; "surrender" 0 |
| `awk` on S:70, 187, 251, 277, 852 + Python sentence length on S:113, 700, 710, 711, 794, 835 | S:113 is plain to a player | goal sentence under 30 words | S:113 one sentence of 60 words; S:710 59; S:711 35 |
| `git show HEAD:S` header grep + register grep lines 102, 105 | S:718, 724, 725 "As before" points to a player check | a player column in 0.7, or a check in register rows 9, 15-16 | 0.7 S:674 has 3 columns, no player column; register says "as before" |
| `rtk proxy grep` D52 and Section 1 term lines | my citations S:81 and S:20-22 | other line numbers | corrected from first draft |
| register grep "blind playtest" | the spec carried the register's pointer | "(section 5)" in S:710 | register:94 has "(section 5)"; S:710 does not |

## NOT TRACED

- I did not play a build. This is a paper review.
- The M3 demo end state that "completes" in S:710 means: not read (M3 design not opened).
- The bot, fog and effect facts of the six reference games are from play memory, not manuals.
- `Cargo.toml` and code: not opened. The lightyear pin `=0.31.0` (S:741) is NOT TRACED.
- Sections outside 1, 2, 3, 4.3 line 187, 6.2, 6.3, 14, 15, 16, 17, 18: not read.

## Re-check after the repair pass (td-repair.md)

VERDICT=pass-with-changes BLOCKING=0 HIGH=0 RUNS=3

Closed:
- B1 closed. S:729: "A person plays the StarCraft-like mod with mouse and keyboard through a blind playtest". I can fail it.
- B2 closed. S:721 and S:728 start with "No client, no player check." S:703 says the same.
- H1 closed. S:113 points to S:707. S:707 says "Internal rules alone are not player-facing mechanics."
- H2 closed. S:708 defines the blind playtest. S:703 says that the `player` agent and a person run it, and that a person signs off.
- H3 closed. core_map2.txt: every CORE item now has a row or an owner. MC S:722 covers B-14, B-15, B-16, B-19, B-21, B-24 and B-62. MP1 S:723 covers B-31, B-32 and B-37. S:761 covers B-42, B-43, B-45, B-49, B-59 and B-61.
- H4 closed. S:724: "a bot that builds, expands, and attacks at least once". I can fail it.
- Touched rows that I can fail from the pixels: MC S:722 (one clause stays weak, see R2), MP1 S:723, MP2 S:724, M5 S:727, MB S:729. QA0 and MS have no client, and S:703 says so.

Remaining (not blocking):
- R1. Needs decision. S:803 adds stop and hold to the 16.2 rules. Those rules are Decided (D55, S:84). A Decided rule changed in a repair pass. The CEO must confirm the change. I asked for this decision. I did not ask for this edit.
- R2. Medium. S:712 says "Arrive as a group: at rest, no two units are on one cell." The cell grid is not on my screen on flat ground. Direction: state the check as something that I can see, for example units that do not overlap on the screen.
- R3. Low. In S:724, "expands" has no 16.2 meaning. The rules give one command center (S:805) and do not say that a player can build a second one. Direction: the MP2 spec must say what "expands" looks like.

Runs:
| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `python3 -I core_map2.py` (new line ranges) | S:722, S:723 and S:761 give each CORE item an owner or a row | NO HIT on any CORE item | 0 NO HIT; noise hits are B-43 S:117/119 and B-1 S:728 |
| `rtk proxy grep` D55/D44 + td-repair.md stop/hold | S:803 is a repair, not a rule change | a CEO decision line for stop and hold | D55 S:84 Decided; td-repair:18 lists the edit with no CEO line |
| `awk` on S:105-125, 698-765, 795-815 | the claims in td-repair.md, row by row | a row that is missing its claimed text | all claimed texts present |
