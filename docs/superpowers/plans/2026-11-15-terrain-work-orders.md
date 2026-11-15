# M4 terrain work implementation

Status: Anvil spec and plan approved by the same reviewer. Each needed one turn and one focused recheck.
The rechecks closed material accounting, geological coverage, permitted previews, and shared terrain geometry.
Do not implement until the SDK16 dependency below is accepted.
Dependency: accept, merge, and push SDK16 before implementation. Merge that accepted source into this design branch first.
Root owns integration. Use three implementation lanes after the common contract compiles.
Do not change the accepted default Skirmish or its launcher while this branch is incomplete.

## Common contract and pure rules: root first

Extend `turts_mod_api` schema/catalog/exports/version-table and runtime raw-presence/reference validation for terrain, materials, movement classes, and work tools.
Keep API17/protocol10 provisional until the predecessor is accepted. Old flat packages retain their defaults.
Use typed qualified IDs; do not select units, materials, or controls by package-local names.
Publish the resolved Rust types and command/view shapes in one integrated checkpoint before parallel lanes diverge.
Use the existing pure `turts_rules` crate for integer geometry, prism quantities, interval/depletion transitions, support/traversal tests, and per-step work/accounting calculations.
Server simulation owns private generated columns. Clients receive only permitted public terrain records.


Freeze these shared geometry contracts in that root checkpoint:

- Terrain cells are horizontal plateaus. Their height is the integer cell elevation times the authored step. Rendering does not interpolate the authoritative surface into ramps.
- Point ownership uses half-open X/Y cell bounds. The map's maximum edge belongs to its final cell. Reject outside points. Use one pure conversion on both targets, with no caller-specific epsilon.
- Render top faces and the exposed vertical faces between unequal neighbors. Ray picking returns the nearest permitted surface hit with its owning cell and that cell's integer top elevation. A wall hit selects its owning column's top level, not a fabricated fractional elevation. Equal-distance face ties use coordinates and face kind.
- Circular body support uses every cell whose interior has positive-area intersection with the disk, plus its center cell. Mere tangency does not add a support cell. Out-of-map footprint area is invalid.
- The body's ground anchor is the highest covered plateau. The covered elevation spread must not exceed its class's maximum step. This prevents terrain penetration while allowing an eligible body to straddle a small step. Static foundations require equal covered elevations in this first slice.
- For a cardinal cell transition, maximum step limits the absolute elevation difference. Maximum grade limits absolute rise in world units divided by cell width. Both limits must pass. Diagonal traversal requires both orthogonal intermediates and all four connecting cardinal transitions, plus the swept-body check; diagonal length cannot bypass a cliff or grade rule.
- Swept traversal verifies support along the complete segment at support/cell-transition events, not just endpoints or fixed-distance samples. A bounded query that cannot finish stops at the last verified point and reports waiting; it cannot assume the remaining segment is clear.
- Navigation, placement, unit grounding, camera anchoring, ghosts, labels and picking consume these common sampling/support/traversal results. No lane can substitute a bilinear or center-only approximation.
- Dynamic surface knowledge stays permission-filtered. An owned body's actual ground anchor is an authorized physical contact fact; it does not grant visibility of neighboring hidden cells. Foreign live anchors require sight; remembered anchors keep their last observed value.

Provide shared pure point-to-cell, surface sampling, disk-support, cardinal/diagonal transition, swept-clearance and ray-hit contracts before dependent lanes start.
Add independent edge/corner oracles, signed-zero/max-edge checks, shallow steps, narrow gaps, fast sweeps across a one-cell cliff, and footprint support cases.
A lane cannot replace these rules with a different renderer or pathfinding interpretation.

Use integer length quanta and checked u128 arithmetic for volume and ratio calculations.
Choose a global maximum of16,777,216 possible material steps from cell_count * (ceiling-floor), before installation.
Bound cell count at1,048,576 and interval count by that public worst case; reject combinations that cannot fit those limits.
Natural bands cover the integer dig floor through the maximum authored initial surface. Clip each generated column to its actual initial surface.
Use data-authored ordered datum bands with positive weighted material choices. A private match seed chooses a material per cell/band through a versioned deterministic rule.
No hardcoded ore/soil generator or mod-name branch is allowed. Equal-weight choices use the same declared deterministic generator on both test targets.
The public archive contains rules and initial geometry, never the selected hidden columns or seed.
The native server accepts a private seed file or generates a seed. Private replay/audit provenance retains it outside HTTP roots.
Do not add a save subsystem for this step. Existing server restart/replay evidence can reconstruct from package lock plus seed and command tape.

