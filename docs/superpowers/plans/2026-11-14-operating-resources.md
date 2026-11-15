# Operating resources implementation plan

Spec: `docs/superpowers/specs/2026-11-14-operating-resources.md`.
Anvil spec review: `target/mc-evidence/operating-resources/anvil-spec-review.md` in the main checkout.
The same reviewer completed both Anvil gates. Two review turns were used. Findings are incorporated here.

## Work and ownership

Use three implementation owners. The orchestrator owns integration and remains an implementer.
Use two child worktrees for the independent simulation implementations.
Publish the following catalog and helper contracts first. Do not duplicate them in the children.

The integration owner adds the existing schema's `provides`, `consumes`, and `income` to ResolvedUnit.
The lists retain schema types with validated, qualified resource strings, as existing constructor rules already do.
Add `when_short` to ResolvedProducer and ResolvedWeapon. Keep ResolvedConstructor's existing list.
The runtime resolves and validates these references against their resource kinds.
No new schema fields or wire record shapes are planned.
An API version bump occurs after integration with damage API 0.2.12.
Keep the next version decision in one place during that integration.

The supply owner implements `economy/power.rs` and focused native supply tests.
The income owner implements `economy/income.rs` and focused native income tests.
They agree helper signatures through the integration owner before editing.
They must not run competing native Cargo builds in the same target directory.
They can implement in parallel while the damage release uses the separate WASM cache.

## Supply implementation

Store immutable per-tick effect totals separately from live admission totals.
Both use `(slot, qualified resource)` keys and checked arithmetic.
`run_supply` replaces the effect snapshot once in its existing schedule position.
Initialize declared resources to zero for each slot. Publish those zero rows too.
Count only completed, positive-health units. Manual work Disable does not alter totals.

Expose capability-specific rate lookup for producer, constructor, machine, and research.
Machine and research use full rate. Weapon lookup applies any active Disable.
Compare active rates by exact cross-products. Keep the smallest rate without multiplying them.
Update the existing pure combined-rate rule and its tests for this explicit composition policy.
Install RateQ from the full catalog. Keep the common denominator limit.

Add a non-mutating creation preflight against live supply and income admission totals.
Use it at initial boot, producer output, and site activation/completion.
Update live totals in the same transaction as creation or completion, and release them on removal.
The integration owner adds one common admission/grant pass for all automatic creations in a tick.
Gather site-completion and producer-output candidates together before either path commits its final progress.
Compute their legal activation points with the existing placement/exit helpers.
Sort by activation point, then producer origin if needed. Do not sort the two loops separately.
Grant checked live admission in that order. Supply and income admission are one atomic decision.
The caller consumes a grant in the same transaction as the final progress and activation.
Preflight before the last running step, not only in `try_output` after progress already commits.
Keep ordinary geometric OutputBlocked behavior distinct from arithmetic refusal.
Test a site at x30 competing with a producer output at x10, then swap IDs and retain the physical winner.
The immutable effect snapshot remains unchanged until the next tick.
An overflow leaves job/site progress and all charged resources unchanged and reports the existing arithmetic refusal.

The integration owner wires rate lookup into production, research, machines, construction, combat, and acquisition.
Audit every existing `power::rate_factor`, `full_rate`, `rate_q`, and site `done += 1` call site.
Convert construction duration and health progress to RateQ consistently.
The committed view must publish actual `done`, `total`, `per_tick`, and current shortage state in that scale.
Machine and research also use the installed global Q. Validate their durations and records with Q greater than one.
Retain the existing Q=1 path and audit fields for catalogs without operating resources.

## Income implementation

Store an activation tick and income deadline for each active income unit.
Initial placements activate at tick zero. Normal spawn and site completion record their actual tick.
Do not initialize clocks from a delayed observation of the unit.
The integration owner provides activation and removal hooks at the existing transaction boundaries.
Keep the private clock state in the income module.
Hooks use prevalidated payloads and cannot fail during commit. Zero-health reactivation also needs a fresh admission grant.

Round period_ms up with checked tick conversion at boot. Reject unrepresentable periods there.
At the Income schedule position, group all due credits by slot/resource before changing balances.
Preflight each account's whole batch through the existing balance/ledger rules.
Credit the account atomically. Add an internal IncomeCredited ledger event, distinct from Converted delivery events.
Use the existing owner-only Credited effect for presentation. No wire variant is needed.
Retain unpaid due payments when the account is full. Do not add missed periods behind them.
On success, schedule the next full period from that success tick.
Remove clocks for missing, unfinished, or nonpositive-health units. Restart on later valid activation.
Confirm free credits do not feed the Gathered statistics path.
Expose one overflow-held state transition to the existing notice/result mechanism. Clear it after recovery.
Do not spam an ignored automatic refusal on every tick.

## Runtime, wire, and HUD integration

Remove only supply/income/when_short refusals whose behavior this batch completes.
Retain reserved Network scope, Queued counts, BlockStart, and unsupported effect/capability refusals.
Add actionable duplicate, kind, reference, and capability checks.
Check RateQ across the merged package catalog, not only each package separately.
Update public catalog construction fixtures with absent/empty defaults.

Fill committed supply rows for the existing owner-filtered export adapter.
Check snapshots, deltas to zero, resync, and match replacement.
Implement the HUD values and predicate from the owner replica's accepted supply records.
Wire existing supply alerts to transitions. Initial/resync baselines emit no transition alert.
Keep all names and labels in the fixture package.

## Validation and delivery

Use a separate mod fixture with two slots, two supply resources, a hybrid producer/constructor, and income.
Run OR1–OR8 as focused native contract tests, including identity swaps at overflow admission.
Include initial placement, site completion, spawn, destruction, two payers on one account, and one unaffected account.
Check rate records and construction health throughout shortage and restoration.
Repeat the same input sequence and compare state/events every tick.
Run the existing frozen no-supply/no-income streams once after integration.

Run OR9 through real mouse input in a fresh silent browser.
One independent code-running foundation review covers the completed integrated feature.
Fix its findings directly. Do not add task reviews or repeat a passed review.
Build the normal release, run the required workspace checks and three scripted player probes, and test the actual launcher.
Record OR1–OR10 evidence paths, the rate-composition choice, and any remaining limits in one feature report.
Update the README progress table, then merge the tested milestone to main. Do not push main.
