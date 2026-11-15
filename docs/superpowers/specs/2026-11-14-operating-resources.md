# Operating resources

Status: Anvil spec and plan reviews complete. Their findings are incorporated below and in the plan.

## Purpose

Complete the existing generic supply and periodic income contracts.
Mods must be able to define power, population-like capacity, and passive income without server-specific game code.
This closes engine gaps. It does not add buildings or change the Skirmish economy.

The baseline is the project specification and MP1 design sections 6.2 and 6.10, rows U5–U7, H7, and H10.
Existing reserved network scope, queued counts, and BlockStart remain outside this batch.
The following choices resolve behavior that the earlier design leaves unclear.

## Player and mod behavior

Each slot has independent totals for each declared supply resource.
Completed units contribute their `provides` and `consumes` amounts.
Construction sites do not contribute. A destroyed or removed unit stops contributing at the next supply computation.
Manual Disable does not erase a unit's capacity or demand. It controls the existing work capability separately.
A shortage exists only when consumed exceeds provided. Equality is sufficient supply.

A capability without `when_short` keeps its normal behavior.
An affected weapon neither acquires nor fires during a shortage.
Recovery restores its prior manual enabled state. It must not enable a manually disabled weapon.
A producer or constructor uses its declared Rate during a shortage.
Existing machines and research retain full rate because their schemas have no `when_short` field.
When several supplies constrain one capability, use the smallest active Rate, compared exactly as a fraction.
Any active Disable disables a weapon. Do not multiply rates or select a rule by iteration order.
Reject unsupported effect/capability combinations with the field path and an actionable reason.

Periodic income adds its declared balance amount after each full active period.
The period rounds up to server ticks with the existing checked conversion.
Map units become active at tick zero. A created unit becomes active when it completes or spawns.
There is no immediate payment at activation and no income from a site or dead unit.
Income follows completion and health. Manual work Disable does not stop this separate capability.
Removal discards its clock. A later unit starts a new clock, even if the same content type is used.
Income must not increase the Gathered statistic.
The Gathered accumulator and its HUD source are not implemented yet.
This milestone tests the actual balance and event boundary. It does not claim an implemented statistics counter.
When statistics land, add a regression that proves periodic income leaves Gathered unchanged.

## Authoritative timing and arithmetic

Compute supply once before commands at each tick, using the existing Supply schedule position.
Every affected capability uses that same snapshot throughout the tick.
Keep this immutable effect snapshot separate from the live admission totals for creations in the tick.
Admit competing creations in physical coordinate order. Unit IDs identify requests but must not rank visible results.
One shared admission pass covers producer outputs and completed sites.
Its physical key is the producer origin for queued outputs, or the site position for completion.
These source anchors remain fixed when an earlier spawn changes the free exits.
Exact source ties receive equal priority: refuse both rather than use an ID fallback.
Refuse overflow before committing the last progress step. Ordinary geometric output blocking keeps its current behavior.
Compute income after commands and before per-tick payment, using the existing Income schedule position.
An order paid during commands sees the pre-income balance.

Use checked integer sums. Do not wrap, silently saturate, or choose recipients by unit ID.
Validate initial totals at boot. Refuse a spawn or completion that would make a supply total unrepresentable.
Such a refusal leaves resources, inventory, entities, and progress unchanged and reports the cause.
The work may resume after capacity is released.
Apply the same checked admission bound to the sum of active income amounts for each slot/resource.
This prevents a due batch larger than the account representation from becoming permanently unpayable.
For income due to one slot/resource, preflight the whole due credit batch before changing its balance.
If the balance cannot hold that batch, credit nothing for that account and retain the due payments for retry.
Do not accrue additional periods behind an unpaid payment. Resume the clock when that payment succeeds.
Other accounts remain independent. This prevents iteration order from deciding who receives a payment.

