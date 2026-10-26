---
name: rtk-diff-false-identical
description: The rtk-filtered diff reported two different files as identical; use rtk proxy diff for verification runs
metadata:
  type: feedback
---

Use `rtk proxy diff` (or `rtk proxy grep`) for any run whose result decides a finding.

**Why:** On 2026-10-26 a filtered `diff` of a spec copy against the spec printed "[ok] Files are identical" while line 3 differed ("Decided user experience" vs "Open question"). A filtered `grep -n 'Open question'` also returned 0 matches on a file that holds no such text, but the filtered output on another file hid the real count. The unfiltered run showed the truth.
**How to apply:** For every run in a review, run the comparison with `rtk proxy`. Treat a filtered "identical" or "0 matches" as unverified.
