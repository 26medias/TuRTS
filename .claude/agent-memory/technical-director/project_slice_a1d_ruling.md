---
name: slice-a1d-ruling
description: 2026-11-12 A1d ruling (ii) facts - C6 probe run, ledger reads logged catalog, ResMut gap in writer scans, compound-name scan probe, TDI line drift
metadata:
  type: project
---

Verified 2026-11-12 in lane TuRTS-pre-a1d-review (a921096b, base afb9c6de). Ruling file:
`TuRTS-mp1/docs/research/mp1/ruling-slice-a1d.md` (RULING=(ii): conservation.rs stays in the test tree).

- C6 probe (qa-lead scratch `a1d-review/zz_a1drev_c6.rs`, run via `run_plant.sh`): planted -> ledger Ok,
  source REFUND red; clean -> red at its guard :149. Ledger ratio comes from the logged catalog
  (turts_audit/src/production.rs:37-43); registry C6 note tests/m3/refutations.json:158 relies on independence.
- D :73-74 "one oracle" came from my TDI :40, not from the CEO decision (vault decision has 0 "conservation").
- movement_writers.rs visitor (:186-253) has no `ResMut<T>`/`Mut<T>` arm; MovementPools is a Resource
  (movement/mod.rs:102). Scan roots: sim, server, pack src (:270-280).
- Compound-name probe: 49 idents / 193 uses contain a content token; `client_shell` (46) is a spec term, so a
  split rule needs an exception design.
- TDI "Damage is out" is at :38, not :35 (at 4598804a). M2 refutation rows have one `plant_path` each.

**Why:** the A1d repair re-check and later scans reuse these. **How to apply:** demand the ResMut arm and its
plant in any writer scan; never merge independent oracles without a common-mode argument. See
[[slice-a2b1-stop-ruling]], [[pre-s1-audit-crate-facts]].
