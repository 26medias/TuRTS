VERDICT=fail BLOCKING=1 HIGH=5 RUNS=18

# Gate 1, rigour lens: `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md` (D below)

Commit: f56125d (HEAD = f56125d09cba). All spec and register line citations in D hold (run 9, 10).
What I changed in the repository: none.

## Blocking

B1. D:392 (A12), D:278. The red build "Today it boots (`lib.rs:13-38`)" does not reproduce for any archive that can reach the test after QA0.
- A Library *with* a map: today it boots (p2 exit 0). After QA0, `PackageArchive::parse` calls `validate_manifest` (`crates/turts_mod_runtime/src/archive.rs:42`), and the K3 rule refuses it on `exports.maps`. The test then never reaches `load_package`, so it never turns green.
- A Library *without* a map: today `load_package` already refuses it at `crates/turts_pack/src/lib.rs:16-23` ("exactly one map, found 0"). It does not boot. If the kind check is removed after QA0, the map-count error still fires. The test turns red only if it asserts the exact text.
- Fix: name the archive (a Library with zero maps). Assert the exact text "a library cannot be the root of a match". Change "Today" to: refused with "a package exports exactly one map, found 0" (`lib.rs:20`).

## High

H1. D:141 contradicts D:149. Line 141: "R4 to R8 apply to all listed specs". Line 149: "`--enforce` runs every `gated` spec and R8". At QA0 the gated count is 0, so R4 to R7 run on nothing. Also, one `status` column holds two facts: client (`no-client`) and gate state (`gated`). When MS passes gate 1, it cannot be both. A manual edit sets `gated`, and no rule ties that edit to a gate pass. A new MC spec can pass gate 1 with status `planned` and skip the gate, against S:721 ("before any new milestone is gated"). R8 forces a row, not enforcement. Fix: split the column (`client`, `gate`). Enforce every listed spec unless its status is in a closed exempt set fixed in the tool, as D7 does for the ledger.

H2. D:143-147, D:107 (A4). The self-test definition cannot go red on an empty case table. It also leaves multi-rule matching undefined (`mut-a` R1 and R4; M1 to M4 R1 and R3). Run 8: a literal model of D:143-144 is green on an empty `expected.tsv`. With an overlap match, `mut-a` that loses R4 still passes, and `mut-j` with an extra R2 still passes. The R8 case (D:159) needs a directory, and `expected.tsv` has no column for it. Fix: match by exact set equality. Make the self-test fail when any rule R1 to R8 has no expected-red row, and when the row count falls below a fixed count. Define a `--specs-dir` option for the R8 case.

H3. D:185 against D:214 and D:197. D:185 gives `t7.js` the role "`harness` and oracle", but D1 permits one role line only. Also, `oracle` is a role (D:180) with no rule and no file, and D:197 uses "the oracle" for a thing that the harness owns. The green state of A6 (D:383) is not defined. Fix: give `t7.js` one role, `oracle`. Define what the oracle role may do, and use "oracle" with one meaning only.

H4. D:316, D:402 (A19, S4). The green arm cannot pass on the current engine. `ENGINE_API_VERSION` is "0.2.1" (`crates/turts_mod_api/src/lib.rs:33`). `validate_manifest` refuses `^0.3` before any field table runs (`manifest.rs:32-40`). Run 6: `engine api ^0.3 does not accept engine 0.2.1`, exit 2. Fix: set `since_api` 0.2.1. Accept on `^0.2.1` and refuse on `^0.2` (both pass the engine check). As an alternative, unit-test the table with an injected engine version.

H5. D:215-217, D:417. Risk 3 says "D3, D4 close dynamic loads". Run 7 shows that this is false. A literal model of D2 to D4 is green on `const r = eval("req"+"uire"); r("node:"+"fs")`, which reads the oracle output. The control case `require("node:fs")` is red. Fix: add `eval`, `Function`, `globalThis`, `process`, and `fetch` to D2, with one red fixture each. Make D5 apply D4 to `pure` files too.

