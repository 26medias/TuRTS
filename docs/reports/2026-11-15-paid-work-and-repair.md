# Paid work and repair

Status: accepted and delivered. Main integration is authorized; no push is authorized.

## Result

API 0.2.14 adds incremental production and construction payment, safe construction cancellation, and repairers.
Protocol 8 carries exact owner repair state and construction recipe identities.
Default Skirmish content and balance stay unchanged. A separate authored fixture exercises the new controls.

Payments use exact cumulative floors and atomic progress. Funding recovery retains fractional work and display hysteresis.
Destruction releases claims and records credited and discarded refunds. Refund capacity cannot prevent lethal damage.
Repair uses normal movement, footprint reach, exact fractional costs, and persistent debt across orders.
Automatic repair respects authored filters, physical priorities, symmetric ties, and the persistent Stop latch.

Evidence root: `target/mc-evidence/paid-work-and-repair/` in the main checkout.

## Acceptance evidence

| Row | Executed evidence | Final delivery status |
| --- | --- | --- |
| PW1 | Runtime validation and public package tests; `workspace-final.log`. | PASS. |
| PW2 | `payment/`, audit `progress-10-false`, `progress-10-true`, `progress-20-false`, and `progress-20-true` streams. | PASS. |
| PW3 | Paid-work tests and `audit/wait-income-recovery.jsonl`; browser site stall and income recovery. | PASS. |
| PW4 | Paid-work final-admission, blocked output, physical-order and FIFO tests. | PASS. |
| PW5 | Paid-destruction cancellation tests and original-endpoint audit streams; real Stop publication regressions. | PASS. |
| PW6 | Ten paid-destruction tests; actual lethal balance-headroom and wide-discard audit streams. | PASS. |
| RP1 | Public repair/order tests; real browser approach and target selection. | PASS. |
| RP2 | Pure heal, repair unit/public, and fractional audit tests at two tick rates. | PASS. |
| RP3 | Repair source-priority, target removal, health cap, and debt tests. | PASS. |
| RP4 | Repair latch, auto-target/filter/tie, combat, and power-independence tests. | PASS. |
| IO1 | Protocol, export, repair-store, HUD, prediction, and frozen-stream checks. | PASS. Six stale scan/version assertions passed after scoped repairs. |
| IO2 | `client/browser-full/result.json`, inspected screenshots, actual mouse input, and committed accounting comparisons. | PASS on the sealed normal release: `final/paid/result.json`. |
| IO3 | Repeated independently booted paid/repair worlds match each audit boundary at 10 and 20 Hz. `review/result.json`: independent review PASS with executed overflow and audit/tamper checks. | PASS. Full gates, process acceptance, cold probes, performance, and the actual launcher pass. |

The preview feature probe passed in 41.16 seconds. Stop removed the held request at tick 214 and retained both placed sites.
Paid training completed. Repair approached the target, waited for funds, resumed after income, and reached 184/200 health.
Reload restored active owner repair state. Foreign selection hid private repair data. Repair Stop cleared it.
Each captured balance, site progress, Paid, and Refund display matched committed records. No page error occurred.
The same full probe also passes on the sealed normal release at clean source `d1bcc482564f3a01d6bb90d91d1dab7e9cb62694`.
`final/combat-loop/result.json` passes training and construction. `final/win/result.json` records visible Victory at 152.924 seconds.
The own castle retained 1,000 health. Nine own combatants survived; no enemy combatant survived.
All three final cold probes pass in `final/cold-1280/`, `final/cold-1024/`, and `final/cold-buttons/`.
The checker returns PARTIAL-PASS for this scripted subset; full manual/reference readiness remains open.
The 400-unit High/Low rows pass at 59.203/59.303 FPS in `final/measure-400/result.json`.
Process acceptance and the actual global launcher pass.

## Fixes and contract clarifications

The real browser run found two product defects. ChargeOverTime menu admission incorrectly required the full upfront price.
The client now applies the ordinary admission check only to policies that charge upfront.
Stop cleared held requests on the server but did not publish the empty replacement to the replica.
The server now publishes that clear once for a surviving constructor. Source removal remains a removed-only record.

SiteProgress includes its production recipe identity. Two recipes can produce the same building, so output type cannot select an exact refund rule.
Job/site WaitingForFunds can identify Balance or Stock. Repair WaitingForFunds identifies only an unaffordable Balance.
Arithmetic and source-tie repair holds use Working with typed authoritative diagnostics.

## Remaining scope and verification limits