Progress uses the existing RateQ integer scheme.
Compute its denominator scale from all accepted rates at catalog installation.
Retain the existing 65,536 limit and exact progress behavior during shortage changes.
Compute the scale across the entire loaded catalog, including library packages.
Rate lookup must identify the capability. A unit with both producer and constructor can have different rates.
Scale site health, site progress, completion, and published per-tick values together.
Machine and research records still express full speed in the same scale.
All work records use the installed global scale when operating resources are present.
Packages without operating resources retain the existing Q=1 path and audit shape.

Period conversion failure is a boot error because the packer does not know the server tick rate.
Store the actual activation tick. Do not derive it from the first later income-system visit.
A nonpositive health value stops and discards a partial income clock. Revival starts a new full period.
Revival must pass the same income admission check as a new activation.
Activation and removal hooks receive all required validated data before mutation; their commit operations cannot fail.
An automatically refused overflow records one typed diagnostic transition in authoritative audit state.
Its work record is held with per_tick zero. The original plan assumed an unsolicited notice path that does not exist.
This batch does not add a wire notice or claim that the owner client displays the overflow reason.
It must not emit a refusal every tick while the same condition persists.

## Public contract and display

Resolve existing provides, consumes, income, and when_short fields into the public catalog.
Keep public types independent of Bevy.
Validate resource kind, references, positive income amount/period, duplicate rows, and rate compatibility.
No new field or protocol record is required unless review finds an existing contract cannot express the result.
Choose the next engine API version at integration, after the in-flight damage API change.
Remove only the completed fields from the unbuilt-field refusals.

Publish existing owner-only SupplyRecords on snapshots and changes.
Publish explicit zero totals when the last contributor disappears.
The client must clear stale values on resync and match replacement.
Declare zero baselines for every supply resource of the slot, including resources with no contributors.
Implement `Supply(resource, Provided | Consumed)` and `SupplyShort(resource)` from accepted replica records.
Use the existing supply alert triggers for shortage and restoration, with one event per transition.
An initial snapshot, resync, or match replacement establishes a baseline without emitting transition alerts.
Game words and labels remain mod data.

## Acceptance

| Row | Required executed result |
| --- | --- |
| OR1 | Pack and boot a separate fixture with two slots and at least two supply resources. Refuse wrong kinds, bad references, duplicates, invalid rates, and invalid periods with field paths. The authored u32-ms/u16-Hz maximum fits u64 ticks; test that maximum and the generic converter overflow separately. |
| OR2 | Check exact provided/consumed totals for completed units, sites, manual Disable, completion, spawn, removal, and destruction. Check the pre-command one-tick boundary. |
| OR3 | A shortage halves producer and constructor progress. Recovery restores full progress without lost fractions or duplicated payment. Two active constraints use the smaller rate. |
| OR4 | A shortage stops acquisition and firing. Restoration permits firing. A manual Disable remains disabled after restoration. Other slots and unaffected weapons continue. |
| OR5 | Check periodic income at two tick rates, including 1,050 ms at 20 Hz. Check initial activation, later completion, removal, and no site income. In an isolated successful-income stream, verify exact balance credits and IncomeCredited events, with no Converted, Extracted, or Stored events. Gathered accounting remains a future implementation check. |
| OR6 | Check supply overflow refusal and account income overflow retry. Swap IDs of simultaneous completions at the bound and retain the physical result. Two due payers share one nearly full account while another account succeeds. No partial mutation, wrapping, or duplicated payment is permitted. |
| OR7 | Two repeated runs produce identical totals, health, progress, balances, and events at every tick. Existing no-supply/no-income operation streams remain equal. |
| OR8 | Native export and replica tests prove owner privacy, changes to zero, resync, and new-match reset. |
| OR9 | A browser fixture shows supply totals, shortage and recovery, slowed work, and periodic balance changes through the public mod interface. Real mouse input changes supply through ordinary orders. |
| OR10 | Required workspace checks, final scripted player probes, and one independent review that runs code pass. The README progress table and feature report link the evidence. |

Every test Chrome uses a fresh profile and `--alsa-output-device=null`.
Preserve the accepted Skirmish launcher until the integrated release passes acceptance.
