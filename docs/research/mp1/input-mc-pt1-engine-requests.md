# Engine requests from the MC PT1 play-test repair (input for MP1)

Source: the ux-designer report on the PT1 repair (`codex/mc-fix-pt1`, 2026-11-12) and the blind `player` checks of that
build. The MP1 plan gate found that this list was in no file; this file is the record. The requests are generic for every
mod. Status of each in MP1 is in `gate2-ux-designer.md`.

- E1: a guide with one page at a time, styled titles, and Next/Back. The same behaviour shows the goal page first.
- E2: in the command card, hide a `SetEnabled(b)` order when every selected unit already has `enabled == b`. Add a primary
  field for the enabled state, so a mod can show "OFF". Red test: a press with `enabled = true` shows Disable only.
- E3: an option in the `world` style: name labels over units at all times.
- E4: an objective marker: a pulsing ring on the units of one content id.
- E5: `Format` drops an empty or null value, together with its separator.
- E6: plain inventory text, and the alert-list rendering defects (row pitch, clipped first row).
- E7: a refusal notice near the pointer.
- E8: an `InserterWaiting` filter by reason (DestinationFull or SourceEmpty).
- E9: the control-group rows as one line in the Bindings list.
- E10: unit shapes or models that show the unit type.
- E11 (round 2): a predicate or value for the player's own slot, so a mod can show a text for the own side only. (The gate
  lens notes that `PrimaryOwned` exists at `crates/turts_mod_api/src/interface.rs:235`; it tests the selected unit, not the
  viewer's slot.)

## First-60-seconds checks (from the same repair; to extend the MP1 checkpoint C1 and play test PT3)

A blind tester fails the build when one of these is false:

1. Within 5 s of the first click, the screen shows the goal in one line and the first action in one line.
2. The first action is on screen with no scroll and no menu, and it has a visible mark.
3. Within 60 s, the tester does the first action with no help, and the screen shows a result.
4. Each own unit is different from each enemy unit by colour and shape on the first frame. The tester names the side of 3 units.
5. In the first 60 s, the tester hears no sound that comes again with no new event. No alert sound plays more than 3 times in 60 s.
6. Each alert names a thing that the tester can find on screen. A click on the alert shows that thing.
7. Each refused order shows its reason within 1 s, near the pointer, and in words that the tester can act on.
8. Money (or the resource) shows, and it changes within 60 s.
9. The tester reads the goal, the first action and the win condition out loud after 60 s.
10. The goal panel and the guide name no key that the player can rebind.