Add ordinary TerrainWork and UnloadCargo capabilities/behaviors and a bounded rectangle-plus-level target.
A tool definition supplies operation, fill material if applicable, rate, reach, allowed exposed materials, costs and output destination.
A terrain step uses the transaction's existing inventory/account endpoints. Output destinations distinguish physical inventory from slot accounts.
Natural output and fill input ratios must divide one prism exactly. Work must require a positive integer amount.
Retain partial work and rate carry on pause. Cancellation clears uncommitted work/carry without undoing committed terrain or accounts.
Pure tests cover changed resolution, denominator/overflow limits, bedrock, datum persistence, placement provenance, and fill/removal cycles.
Use a simple independent integer conservation oracle for randomized operation tapes. Do not duplicate the implementation in the expected-value code.

## Lane A: simulation, work, and local navigation

Own new terrain simulation modules, staged package installation, transaction overlays, order execution, and navigation integration.
Root coordinates shared order and transaction entry-point edits before they land.
Install sparse/chunked immutable public surface data and separate private geological state.
Stage changed columns, heights, inventories/balances, work rows, events and revisions in one WriteSet; preflight all failures before commit.
Extend the existing audit replay for independent step conservation and deterministic command replay.
A failed step leaves terrain, natural cursor, placed provenance, resources and revisions unchanged.

Use one TerrainWork phase after movement, tactical sight refresh and Acquire, before Repair and Combat.
Completed knowledge/world publication remains after those phases.
Work permission uses that immutable tactical basis. A committed surface exposure belongs to the completed basis.
No second movement pass uses stale navigation after that phase. Next-tick movement and placement use the new terrain revisions.
Recheck all occupancy/support and capabilities at each step; unfinished construction and newly accepted sites block edits immediately.
Resolve newly competing work claims from physical readiness. Preserve an established claimant. Exact conflicting starts remain contested.
A worker can pause or choose another designated cell from permitted geometry; hidden material cannot alter enumeration or pre-exposure metadata.

Add movement-class traversal profiles and bounded chunk/cell path searches keyed by map, class, unit radius and local revisions.
Use the full circular footprint and swept segment checks. Do not replace the existing local collision/arrival solver.
Revalidate the route at execution, even when a cache entry exists.
Retain blocked orders with a visible reason. Re-search on relevant revision changes under bounded per-tick admission.
Cache scope and dirty-region counters must demonstrate local invalidation; one edit cannot invalidate unrelated chunks.
Use geometry to choose equal-cost alternatives. Never use unit identity to choose a route, work claim, or physical winner.
Implement physical unloading through existing transfer/drop-off rules and an ordinary visible order. No teleport or resource conversion.

## Lane B: permitted transport, public bots, and server setup

Own protocol10 terrain/work records, server private seed loading/provenance, permitted export/diff, bot queries/actions, and private HTTP-boundary tests.
Use the existing bounded atomic transaction path. Extend retained-byte accounting and client validation budgets for every added vector/string/table.
Atomic payloads group surface revisions, knowledge, work and resource changes. An account credit cannot activate ahead of its terrain step.
Hidden columns/seed never enter human snapshot, advertisement, package assets, bot basis, or generic resync.
Surface memory follows SDK16 permission and reset semantics; dynamic hidden edits do not update an old observed surface.
A designation enumerates all public cell coordinates with explicit Unknown entries. Query counts/cursors/budgets cannot vary with hidden geology.
Own work status must not leak a future hidden cell's blocker, hardness, material, or support verdict.
Authoritative physical interactions retain the spec's ordinary secrecy boundary.
Reuse SDK16 cancellation, oldest-timeout, MessageId-credit, cap, and healthy-viewer isolation logic.
Extend the bot coordinator's public work/commitment model for terrain steps without inventing private strategy access.
Use paired hidden-geology worlds, actual server HTTP downloads, real human wire, and paginated bot queries as evidence.
Update exact legacy debug/projection oracles for new optional terrain fields. Validate the absent/null case explicitly; do not drop new fields to obtain equality.

## Lane C: browser, controls, and two content packages

Own permitted client terrain ingestion, chunk rendering, height-aware picking/projection, unit grounding, work command UI, status presentation, and public fixture content.
Use current Bevy0.20 repository patterns and supported local skills; do not copy older camera/renderer APIs from the historical draft.
Render only accepted surface data. Keep unknown/stale geometry consistent with permitted observations.
Update only changed chunks and required shared borders. Preserve model textures, High detail, shadows, animation, picking, and effects.
Every ground consumer must use the terrain surface: camera zoom anchor, work ghost, building ghost, entity foot point, labels, effects and command marks.
Flat maps continue through the existing path.

