---
type: report
date: 2026-11-01
tags: [turts, mc, codex, stop, ceo]
ai-first: true
source: codex
---

# MC STOP-PAL1: CEO ruling required for the default palette and VIS1

Implementation stopped under the user's brief section4. All three active lanes/agents are quiescent. No palette, severity, threshold, design or acceptance row was changed to obtain a pass.

The conflict is between design revision13 section9.1:850-853, which fixes the eight Okabe–Ito default hex values, and VIS1:1254, which requires CIEDE2000 distance at least20 for every pair under protan, deutan and tritan Machado simulation. The design does not state a numerical severity. The proposed native test uses the standard full-severity1.0 matrices; root did not choose a weaker simulation to force green.

| Simulation at severity1.0 | Failing pair | Agent Node ΔE00 | Independent Colour0.4.6 ΔE00 | Required |
| --- | --- | ---: | ---: | ---: |
| protan | #0072B2 / #CC79A7 | 12.24584 | 12.24710 | >=20 |
| deutan | #E69F00 / #F0E442 | 11.60511 | 11.60519 | >=20 |
| tritan | #E69F00 / #CC79A7 | 10.86859 | 10.86819 | >=20 |

The independent computation parses the palette from the design, uses Colour's own Machado matrices, sRGB transfer/XYZ/Lab conversion and CIEDE2000 implementation; it does not use the agent's Node/Rust formula. Both computations pass the published Sharma pair2.0425 within0.0001 (actual2.04245968). There are6,4,7pairs below20 respectively. This is numerical source evidence, not a claimed Task13 Cargo/browser acceptance run.

Reproducible local evidence, all under main target/mc-evidence:

- t13-builder/run62/palette-distance-repro.js SHA b4e2d7a81b73ca2f040f739eff603a0552ff7123a0ab2635d1227eaba53a9a72; actual Node exit1 and full pair JSON95efe5be…ee3.
- t13-root/palette-stop-independent/colour-library-verification.json; independent numerical data and original design SHA.
- t13-root/palette-stop-independent/verify_palette_colour.py SHA1173a34e456a7c51ccd89ace6bb9eca7435fba26016109e01e65e5298e790271.
- mc-palette-stop-20261101/independent-colour-receipt.json: actual independent exit1, stdout SHA0abe7af0…e09. Published formula reference: [Sharma, Wu and Dalal's reference data](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/). Simulation method: [Machado, Oliveira and Fernandes2009](https://profs.ic.uff.br/~laffernandes/content/publications/journal/2009_tvcg_15%286%29/machado_oliveira_fernandes-tvcg-15%286%29-2009-corrected.pdf).

## Why root cannot decide this one

Brief section4 reserves to the CEO a change to design that changes what the player sees/does or changes an acceptance row. Changing the default colours changes section9.1; lowering the threshold or weakening simulation changes VIS1's acceptance. Root's ordinary instrument/mechanism rights do not authorize hiding that mismatch. A ruling may retain the >=20 goal and permit a new validated default palette, retain the exact palette and amend VIS1, or explicitly define the approved simulation. No option is presumed approved. An asynchronous CEO question is pending.

## Preserved state

14/25lanes complete;63/170runs registered. Wave5 is complete locally at mergeaf567c4 and pushedad0eef4. Task12 candidate77ca599 passed all required gates, fresh8HUD runs, required production cursor fault/restoration and independent7probes/own plant,0openfindings. Integration11/11gates passed (1306/0/19,frozen,T7). Main remains721e56e; no main merge/push.

Task5 is on codex/mc-t5 at7e010c2227760b7cf19e53c6fc28c869b69d6ecb with8preserved WIP source files. Builder reports focused combat29/29, mc_orders7/7, frozen26+1+1green and exact frozen SHA; the finite reviewer probe run has four remaining Point completion/replay/queued activation/spread failures. Task5 is NOT accepted. Root preserved exact dirty file bytes, hashes and binary diff in mc-palette-stop-20261101/t5-wip-sources and t5-wip.patch. The sole cache is target/mc-t5, retained idle for continuation. C16 relocation is approved but not executed; no plant retirement.

Task13 is on codex/mc-t13 at128ca44697c2d83b99135e8d6c7a0ddd863e57b7, clean. Instruments/launcher preparation is committed; rendering remains the old one-colour baseline. Full production candidate patches are preserved outside the lane in t13-builder/run62/candidate. No Cargo/compiler/browser/server command or second cache was started; target/mc-t13 is absent. RUN63's finite15independent probes and runner remain outside Task5. All agents report no owned processes and remain quiescent.

## Final Wave5 hosted CI read

[CI41300331651](https://github.com/26medias/TuRTS/actions/runs/41300331651) completed: agentsSUCCESS, checkFAILURE, browserFAILURE. These are recorded and not called green.

- check failed Python refutation harness test_browser_registry_anchors_are_unique_and_install_playwright_in_scratch: FileNotFoundError for removed crates/turts_client/src/economy_ui.rs. The three registered old-panel mutations remain in the explicit Task12→21red window; Task21must rewrite their targets/static harness checks, not retire them.
- browser failed M2 browser proof waiting5000ms for Command Center selection and rebuilt owner economy panel; last phase Joined slot0. This is a NEW exact signature after panel removal, not relabeled as the old M2_CASE_TIMEOUT. Task21owns the legacy browser rewrite. The final job/step data and saved logs are wave5-ci-ad0eef4/final-jobs.json, check.log and browser.log.

No implementation or runtime resumes until the CEO ruling releases STOP-PAL1. The native/fixture/frozen values and all WIP remain intact.
