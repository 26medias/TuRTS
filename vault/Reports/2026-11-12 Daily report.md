# Daily report for the CEO, 2026-11-12

## In one paragraph

MP1 lane 0 (the contract for every later lane) went from 4 to 13 of 18 tasks merged. The pre-MP1 slice went from 9 to 11 of 21. On MC, the hosted CI went from 11 red browser jobs to 1, and two real combat defects were found and repaired. You tried the play build a second time and found it unplayable; the cause is now recorded exactly, and the way a build is checked before a person sees it has changed.

## What you decided today

- You defer the MC person play test. MC is recorded as "not signed by a person". No play request reaches you before a build passes the new readiness check. File: `vault/Decisions/2026-11-12 MC person play test deferred.md`.

## Why the play build failed for you

A guide covered the whole screen but let the map show through. While it was open, no click reached the game, and the first click on "Close" was lost. Every building has orders in the package; they were hidden behind that layer. The `player` agent had judged pictures made by a scripted driver that already knew how to close the guide. Nine acceptance conditions and a strict cold-start readiness check now bind MP1. File: `docs/research/mp1/input-ceo-second-play-test.md` on branch `mp1-design`.

## Product defects found and repaired

- **A freeze after each kill.** The owner's view stood still for about 1 second after a unit with an attack order killed its target. Cause: the server kept a reference to the dead unit, and the client refused every update that named it. Repaired on the server, with a native test. Merged on MC and on the MP1 branch.
- **Units that never fire.** An attack order from out of range stopped the unit a rounding step outside its range, and it stood there for ever (102 of 256 cases). Repaired with one rule for "stop" and "fire". Merged on the MP1 branch.
- **Not a product defect:** a third client could not start on the test machine. A probe showed that the cause is the virtual display of the test host, not the client. The frame limit is a player setting with no limit by default.

## Hosted CI (MC branch)

- The earlier statement "the browser jobs fail because there is no GPU" was wrong. The jobs failed at an offline cargo build.
- After the repairs: all 10 browser fault jobs are green. One job is red, on one case with a known timing cause; its repair is merged and waits for a hosted run.
- Five cases need a real GPU. They run in the local gate with a receipt file and a check. The paid GPU runner is not needed for MC.

## Faults of the orchestrator today

- A gate script asked for 8 build jobs where the scripts accept 4 (one false red).
- A commit in a worktree during a gate run made that run invalid (run again, green).
- My merge gates did not run test files that were outside the CI list; 37 browser test files and several Python files were outside it. Now a check fails when a test file is in no list.
- Stale plant records reached the integration branch three times. A fast check now runs at each merge.
- The disk filled to 99 %. Build caches of finished lanes are now removed during the day.
- I accepted a default (10 frames per second with no focus) that would have been a loss for players; a reviewer found it, and it is reverted.

## Open, with owners

- MP1: `L0-WIRE` (repair after review), then `L0-STORE`, the `L0-EXIT` freeze review, the cold-play tool `Q-COLD`.
- Slice: `A1c` (repair after review: one fault class had no instrument after the deletion of the replay validator).
- MC: the row follow-up (in re-check), then a full run of the 34 pixel rows on a quiet machine, the local receipts, the regression and the records.
- A possible Steam release is not in the specification. It is a later phase; say the word and the producer writes it as a proposed milestone.

## Nothing needs a decision from you today.


## Evening update

**For the CEO, in short:** no playable build yet. Today's work was mostly proof work on the slice and on MP1 lane 0. You changed the working mode: playable first, the full process only for the foundation. Codex takes over with that mode.

- **Decided by you today:** move points follow "arrive first"; the unit id never decides what a player sees; "Anvil only the big specs".
- **Ready to land (one step each):** slice A1d and A2b1; MP1 forward merge, L0-STORE, sound off by default.
- **Stopped on a measurement:** the SEL4 test repair (the next step is named in the handoff).
- **Nothing needs you now.** The next thing you see should be a build that passes the player probes.
- Handoff: `vault/Reports/2026-11-12 Handoff to Codex.md`.