## Medium and low (non-blocking)

M1. D:446-448. The budget disagrees with D:236-238 and D:292-293. C costs 1.5 tasks and O2 costs 2, so the recommended set is 8 + 1.5 + 2 = 11.5 tasks, about 43 runs, not 10 tasks and 38 runs. D:31 says A costs "0 tasks now" and B "about 2 more tasks", but D:236-238 says 0.5 and +1. Run 13.

M2. Term collisions. "T7" is the browser join (D:387, `scripts/t7.sh`) and also plan task T7, the schema (D:442). "C1/C2" are probes (D:17) and also schema cases (D:293, D:300). D1 to D7 are driver rules, and `D<nn>` are decision ids (D:119, D:236 "D6 keeps them"). "Fixture" has three meanings: S:711, `tests/qa/driver/`, and `crates/turts_pack/tests/fixtures/`. Fix: rename (for example DR1, SC1, P0).

M3. D:88, D:455. The receipt `runs-c1.txt` records `exit=0` on all five mutations. My re-run (run 1) gives exit 1 on each. The claim holds, but the cited receipt contradicts it. Replace the receipt.

M4. The cut line (D:428-430) and O1 (D:318) remove A8, A13 (`--lib`), and A17 ("naive O2 build"). The acceptance list does not say what replaces them. The A8 tolerance (D:241) has no number. A8 also does not say how the oracle learns the pixel that the driver clicked.

L1. D:334, L8: "Red when" (an error) conflicts with "the lockfile lists the used set only" (a filter). Say which one.

L2. STE. D:32 has 34 words and D:135 has 32 words: split them. Passive voice at D:80 "Any dependency is refused": write "`validate_manifest` refuses any dependency". At D:146, name the actor of "is amended".

L3. D:250. An empty `KNOWN_CAPABILITIES` refines S:231, which is Baseline ("requested capabilities"). State it as a Baseline refinement. All `mods/` and fixture packages use `capabilities: []` (run 12), so nothing breaks.

## Scope (lens 5): pass

D:58-62 excludes MC, MP1, and player features. No section designs them. The K2 C colour search is safe for one reason only: T7 uses `mods/acme`, whose map has one unit (`mods/acme/maps/field.ron:4`). All units share one material (`crates/turts_client/src/render.rs:405`). A map with more units would need team colours, which are MC scope (S:722).

## Undecided (correctly marked as calls)

K1 to K6 are open. K3 is correctly a call against S:229 (Decided). K4 is correctly a call against S:709. K5 keeps the server refusing any root with dependencies (`resolve(root, [])`). The CEO must see that, under K5, QA0 "replaces the refusal" on the server with a check that always fails.

## What I ran

| # | Command | Claim tested | Refuting result | Observed |
|---|---|---|---|---|
| 1 | probe C1 on M1-M4, fx_green, 5 mutations (`rig-c1.txt`) | D:88, A2, A3 | any spec green, fixture red | M1-M4 exit 1 (R1, R3); green exit 0; mutations exit 1 |
| 2 | `turts-pack` (release, built after the last crate change) on p0-p6 copies (`rig-pack.txt`) | D:77-81, A9-A11, A13 | p1/p2 exit 2, p3 exit 0 | p1 0, p2 0, p3 2, p4 2, p5 2, p6 2; diffs vs `mods/acme` are single-field |
| 3 | `cargo run` schema-probe | D:298-302 | C2 accepted, C4 rejected | C0-C3 as stated, C4 accepted |
| 4 | `hook_ban.py` t7.js, m3-human.js | D:227, A6 | either green | t7.js 134,135,152,213; m3-human 7,27,35,43 |
| 5 | pack with a duplicate unit field | D:314 "today green" | exit 0 | exit 2, duplicate field |
| 6 | pack with `engine_api: "^0.3"` | D:316 S4 green arm | exit 0 | exit 2, engine_api |
| 7 | D2-D4 literal model on eval driver (`rig-driver.txt`) | D:417 | eval driver red | GREEN; control red |
| 8 | self-test literal model (`rig-selftest.txt`) | D:143-151 | empty table red | green |
| 9 | `sed` on 18 cited S lines | D:9-11, 1.3 | text mismatch | all match |
| 10 | `sed` on cited R lines | D:12-13 | mismatch | match |
| 11 | grep `validate_manifest` callers | A12 path | no parse-time call | `archive.rs:42` |
| 12 | grep capabilities/PackageKind, deny count | D:76, D:82 | production reader; not 46 | none; 46 |
| 13 | budget arithmetic | D:446-448 | sums agree | 11.5 vs 10 |
| 14 | sentence-length script | STE | none over 25 | D:32, 102, 135 |
| 15 | read `ci.yml` 30-40, 105-145 | D:93, D:356-362 | wrong job | `ci.yml:37` is the `check` job, not `agents` |
| 16 | grep `m3-human` in scripts, .github | D:31 | a CI hit | none |
| 17 | grep unit colours in client | K2 C | per-slot colour | one material |
| 18 | `ls` specs dir | R8 | file without a status | 7 files, all covered by D:102 statuses |

