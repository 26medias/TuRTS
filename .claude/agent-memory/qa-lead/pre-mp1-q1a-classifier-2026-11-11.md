---
name: pre-mp1-q1a-classifier-2026-11-11
description: Q1a re-freeze classifier (tests/m3/refreeze_classify.py, commit 5af18c1) - red/green record, mutation reds, the greedy-match flaw, facts about the frozen stream
metadata:
  type: project
---

Verified 2026-11-11, lane TuRTS-pre-q1a-classifier, branch pre-mp1/q1a-classifier, commit 5af18c1.

- Red on stub (accept-all): `self-test: 0/21 plants red`, exit 1 (evidence pre-q1a-red-stub).
  Green: 23/23 plants red, clean pair green; unittest 26 OK (pre-q1a-gate-*).
- Mutations red then restored (cmp sha 578b940a...): P0 always true -> 20/23 (3 P0 plants);
  P3 id-order off -> 22/23; P3 key without phase -> 22/23 (flag flip on tick-1 op passes).
- First draft flaw: greedy in-order match on (unit, kinds, flag) read a flipped tick-1 op as
  "added" (an added op with the same key matched the old one). Key now holds the trace phase.
- With P0 off, a flipped base position bit gives RESULT PASS: P0 is the only check on base unit
  records (P2 = events, P3 = stream).
- Frozen stream facts: op ids are 1..n consecutive in each package; packages in order
  base, haul, acme, cedar; `json.dumps(indent=2, sort_keys=True)+"\n"` gives the exact bytes.
- Arrival calc from sources: ceil(26*sqrt2 / (6/20)) = ceil(122.5652) = 123 (scout.ron speed,
  field.ron pos, m3_m2_stream.rs MoveTo (30,30), headless_app(20)).
- CI: step "Python refutation harness tests" runs one file (ci.yml:38-39); the new self-test
  needs a ci.yml edit (not done; another lane owned ci.yml).

- Review round (simulation-core-engineer): 6 of its 25 plants passed and 2 mutants survived at
  5af18c1. Lessons: a self-test whose plants all come from the author misses "the wrong thing is
  listed" (allowed list trusted), JSON type drift (0/1 for bool, 5.0 for int: compare bytes, not
  parsed values), a declared hash that hashes no file, and a check that compares a file to itself.
- Repair 45a545f (trace/2, allowed/2): red on a build without the new rules 30/44 (14 not red);
  green 44/44, 4 clean pairs; 7 mutants each red (owner, P1 value, P0, exactly-one, arrival
  span, stream bytes, --p0 stream), restored sha 35640616...
- A wrong lacking build can fail its own clean pair and so make plants red for the wrong reason:
  check that the clean pairs are green on the lacking build before you trust its red.

See [[pre-mp1-slice-gate-2026-11-11]].
