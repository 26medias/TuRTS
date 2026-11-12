# PT1 play log

## Person run 1: the CEO, 2026-11-11 (not blind)

Build: the play folder at integration commit 8d6902c (Tasks 21 and 22, the O15 UX repairs). Mod `forge`. Chrome on the development desktop, started with the WebGPU flags after a first start failed with "Unable to find a GPU" (Chrome on Linux has WebGPU off by default).

Result: FAIL. Not signed off.

The CEO's words: "It beeps non stop. I couldn't figure out a single thing to do. I don't get the goal of the game. it's confusing. It's unplayable. [...] as it is, there's nothing to play really."

Findings, in the order of their weight:

1. No goal and no first step. The tester did not find anything to do. The fail condition of row PT1 is met at the start: the tester cannot find an action, and 60 s pass with no sign of progress. Step reached on the Player path: not past step 3. Cause: NOT TRACED in detail (the tester gave no step list). The demo has a guide of five text pages and no objective, no highlighted first action, and no reaction that tells a new player what the game wants. Owner: ux-designer (guide and first steps, Task 17) with the orchestrator.
2. The alert sound repeats without end. Cause, traced by Root: `mods/forge/interface/hud.ron:130-136` gives seven alert rules the same sound `forge_alert` and a cooldown of 1.0 s each. In `forge` a loader waits and a turret needs shells for most of the time, so one of the rules fires about each second. Owner: ux-designer (content), Task 15 and Task 17.
3. The first start showed nothing useful: with WebGPU off the client panics with a console message only. Recorded as an MP1 task (a plain page that says what is missing).

What this run does not show: which Player path steps work for a person once the person knows what to do. No step after the start was reached.

Root's assessment: the automated PT1 run passes all 19 steps, because its driver knows the steps. That proves the controls work. It does not prove that a person can find them, and this run shows that a person cannot. The row PT1 exists for this difference, and it did its work.
