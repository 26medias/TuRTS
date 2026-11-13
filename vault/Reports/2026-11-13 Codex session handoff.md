# Codex session handoff — 2026-11-13

## Start here

The user played the browser skirmish and said: "It worked and it was fun."
The first playable goal is complete. The user then requested zoom under the pointer.
That fix is complete and browser-tested. The user authorized this merge to main.
This authorization covers this merge. Keep future main merges under user control.

Follow AGENTS.md and Codex instructions. Do not read or import Claude instructions or memories.
Old process gates, binding amendments, planted-fault ledgers, and review loops do not bind Codex.
The user's fast workflow takes precedence. Build whole features and review once at the end.
Use a short spec and one independent review that runs code for server authority, wire,
determinism, and mod API contract changes. Do not add process machinery.

## Current code

- Game and zoom baseline: `d79596a64900c20c3c6a5a9a65d23b49f70c3101` from `mp1/integration`.
- Main includes the completed MC reports and MP1 design documents.
- The older pre-MP1 movement/audit fork is recorded in merge history. Its alternative runtime
  was superseded by the tested MP1 runtime. Do not reapply that fork over the current game.
- The merge changes no runtime, content, test, tool, Cargo, or CI file from the tested baseline.
- Main's local Claude files may be dirty. Those are preserved user files. Do not stage them.

The game supports building Barracks, harvesting timber for credits, training workers and
soldiers, fighting a computer, server-owned Victory/Defeat/Draw, and a New skirmish button.
The computer first attacks after three minutes. It defends before then.
Workers use small boxes. Soldiers use capsules. Training text and Cancel are separate.
Wheel zoom keeps the ground under the pointer fixed, within the map and zoom limits.

## Play

Run this from any directory:

```sh
bash /home/julien/Projects/TuRTS/target/mc-evidence/play-skirmish.sh
```

The earlier `play-skirmish-53a2d39d.sh` command is an alias to the same launcher.
It serves `target/mc-evidence/skirmish-pointer-zoom-prepared`.
The launcher still uses the clean worktree `/home/julien/Projects/TuRTS-mp1-integration`
at d79596a6. Keep that worktree while this prepared build is in use.
Do not change it without updating the launcher and preparing the next build.
A user may have the game open. Do not kill an existing Chrome or game server.

Every browser started by Codex or an agent must include `--alsa-output-device=null`.
Use fresh profiles. Never use the user's Chrome profile or change system sound.
Do not ask the user to play a new build before its scripted player probes pass.

## Verification already run

- Full integration gate set and acceptance ran for the skirmish integration.
  Obsolete checks were repaired; affected suites were then rerun successfully.
- Independent construction, match/opponent, and arrival reviews ran code. Findings were fixed.
- Browser win/loss paths were run. Final delivery Victory and New skirmish passed:
  `target/mc-evidence/codex-delivery-win-3/result.json` and
  `target/mc-evidence/codex-delivery-win-3-browser/receipt.json`.
- Cold readiness passed. Manual input/statement checks were inspected. The user then played.
  `target/mc-evidence/codex-skirmish-final-cold-check.log` and
  `target/mc-evidence/codex-skirmish-manual-cold-review.txt`.
- Zoom regression failed before the fix and passed afterward. Full interaction suite and
  interaction clippy passed. Browser: 16 zoom gestures, two viewport sizes,
  maximum measured anchor drift 0.000183 pixels. All three player probes passed.
  Evidence: `target/mc-evidence/codex-pointer-zoom-browser-2/result.json`,
  `codex-pointer-zoom-browser-and-probes-2/receipt.json`, and
  `codex-pointer-zoom-validation/`.
- No new blind-agent or foundation review is needed for these completed changes.
  Do not rerun old checks unless a new change or failure requires them.

## Next work

The user asked what is next. Proposed order:

1. Combat feedback: visible unit deaths and a clear warning when the base is attacked.
2. Barracks rally points and easier army selection.
3. One additional useful unit choice, then a balance pass.

These next features are proposals. They are not implemented yet.
Keep extra modes, factions, maps, ranked play, final artwork, and the unused remainder
of the old 76-task plan parked unless they help the current playable game.

Arrival rule: the unit that actually arrives first keeps the point. Later units stop
nearby without overlap. Unit IDs must not decide a player-visible arrival result.

## Build and disk

Keep `target/mc-evidence`. Finished lane caches were removed. A 43.5 GiB integration cache
was removed after the first delivery. One active cache was recreated for the zoom fix:
`/home/julien/Projects/TuRTS-mp1-integration/target/mc-mp1-integration`.
About 265 GiB was free after the zoom build. Check live disk state before cleanup.
Reuse one active cache. Remove finished caches after checking for active processes.

`/tmp/codex-mp1-cargo` supplies the existing ALSA development sysroot and shared target.
The sysroot is in `target/mc-evidence/native-prerequisites/alsa/sysroot`.
Do not change system audio. The temporary main merge worktree has no build cache.

Always use explicit paths with git add. Never use git add -A.
Write short, plain sentences. Ask only about player-visible choices.