## NOT TRACED

- A15 red build: whether the boot succeeds when the refusal is lifted and the `resolve` call is skipped.
- A8 on Xvfb with Vulkan.
- The Python version on the CI runner.
- O2 with the five required fields (`schema.rs:331-340`).
- D:84 parse sites beyond `archive.rs:42`.
- The yardstick 181 count: probe output `CORE=181` only, no independent awk.

---

# Re-check, revision 1 (2026-10-29)

VERDICT=fail BLOCKING=1 HIGH=1 RUNS=6

Scope: B1, H1 to H5, the budget, the cut line, and new contradictions in the repaired lines. D below means revision 1. What I changed in the repository: none.

## Status of the gate 1 findings

| Finding | Status | Evidence |
| --- | --- | --- |
| B1 (A12) | Partly closed. The archive and the exact text are named now. The mechanism is wrong: see N2. | D:406-410, D:597-599 |
| H1 | Closed. `client` and `gate` are separate columns. The exempt set is closed. A new `client: yes` spec must be green. One residual: N4. | D:124, D:184-193 |
| H2 | Closed. Re-run R1 (below): an empty file is red on conditions 1 to 3, a subset is red, a superset is red, a file with no R8 row is red, a green M4 is red, and the full 19-row file is green. One residual: N5. | D:195-205 |
| H3 | Closed. One role for each file. Oracle defined. DR6 added. | D:250-261, D:313 |
| H4 | Closed. SC3 calls the table directly with since 0.2.1. The green half is reachable. | D:438-447, D:461 |
| H5 | Closed for the listed forms. My gate 1 evasion is now red (DR2 `eval`, `module`). One residual: N3. | D:309-310 |
| M1 budget | Closed. 6 x 3.75 = 22.5 (26 with one repair entry). 8 x 3.75 = 30 (38 with O2 and one repair entry). C = P7 + P8 = 2. B - C = 1. A = 0.5. | D:39-40, D:339-341, D:659-661 |
| M4 cut line | Closed. Each slip item names what replaces it. Only A8 is tier S. | D:556-566, D:590-592 |

## New findings

N1. **Blocking.** D:312 (DR5), D:275, D:341, D:584 (A6). DR5 applies DR2 to DR4 to every `pure` file. A6 cannot go green on the tree with the roles of 4.2:
- `tests/browser/m2-audit.js:4` `require("node:fs")` (DR4), and `:286` `module.exports` (DR2 `module`);
- `tests/browser/t7-launch-options.js:24` `process.env` and `:36` `module.exports` (DR2).
The PNG decoder "on `node:zlib` in a `pure` file" (D:341) is also red, by DR4 `node:zlib`. My gate 1 H5 fix ("make D5 apply D4 to `pure` files") caused this conflict.
Fix: split the role in two. `pure` means a driver may require it, and it stays strict. A new role (for example `helper`) means a driver may not require it; give it to the two files above. Put `node:zlib` in the DR4 allow list, or put the decoder in the harness.

