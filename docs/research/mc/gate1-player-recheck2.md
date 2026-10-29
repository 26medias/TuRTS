VERDICT=pass-with-changes FINDINGS=2 HIGH=1 RUNS=4 NOT_CHECKED=1

## Items

- N1: fixed. The station's own state fires the alert, plus once at join (D:859). The selection plays no part.
- N2: fixed. I see "Reconnecting to your game" and a 15 s countdown (D:948-950). The old session lasts 10 s at most (D:945-947), so the countdown is my sign of progress.
- N3: fixed. The marker stays until rest or Blocked, 60 s limit (D:811).

## New findings

B1. D:1190 against D:1042-1044 and D:1036-1039. High (blocking). The new goals ask for group 1, a double tap to centre, a two-point order, and the Stop key in the menu. The guide has four pages: factory, research, pause, and "what the drone player does". No page and no widget tells me Ctrl+1, the double tap, Shift+right-click, or F10. The forge HUD has no group strip. I did not know what to do. The goals for "factory", "pause" and "turret range" I could find from the card and the range circle. Fix: guide page 4 names these keys.

M1. D:596 against D:449, D:715. Medium. A big order waits as `Pending`, but `Pending` is not in the activity list, and no widget shows activity. I see the marker, then my units do nothing, with no reason. PT1 and PT2 never reach 2,048 units, so the playtest passes.

The 60 s rule: window A can still show "0/10" for about 40 s at the start (NOT TRACED, from the last re-check). The countdown covers the reload.

## What I ran

| run | claim | refute | observed |
| --- | --- | --- | --- |
| read D:855-862, 933-954, 806-814 | N1-N3 fixed | old wording | fixed |
| blind walk of D:1190 goals with only the guide (D:1042-1044) and the HUD (D:1036-1039) | each action can be found | a page or widget for the keys | none |
| grep "Pending", "activity" | D:596 | Pending in the enum or a widget | neither |
| countdown against timeout | D:945-950 | timeout over 15 s | 10 s |

Not checked: arm loader order.

## What I changed

none
