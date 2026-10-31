# MC Task6 stop: finite-map pool capacity needs a CEO ruling

Status: STOPPED pending a CEO movement-design decision. No fallback was implemented. The full MC goal remains active and incomplete.

## Confirmed conflict

Design revision12, section6.3 lines607-616 requires all pool slots to be legal and radius-inset, a pool of N + max(6,ceil(N/4)) slots, and a lone unit to take the requested point after clamping. Section14 GMU also bounds time to rest or Blocked.

The native fixture loads a valid2x2 map with one radius0.5 mobile unit at(0.5,0.5). A nonzero-group Move to(1.5,1.5) is accepted. There are no other units or obstacles. Its route is legal and clear. N=1 requires seven pool slots, with spacing1.1. Exactly two legal points exist on the specified hex lattice. Independently, seven non-overlapping radius0.5 discs require area5.4978, greater than map area4, so changing grid orientation cannot solve it.

Actual compiled test `finite_map_singleton_reaches_point_when_spares_do_not_fit` exits101. The accepted unit remains at(0.5,0.5) after201 ordinary20Hz app updates instead of arriving at(1.5,1.5). This is a semantic failure, not a setup or compilation error. The later pending/target assertions are not reached. Root read the raw log, accepted-command assertion, exact test source and independent geometry. Attribution: d3ef732 plus the preserved dirty source; this was not a clean-d3 run and not a reviewer execution.

## Proposed CEO decision — not implemented

Recommended: require all N primary slots atomically, and treat max(6,ceil(N/4)) as the desired spare count, using the legal spares available up to that count. A reachable singleton then takes P even if six spare points cannot fit. If even N primary slots cannot fit, use an explicit atomic Blocked failure through the existing failure mechanism and normal replacement/queue rules. Do not leave an accepted envelope pending forever, dispatch only part of it, fabricate off-map slots, or duplicate occupied slots.

Alternative: retain the strict full-pool count and explicitly authorize atomic Blocked/refusal when the complete pool cannot fit, including an exception to the lone-unit arrival rule. This changes the result for a reachable point. A third option is a new map-validity restriction, which would reject currently valid content.

Either choice needs an authoritative amendment/applicability ruling. Codex has not selected one.

## Why Codex stopped

Brief section4:55 says: “Stop and report to the CEO, do not decide” for “a change to the design that changes what the player sees or does”. Caching, naming, fixture details and other delegated choices do not authorize choosing between successful movement, a new failure, and fewer mandatory spares. Both the builder and independent performance architect confirmed that the literal rules cannot all be satisfied here.

## Evidence and preserved state

- Main target/mc-evidence/t6-builder/run39/finite-map-singleton-red.log and .exit: compiled semantic101.
- finite-map-singleton-pre-red.rs / .sha256 / .head: exact fixture source attribution.
- t6-review/run45/tiny-map-contract-proof.json: independent geometry and rule references.
- t6-review/run45/stopped-source-review.md and stopped-source-review-receipt.json: all17 requirement areas and source-only review; no finalPASS, no independent runtime or own plant.
- t6-root/pool-capacity-stop/receipt.json, sources/ and dirty-working-tree.patch: all four dirty files preserved with exact hashes. No reset, cleanup, merge or push occurred. Compiler cache is idle in target/mc-t6.
- LaneHEAD d3ef732ba7b18f83fb5b2074a82f178d76b6ffdc; source/test corrections remain dirty. The other18 group movement tests passed with this probe explicitly excluded; that is not a full task pass. Final workspace/frozen/refutations/browser gates and source review are unfinished. Several RUN45 source findings remain open.
- Frozen operation stream hash is unchanged: bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b.
- Build budget45/170; Task6 has3 agent runs, below estimate6/soft12/hard24. There is no budget stop.
- Main remains c3f09d0. Integration code remains the Wave4 code fb5e2d6 plus local vault-only records. No main merge is authorized.

The underlying objective is still the entire25-lane MC build, full acceptance, zero legacy ledger rows, CEO PT1/PT2 not blind, measurements and all three hosted jobs green. This stop does not narrow completion.


## Wave 4 terminal CI and speedup approval

The CEO confirmed the speedups. C39 remains the execution rule: focused checks during repairs, grouped required final checks on the stable candidate, no duplicate external Trunk build or convergence run, one compiler cache, and all required independent reviews, plants, per-merge gates and per-wave push/CI reads. This approval does not select a Task 6 pool-capacity policy.

Root read the terminal hosted jobs for run 41195343950 at integration code fb5e2d6: agents passed; check failed; browser failed. Check failed in M2 native-only semantic refutations. The named test e9_loss_and_small_buffer_converge panicked at crates/turts_server/tests/economy.rs:2173:21: automatic operation 6744: gather state differs from the independent full transition projection. The test reports 0 passed, 1 failed, 53 filtered, 830.86 seconds. The exact refutation phase and cause are NOT TRACED. This code predates Task 6. M3 native refutations and acceptance were skipped. Raw log: main target/mc-evidence/wave4-check-job.log, SHA256 0cd6d28ebc0e54ee40ac5b25d3ab6249d3a6a68769841181ba0ed5e2772a099b.

Browser retains the exact known M2_CASE_TIMEOUT E9-browser-base-economy failure; its cause is NOT TRACED. Later M3/browser refutations were skipped. No hosted-green claim. CI: https://github.com/26medias/TuRTS/actions/runs/41195343950.

Task 6 remains stopped under brief section 4 pending the separate CEO movement policy ruling. No builds, source repairs, agent dispatches, merge or push were performed. Budget stays 45/170; completed lanes stay 10/25. Main was fetched and remains c3f09d0. Disk has about 677 GiB free. No outputs were deleted. The full MC objective remains incomplete.
