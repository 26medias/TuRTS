# MC person play test deferred (CEO, 2026-11-12)

**Status:** decided by the CEO in chat. The interpretation below is the orchestrator's; the CEO can correct it.

## What happened

The CEO ran the play build (`bash TuRTS-mc-play/play.sh forge`, build 9b3a7bd, the PT1 repair lane) for the second time. The CEO's words: "I really don't understand what to do. I clicked on every building and I don't see a single option available for me to do. I don't get what there is to test. I don't get what you need from me. Let's just move on, as I said at this stage it's not ready for me to test, this is unplayble for a human."

Fact checked by the orchestrator: the server log of that run shows the CEO's client took slot 0, so the CEO owned the buildings. Ownership was not the cause.

## Decision

- The CEO does not test MC builds. The person sign-off of PT1 and PT2 (design rule G11, G12: "a person signs off") is not given for MC.
- Orchestrator interpretation: MC can close as a technical milestone (control mechanics proven by the acceptance rows), with the person check carried to the first playable MP1 build (checkpoint C1). MC is recorded as "not signed by a person", never as "playable".
- The `player` agent judged the repaired build PLAY-NOW in a blind check. That verdict was wrong for the second time. An agent player verdict is not a stand-in for a person.

## Consequences

- No further request to the CEO to play until a build has a goal and a first action that a first-time person finds without help.
- The UX seat makes a first-click record of the forge build (what a new person sees on each building) and explains why the blind check passed. Its findings become acceptance conditions for the MP1 tasks `UI-HUD`, `UI-HINT` and row GOAL1.
