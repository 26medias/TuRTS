# Paid work and repair

Status: integration candidate. Release acceptance is running. Do not use this report as milestone approval yet.

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
| PW1 | Runtime validation and public package tests; `workspace-final.log`. | Candidate checks running. |
| PW2 | `payment/`, audit `progress-10-false`, `progress-10-true`, `progress-20-false`, and `progress-20-true` streams. | Focused tests pass. |
| PW3 | Paid-work tests and `audit/wait-income-recovery.jsonl`; browser site stall and income recovery. | Focused tests and preview pass. |
| PW4 | Paid-work final-admission, blocked output, physical-order and FIFO tests. | Focused tests pass. |
| PW5 | Paid-destruction cancellation tests and original-endpoint audit streams; real Stop publication regressions. | Preview Stop, placement preservation, and cancellation pass. |
| PW6 | Ten paid-destruction tests; actual lethal balance-headroom and wide-discard audit streams. | Focused tests pass. |
| RP1 | Public repair/order tests; real browser approach and target selection. | Focused tests and preview pass. |
| RP2 | Pure heal, repair unit/public, and fractional audit tests at two tick rates. | Focused tests and preview funding recovery pass. |
| RP3 | Repair source-priority, target removal, health cap, and debt tests. | Focused tests pass. |
| RP4 | Repair latch, auto-target/filter/tie, combat, and power-independence tests. | Focused tests pass. |
| IO1 | Protocol, export, repair-store, HUD, prediction, and frozen-stream checks. | Final workspace run pending. |
| IO2 | `client/browser-full/result.json`, inspected screenshots, actual mouse input, and committed accounting comparisons. | Full preview PASS. Release run pending. |
| IO3 | Repeated independently booted paid/repair worlds match each audit boundary at 10 and 20 Hz. `review/result.json`: independent review PASS with executed overflow and audit/tamper checks. | Final gates, process acceptance, cold probes, and launcher pending. |

The preview feature probe passed in 41.16 seconds. Stop removed the held request at tick 214 and retained both placed sites.
Paid training completed. Repair approached the target, waited for funds, resumed after income, and reached 184/200 health.
Reload restored active owner repair state. Foreign selection hid private repair data. Repair Stop cleared it.
Each captured balance, site progress, Paid, and Refund display matched committed records. No page error occurred.
This preview is unsealed. It proves logic and interaction, not release performance or delivery readiness.

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
