DONE items=6/6 acceptance=21 runs_must=22-26 runs_all=30-38

# QA0 design, second repair (revision 2)

I changed one repository file: `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md`. I also updated my memory note.

| # | Fix | Where |
| --- | --- | --- |
| 1 | `resolve` runs only in `load_package` (step 1, with LR7 first). Only the Game path of `turts-pack` and the server boot reach it. The Library path parses `dependencies`, checks LR1 and LR2, keeps the field, and does not resolve. SC4, A11, and the 5.2 test rows can now exit 0. | 5.2 items 2 and 3; 7; SC4; A11 |
| 2 | Call sites of `check_lockfile`: `join.rs:168` (the advertisement exists at `join.rs:155`), `tests/acquire.rs:80,85,94`, and `tests/t2d_client_hash.rs:67,81`. | 7; A16; 2 |
| 3 | The driver rule now has Part A (banned) and Part B (allowed). `node:fs` and `node:zlib` are allowed. Part A applies to the require closure. | 4.4 |
| 4 | The test packages declare `^0.2.2`. LR7 runs before `content()`. The A12 test packs the directory, calls `parse`, then calls `load_package`. When LR7 is removed, the test turns red on the map-count text. There is one kind check (LR7), not two. | 5.2; A12; A15 |
| 5 | New rule R9: `client: no` only for the literal list `NO_CLIENT` = {QA0, MS} (S:721, S:728). `n/a` only for the literal list `NOT_MILESTONE`. | 3.3; 3.1 |
| 6 | `cases.tsv` gets a `mode` column (`spec`, `enforce-dir`). Two directory cases: `dirs/r8` and `dirs/r9`. There are 20 rows. | 3.1; 3.4; 3.5; A4; A5 |

## Additions beyond the decisions (please review)

- **Part A item A6.** It bans the built-ins `child_process`, `vm`, `worker_threads`, `inspector`, `module`, `net`, `http`, `https`, `http2`, `dgram`, `cluster` and `repl`. It also bans the globals `fetch` and `WebSocket`, and any npm package that is not on the allow list (`playwright-core` is never on it). Without A6, a driver can run code through `vm`, drive a second browser, or send orders to the server directly.
- **Part A item A4.** It also bans the `process` members that load code (`binding`, `mainModule`, `getBuiltinModule`, and others). It bans string-built computed keys on any object. Without this clause, `io["con"+"structor"]["con"+"structor"]` reaches the Function constructor.
- **T7 oracle conditions.** They now use these sources:
    - `turts_debug` `command_results` and its `applied_tick` (`debug.rs:175,196-197`);
    - the server tick log (`t7.sh:52`);
    - `page.screenshot({ scale: "css" })` (playwright-core 1.64.1, `types.d.ts:13702`).
  This closes the rigour NOT TRACED item on the oracle source and the pixel scale.

## Driver-model result

- 14 of 14 evasion samples are red. The rigour sample `global["pro"+"cess"]` is red by A3 and A4.
- `require("node:fs")` alone is green, as Part B allows.
- 7 of 7 green-set files are green: `m2-audit.js`, `t7-launch-options.js` (the two `pure` files), the PNG file on `node:zlib`, `ok-driver.js`, the zlib driver, `process["env"]`, and plain index keys.
- `t7.js` and `m3-human.js` are red.
- Output: `repair2-driver.txt`. Model: `repair-driver/dr_model2.py`.

## Runs

| Command | Claim | Refuted if | Observed |
| --- | --- | --- | --- |
| driver model v2 (`repair2-driver.txt`) | 4.4 | an evasion green or a set-2 file red | as above |
| `turts-pack` p7: Library, zero maps, `^0.2.2` (`repair2-pack.txt`) | A11 "Today" | exit 0 | exit 2 on `engine_api` (engine 0.2.1) |
| grep `check_lockfile(` | 7 list | another site | 6 sites |
| read `archive.rs:20-40`, `join.rs:155-170`, `debug.rs:170-200`, `t7.sh`, `types.d.ts` | 5.2 entry; 4.6 | public constructor; no applied field; no `scale` option | as stated |
| STE length check | under 25 words | over | new long sentences split |

## Residual

The static check can still be evaded. A driver can build a key in a variable first and then use it on an object that is not in the A4 list, for example `const k = "con"+"structor"; io[k][k]`. Code review must catch this, not the check. The stronger fix is a separate Node process for the driver with the Node permission model; that model on Node 20 is NOT TRACED.

## NOT TRACED

- Run times on the CI runners, and the runner's Python version.
- Whether the colour search works under Xvfb with Vulkan.
- The Node permission model on Node 20.
- O2 on `Exports`.
- Whether `scale: "css"` gives exactly logical pixels when the window scale factor is not 1. The type exists; I did no browser run.
