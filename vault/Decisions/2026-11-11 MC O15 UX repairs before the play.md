---
type: decision
date: 2026-11-11
status: decided
tags: [turts, mc, ux, orchestrator-ruling]
---

# O15: UX repairs before the person play (design revision 16)

Decided by the studio (ux-designer RUN131, Root) under the CEO guidance of 2026-11-11 that design trade-offs go to the expert seats. Three of these change what the player sees; the CEO can reverse each one.

| Item | Decision | When |
| --- | --- | --- |
| Missing glyph boxes in button labels | Button labels use `tooltip_style.font`; size stays 14 px. Cause: labels used the Bevy default font FiraMono-subset (95 glyphs, no U+00D7, no U+2014): crates/turts_client/src/widgets/mod.rs:629-633, :961-965. An optional Button `font` field in the mod API follows at MP1. | Before the play (RUN132) |
| Start overlay with locked audio (review finding UX1) | The first gesture removes "Click to play". When audio stays locked: a notice "Sound is off. Click or press a key to turn it on." for 8 s, input works, each later gesture retries audio, the menu shows "Sound: off". Before: a browser whose audio context stays suspended could never play (client_shell.rs:835-841, audio.rs:330-332; old design 10.1). | Before the play (RUN132) |
| Controls menu | A row takes its label from the order name when an order has that action as its hotkey (`key_stop` shows "Stop selected units"); mod actions first, then camera and selection, then groups. Before: 51 rows by action id; Stop was row 44 with the label "Key stop". Groups, search and keyboard navigation of the list wait for MP1. | Before the play (RUN132) |
| Research tooltip | Keep "Hardened Shells [R]: Needs 10 Shell (0/10)" (repair f486d87). "Unbound" in place of an empty key waits for MP1. | Done; MP1 |
| Row GMU text | The corner record names the true cause of each `Blocked` unit (review finding L-R2). No player change. | Done |

Alternatives not taken: ASCII in the Forge labels (hides an engine gap that every mod would hit); a time-out for the overlay (a guess, and it blocks the player in silence); keep the design for UX1 (a dead end for a player whose browser blocks audio).

Open, for MP1 (bevy-rendering-engineer): marks above the first alert row at the top clip edge of the alert list, and alert rows 62 px apart where hud.ron:89 gives 31 px; causes NOT TRACED.

Goals text for the CEO play (from design rows PT1 and PT2, and ruling O14): recorded in the Root session and sent with the page URL.
