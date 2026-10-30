---
name: mc-plan-gate2-2026-10-29
description: MC plan gate 2 facts (f69badf) - tick log has wall_ns so DET1 cmp is always red; LEDGER_LIMIT 0 breaks dr6-repaired self-test; CI skips browser steps after M2 red; io.page bypasses driver rule
metadata:
  type: project
---

Verified on HEAD f69badf, 2026-10-29 (MC plan gate 2, instrument lens):

- Tick log line is `tick,wall_ns,unit_id,x,y`, wall_ns = SystemTime::now() (`turts_server/src/ticklog.rs:64`); `--run-ticks` may run past N. A byte `cmp` of two server logs is red on every build: strip column 2 and cut at tick N.
- `driver_rule.py` with LEDGER_LIMIT 0: self-test fails on `dirs/dr6-repaired` (DR6 needs a ledger row, any row is DR7). Ledger-to-0 needs a fixture or limit-parameter change.
- `check_file` applies no rule to roles harness/instrument/unit; `check_oracle` does not follow requires. DR3 blocks a driver from requiring a harness file.
- Driver probe: `io.page.evaluate((k) => self[k](), ["tur","ts_debug"].join(""))` passes the rule (exit 0). Only the shape of `io` guards it.
- CI browser job stops at the first red: since run 40960227409 "M3 browser controls" and "M2 browser-only semantic refutations" are skipped, not red. All 8 browser plants run `m2-browser.sh` and match `m2.js` assert texts.
- `scripts/acceptance.sh` lists each ignored test explicitly; a new `#[ignore]` test has no runner until a line is added.
- `m3_m2_stream.rs` calls `apply_command(CommandKind::MoveTo)` directly: it never sees a wire-to-sim routing change.
- Web bridge has no per-frame record (`last_frame` only); "first presented frame" claims need one.

**Why:** these set which MC tests can go red or green; re-check them at MC verify.
**How to apply:** at the MC re-gate and verify, confirm each fix against these facts. See [[qa0-plan-2026-10-29]], [[gap-audit-2026-10-28]].