N2. **High.** D:406-410, D:597-599 (A12), D:595 (A11), D:605-608 (A15), against D:450 and D:462 (SC4).
- `PackageArchive.files` is private (`crates/turts_mod_runtime/src/archive.rs:28`), and `parse` is its only constructor (`archive.rs:35`). So a test in `crates/turts_pack/tests` cannot "call `load_package` directly" without parse (D:408).
- `load_package` calls `archive.content()` first (`crates/turts_pack/src/lib.rs:14`). The version table runs inside `content()` (D:447).
- So, if the test archive declares `^0.2`, the table refuses `kind: Library` on `engine_api` before the kind check. The A12 text never appears, and the test never turns green.
- A11 expects exit 0 and cites p6 (`^0.2`), but SC4 (D:462) says that this package is refused.
- Today the A12 archive fails at parse, at `manifest.rs:55` ("... at M1, found 0"), not at `lib.rs:16-23`. The test is still red today.
Fix:
- The test packages of A11, A12, and A15 declare `^0.2.2`.
- The A12 test builds its archive with `parse`.
- The kind check is the first statement of `load_package`, before `content()`.
- State the order of `resolve` and `content()` in `turts-pack`, so that A15 gets LR3.

N3. **Medium.** D:309, D:630. `global` is not a DR2 word. The driver `global["pro"+"cess"]["mainModule"]["req"+"uire"]("node:"+"fs")` passes my model of DR2 to DR4 (R2). On Node v24.10.0 it reaches `fs` (R3). Risk 3 names only the `constructor` residual. Fix: add `global` and `Reflect` to DR2, and make a computed member key that is not one string literal red. As an alternative, run the driver in its own process.

N4. **Medium.** D:124, D:184. The `client` value is declared by the author of the spec. A client spec registered as `client: no` or `n/a` skips R1 to R3. Fix: the tool fixes the `no` set {QA0, MS} and the `n/a` files by name, as it does for the exempt set.

N5. **Low.** D:129, D:226-229. The `cases.tsv` columns (`file`, `as_milestone`, `expected_rules`) cannot describe the R8 case (`--enforce --specs-dir`). Add a `mode` column.

NOT TRACED: which server output lets `t7-oracle.js` see an applied `MoveTo` (D:350). The tick log has no command field (CLAUDE.md). Also the screenshot pixel scale against logical pixels in condition 3.

## Runs (re-check)

| # | Command | Claim tested | Refuting result | Observed |
|---|---|---|---|---|
| R1 | `rig-recheck/selftest2.py` on 6 case files (`rig-recheck-selftest.txt`) | D:195-202 | empty, subset, superset, no-R8, or green M4 is green; full file red | all five red; full green |
| R2 | `rig-recheck/dr.py` on 5 files (`rig-recheck-driver.txt`) | D:309-312, D:630 | `global` evasion red; pure files green | evasion GREEN; `m2-audit.js`, `t7-launch-options.js`, `png-pure.js` RED |
| R3 | `node run-evade.js` (v24.10.0) | D:630, only the `constructor` residual remains | throws | reaches `fs.readFileSync` |
| R4 | grep DR2 words in the two `pure` files | A6 green | no hits | `m2-audit.js:4,286`; `t7-launch-options.js:24,36` |
| R5 | read `archive.rs:20-35`, `archive.rs:125-135` | D:408 direct call | a public constructor | `files` private; `parse` only |
| R6 | budget arithmetic | D:659-661 | sums disagree | agree |

---

# Final re-check, revision 2 (2026-10-29)

VERDICT=pass-with-changes BLOCKING=0 HIGH=1 RUNS=5

Scope: N1, N2, the `client` rule (R9), the R8 case line, and new contradictions. D below means revision 2. What I changed in the repository: none.

## Status

