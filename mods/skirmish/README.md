# First skirmish content

This package uses simple shapes. It has a computer opponent and server-owned win and loss results.
Destroy the enemy command centre to win. Keep your command centre alive.
The computer builds, harvests, and trains. Its first small attack starts after 75 seconds.

Use `bash scripts/skirmish-play.sh --prepare DIR --silent` to build it.
Use `bash scripts/skirmish-play.sh --prepared DIR --silent` to start a fresh game.
The readiness procedure is in `docs/qa/cold-play.md`. Every agent run must use `--silent`.

Select the command centre. Click **Build Barracks**. Click a free point near the base.
Select the completed barracks. Click **Train Soldier**.
Select a worker. Right-click a timber stand to harvest and deliver credits.
Select soldiers. Right-click an enemy to attack it.

The package uses the existing engine assets from `mods/cedar/assets`.
It adds no new artwork or sound. Sound starts off.