The mod-defined work card enters reference-height mode, then rectangle mode. The preview is public geometry plus permitted facts only.
Show operation, target height, marked Unknown cells, permitted cost information, progress and short actionable wait/refusal text.
Append, stop and cancel reuse the ordinary queue/command flow. Provide a visible Unload action and a way to resume work.
Quarry has physical cargo, a depot, a worker, a larger rover/tool machine, and an ore-bearing ridge.
The second renamed package changes terrain dimensions, classes, materials, tools and output accounting to slot balances.
Reuse existing credited assets. Do not spend this slice on new art or a long guide.
Add real-input browser evidence for work, unload/resume, passage traversal, fill/cancel, height picking and both zoom extremes.
Every Chrome uses a fresh profile and --alsa-output-device=null on an owned display.

## Frozen TW7 native workload

Freeze the authored JSON/hash and seed provenance before timing. Do not change this workload after a failed run to obtain acceptance.
Use the SDK16 128x128 world and1000 units, split equally across two slots, at20Hz.
Terrain has128x128 cells, cell width1000 quanta, elevation step250 quanta, scale1000 quanta per world unit, chunks16x16, floor-8 and ceiling8.
Use flat approach lanes plus authored ridge strips outside the ordinary combat movement lanes.
Per slot:100 moving sight sources,150 other moving combat units,100 stationary combat units,50 gatherers,50 terrain workers,50 producers.
The original500 moving units follow fixed ordinary command routes. All100 real production jobs remain active.
The100 tool workers stand at valid adjacent work positions, emit authored radius6 sight, and work distinct cells.
They alternate one-step excavation and paid fill through ordinary work orders. Placed excavation yields nothing.
Do not restore natural ore or reset columns between samples. Initial natural depletion persists through all cycles.
Each worker has0.5second work per prism and sufficient authored inputs/capacity for the complete run. Record these finite normalizations.
Require all100 tool workers to commit at least one real step per20-tick interval after warm-up; require actual combat, harvesting and all500 movers each interval.
Keep the public sight-source movement. Tool sight is additional real authored work eligibility, not an oracle visibility override.
Use explicit non-overlapping coordinates and validate work reach, support, coverage, claims and resource conservation before timing.
Freeze those coordinates and all remaining fixture values in the committed driver before its first timing run.

Run100 warm-up intervals, then1000 measured20-tick intervals in fog mode; record an equivalent AllVisible terrain-work run with the same workload.
Request a full resync per slot every100 measured intervals. Include all deferred work and final ACK drains.
Reuse the SDK16 direct full-service, per-client whole-batch and chronological debt timers, with no baseline subtraction.
Server p99<=40ms, p99.9<=50ms, maximum<=100ms; debt against50ms periods has maximum<=50ms and ends at0.
Each native client's per-tick and complete receive-batch p99<=16.667ms and maximum<=33.333ms.
Record every sample, service boundary, observed work counter, physical outcome, wire byte count, retained-memory peak, process RSS and local invalidation count.
Run on four recorded selected CPUs with no concurrent builds or browser measurements. These fixtures do not claim M7/M12 hosting capacity.
Any failed result requires implementation repair and remains in the evidence. Do not change these numeric criteria afterward.

## Final integration and acceptance: root

Integrate lane checkpoints early on one non-main branch. Do not wait for large detached lane deliveries.
Run focused affected tests during implementation. Check the compiled API/command contracts before dependent lanes diverge.
Before the expensive final WASM build, run broad native compatibility tests with --no-fail-fast so unrelated failures are collected together.
Fix failures in a batch, then recheck affected code. Do not run a complete gate campaign for each lane.
Use the shared native target and ~/.codex/bin/turts-cargo. Keep target/mc-evidence and the accepted runtime.
Run strict lint/format, the required complete gate set, process acceptance and the independent code-running foundation review on the assembled candidate.
The final foundation reviewer must be independent of these implementation lanes. One reviewer and one whole-feature review are sufficient.

After native correctness and timing are ready, build one final release browser candidate and seal its clean source identity.
Measure the unchanged400 detailed units with editable terrain and active work at normal and both extreme zooms, High quality, >=30FPS after warm-up.
Keep existing assets, model detail and camera limits. Record browser/GPU versions, visible counts, frame tails, uploads, changed chunks and errors.
Run TW6 and final cold-play scripted probes on the sealed build, with fresh silent profiles.
Record TW1-TW7 evidence and limits in one report. Fix final findings; update README progress, merge and push under standing approval.
Switch global launchers only after the candidate passes. Start the actual global launcher and confirm the new build loads.
Continue with M4b group movement and then M5 geological discovery. A completed terrain demonstration does not replace the full engine goal.
