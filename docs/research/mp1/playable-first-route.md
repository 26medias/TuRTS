# First playable skirmish

The user instructions of 2026-11-12 replace the old execution process.
Work stays on `mp1/integration`. The user owns every merge and push of `main`.

## The player goal

Open the browser. Build a base. Harvest resources. Train troops. Fight an enemy.
The match ends with a clear win or loss.

The default first opponent is a computer enemy. The user can choose two humans instead.
This choice does not block construction, feedback, or match results.

## The shortest path

1. Finish the reviewed A2b1, client store, and sound changes. Test each integration push once.
2. Make the first minute usable. Show a short goal. Keep the world clear at start.
   Give each button a name and a visible response. Keep controls inside both tested window sizes.
3. Connect construction to the existing harvest, production, and combat systems.
   Use the declared mod capabilities and shared placement rules. Keep the server authoritative.
   Build one small map with one resource, a base, a production building, a worker, and a combat unit.
4. Add match defeat and victory. Show the result.
   Add the selected opponent through server orders. Do not give the client authority over outcomes.
5. Apply the arrival rule to contested movement. The first arrival keeps the point.
   Later arrivals stop nearby without overlap. Unit ids must not choose a visible result.
6. Run the cold-play probes at 1280x720 and 1024x768. Repair product failures.
   Complete the remaining cold-play checks before asking the user to play.

Construction, match authority, movement determinism, and public API changes each need a short spec.
One independent reviewer runs the changed foundation code. Other work gets one review per whole feature.
Use the existing protocol and API shapes where they fit. A declared but unsupported feature stays refused.

## Parked until the loop works

Park the remaining slice work A2b2, A4, A3, REG, and TDN unless a playable failure requires it.
Keep A2c's arrival rule in scope. Park Q2 unless the required movement change needs the frozen trace update.
Park MC receipt consolidation and SEL4 timing work unless they block the actual player probes.
Park ranked play, extra maps, extra factions, repair, rally controls, advanced lobby options, and final art.
Park an in-game restart control until the first match works.
Park supply, free income, and new payment policies for the first content package.
Use the existing harvest income and payment policy. These cuts do not remove supported API behavior.
Do not add fault registries, receipt ledgers, rulings, or new gates.

## Build and evidence

The content package is `mods/skirmish`. The launch command is in its README.
The computer uses normal server-validated orders. The server owns the final result.
The package uses API 0.2.4 and protocol 5. Unsupported capabilities remain refused.

Browser probes ran construction, training, harvesting, combat, Victory, and Defeat.
The main worktree keeps their screenshots and records under `target/mc-evidence/`:
`codex-opponent-player` and `codex-skirmish-win-2`.
The final click probes are `codex-skirmish-cold-first-click-1280x720`,
`codex-skirmish-cold-first-click-1024x768`, and `codex-skirmish-cold-buttons`.
The full readiness check also needs the fresh session and the reference calibration.

The short foundation contract is `first-skirmish-foundation.md`.
Each foundation feature has one independent review with executed probes.
Its regression cases remain in the ordinary test suites.

Keep `target/mc-evidence`. Remove caches of finished lanes after their processes stop.