Research ChargeOverTime, individual held-request cancellation, unfinished-site repair, resurrection, and new artwork remain deferred.
The feature does not complete the full engine.
Independent audit streams cover paid progress, recovery, cancellation, fractional repair, Stop/retarget debt, and destruction discards.
The native suite covers inventory all-or-discard, absent refund destinations, and research destruction.
Separate audit streams for those exact three boundaries were not added.
The audit checker is a set of independent contract models. It is not a complete second simulation.
Full fresh-agent and manual readiness, human multiplayer, other hardware, and larger workloads remain unverified in this milestone.

## Integration checks

The original full workspace run completed 203 targets: 2,573 tests passed, six stale scan/version assertions failed, and 40 tests were ignored.
`workspace-final-summary.json` preserves that original result. Do not read it as an exit-zero full run.
`checks-repaired.log`, `wire-repaired.log`, and `server-repaired.log` pass every failed target after the exact instrument updates.
This establishes 2,579 unique passing workspace tests across the original run and scoped repairs.
The new E9 oracle regression adds one passing test in `acceptance-oracle-sdk14.log`.
`audit-retained.log` also passes after the generic accounting-variable rename.
Strict full Clippy passes in `clippy-layout-final.log`. Formatting passes in `format-final.log`.
WASM checking and probe neutrality pass in `wasm-final.log` and `probe-neutral-final.log`.
Nine existing non-Cargo harness commands pass in `harness-final/results.json`.
The original Node suite passed 408 tests. Installing the missing pinned dependency and repairing three stale instrument calls made all 95 affected tests pass in `node-harness-repaired.log`.
Together the original run and scoped repairs establish 437 unique passing Node tests.
The exact formula counts remain enforced. A generic refund accumulator no longer triggers the content-name scan.
The old URL diagnostic assertion now follows the current message. Two old browser helpers use the existing synchronous wait wrapper.

An accidentally combined native-binary build was stopped. Standalone server/client builds in process acceptance and `release-pack-build.log` replace it.
The normal WASM release passed in `release-web-build.log`; compression passed in `release-compression.log`.
The prepared runtime is `target/mc-evidence/skirmish-paid-work-prepared`, bound to clean frozen source `d1bcc482`.
Its source hash is `24e4a323318a`; its package hash is `ed3f71dc0dcd`.
The release client has unchanged runtime code after its build. Later client changes only share a cfg(test) fixture.

The E9 process oracle now retains exact progress and repair families instead of omitting them.
The complete lifecycle E9 passes in `acceptance-after-e9/`. The loss/buffer case exposed an older incomplete active-rate expectation.
The repaired oracle validates every captured rate against its original audit tick, then replays delivered anchors to the report tip.
It rejects poisoned historical rates and charged rows. No comparison field is removed.
Retained haul/conversion/charge captures all pass in `acceptance-oracle-retained-*.log`.
The real-simulation anchor regression passes in `acceptance-oracle-sdk14-anchors.log`.
The full loss/buffer process rerun passes in 62.06 seconds in `acceptance-after-anchors/`.

Required release process acceptance passes across `acceptance-final/`, `acceptance-after-e9/`, and `acceptance-after-anchors/`.
The last native reference required its supported four-job build setting; only that unrun tail was executed at that setting.
Both final native checks pass in `acceptance-hd1-reference.log` and `acceptance-hd1.log`.
The source keeps the same runtime. Only independent test instruments changed after the accepted browser candidate.
Whole-workspace Clippy and final oracle-target Clippy pass in `clippy-layout-final.log` and `clippy-oracle-retained.log`.
The original default workspace count remains distinct from two added passing oracle regressions and three explicit retained-capture runs.
The parameterized retained-capture helper is ignored by default and requires explicit evidence paths.

## Delivered runtime

The three global launchers point to `/home/julien/Projects/TuRTS-paid-play` and `target/mc-evidence/skirmish-paid-work-prepared`.
Their flags and modes are preserved. Backups are in `launcher-before/`.
The actual global `play-skirmish.sh` passes startup, map/model readiness, Sound off, and compressed delivery.
Raw WASM is 61,102,292 bytes. Gzip is 15,146,135 bytes. The decoded transfer hash matches the raw WASM.
Evidence is in `final/summary.md` and its linked launcher folder.
Only the normal computer-mode launcher was started. Two-player and shared paths were verified but those modes were not rerun.
All test profiles were removed. The final wrappers report no surviving processes.
The separate API15 public-bot work is unfinished and is excluded from this milestone.
