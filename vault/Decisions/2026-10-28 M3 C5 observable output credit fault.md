---
type: decision
date: 2026-10-28
status: adopted
tags: [turts, m3, testing, acceptance]
---

# M3 C5 observable output-credit fault

The C5 scenario reaches `OutputBlocked` with a full output inventory. A normal output-credit
helper returns `false` at its capacity guard. If a plant calls that helper during cancellation,
transaction preflight rejects the over-capacity write before the independent conservation oracle
can observe the output credit.

The accepted C5 semantic plant is compound and exists only in the isolated mutation copy. It adds
the recipe output to the output inventory during `OutputBlocked` cancellation and bypasses the M3
transaction capacity preflight for that plant. The independent source-RON oracle reports
`BAL-shell` because stock increases by one with no matching `ItemsProduced` event. The runner
records baseline GREEN, semantic RED, exact restoration of both changed files, and restored GREEN.

This mutation does not change candidate production code. The real output-credit and transaction
capacity guards remain enabled. Cost if wrong: extra mutation setup in one source file and the
transaction helper; no gameplay behavior changes.

The C13 ammo plant also exposed an assertion-order issue. Keep `DANGLING` first, then run
`SHOT` before aggregate balances so the no-debit firing fault reports the SPEC's `SHOT` invariant.
All `BAL-ore`, `BAL-shell`, `RES`, `REFUND`, and tick-level `DMG` checks still run.