| Finding | Status | Evidence |
| --- | --- | --- |
| N1 (`pure` files red, A6 unreachable) | Closed. Part B allows `node:fs`, `node:zlib`, `process.env`, and `module.exports`. The author's model gives green on both `pure` files. | D:347-349, D:370-372 |
| N2 (A12 mechanism) | Closed. LR7 is the first rule of `resolve`, and `resolve` is step 1 of `load_package`, before `content()`. The test packages declare `^0.2.2`. The A12 test enters through `parse`. With LR7 removed, the map-count text fires. A15 gets LR3 first. | D:454-460, D:478-484, D:555, D:694-699 |
| N3 (`global` evasion) | Closed. The author's model is red on my sample: A3, A4 (run F1). | D:337-338 |
| N4 (`client` self-declared) | Closed for the `client` value. R9 uses the literal lists, and the `dirs/r9` case covers it. S:728 is the MS row. A residual remains: F-M1. | D:190-196 |
| N5 (R8 case row) | Closed. There is a `mode` column, and the case directories are `dirs/r8` and `dirs/r9`. My self-test model with R9 and 20 rows: empty red, no-R9 red, green `dirs/r9` red, full green (run F2). | D:137-138, D:212-221, D:245-249 |

## Remaining findings

F-H1. **High.** D:347-348 against D:340 (A6). Part B allows `node:fs` in a driver "so that the driver writes screenshots", but `io.save` already writes artefacts (D:307). A6 bans "a second channel to ... the server", but `node:fs` reads the server output files.
- Run F1: a driver that reads `target/t7/server-tick.log` and right-clicks at the last unit position is GREEN in the author's model (`dr_model2.py`).
- So the check passes a driver that does not use only pointer events, key events, and screenshots (S:710).
- Fix: ban `node:fs` and `node:fs/promises` in DR2 (the driver and its require closure). Do not ban them in DR4, so `m2-audit.js` stays green: no driver requires it. Add one red sample.

F-M1. **Medium.** D:202-203, D:192. The exempt set ("the specs of M1, M2, M3 and M4") and `NO_CLIENT` match on the `milestone` value, which the author sets. An MC spec registered as milestone `M4` with `gate: draft` is exempt from every rule. Fix: name the exempt specs by file name, or add a rule that each milestone id appears in at most one registry row.

F-M2. **Medium.** Not in the model. `module.parent` is a literal member access, so Part B allows it. On Node v24.10.0 it reaches the exports of the requiring module (run F3). If `harness/run.js` exports `page`, a driver reaches it. Fix: ban `module.parent`, `module.children`, and `require.cache` in Part A. As an alternative, state that the harness exports nothing. (`require.cache` is already red by A5.)

F-L1. D:134 is stale. It lists only `--specs-dir` and "rules R5 to R8 added". D:201 adds `--registry` and R9. D:233 also still reads "R1 to R8".

Accepted residual: a key built in a variable (`const k = "con"+"structor"; io[k][k]`) is GREEN (run F1). D:630 and the author's report name it. Code review covers it.

## Runs (final re-check)

| # | Command | Claim tested | Refuting result | Observed |
|---|---|---|---|---|
| F1 | `repair-driver/dr_model2.py` on 5 samples (`rig-final-driver.txt`) | D:331-349: Part A closes the channels | fs-ticklog red; global red | fs-ticklog GREEN; mod-parent GREEN; var-key GREEN; req-cache RED; global RED |
| F2 | `rig-recheck/selftest3.py` (R1-R9, minimum 20) on 4 files (`rig-final-selftest.txt`) | D:212-221, D:245-249 | empty, no-R9, or green r9 is green | all red; full 20 rows green |
| F3 | `node harness-sim.js` (v24.10.0) | F-M2 path works | `module.parent` undefined | prints `PAGE` |
| F4 | read D:128-140, D:180-260, D:323-379, D:440-600, D:690-700 | N1, N2, N4, N5 | text missing | present |
| F5 | `sed` S:728 | D:193 cites the MS row | another row | MS row |
