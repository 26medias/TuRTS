RESULT=PASS

# Repair 3 of L0-SIM: merge of integration b215c3a (simulation-core-engineer)

Worktree `/home/julien/Projects/TuRTS-mp1-l0-sim`, branch `mp1/l0-sim`, pushed (c0ab357..b7fff19). Cache `mc-mp1-l0-sim`. Evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/l0sim-fix3-*/command.log`. Scratch: `scratchpad/l0sim-fix3/`. No re-freeze, no headed run, no merge into integration.

## Merge

- Commit b7fff19 `Merge origin/mp1/integration (b215c3a) into mp1/l0-sim` (git "ort", 103 files changed from the first parent). b215c3a is an ancestor of HEAD.
- No conflict. Not even `formula_ratchet.rs` or `docs/measurements/mp1.md`: `git diff HEAD^1 HEAD --stat -- crates/turts_checks docs/measurements` is empty, so the merge changed no ratchet count and no baseline row. The ratchet is exact, and it is green on the merged tree (it is in `cargo test --workspace`, exit 0), so the table of the previous merge (0e860dc) still equals the true counts. I changed no baseline in this step.
- `SLOT_COUNT` (moved to `turts_mod_api`, re-exported by `turts_sim::slots`) merged with no edit on my side; clippy, tests and the wasm32 check pass.

## Gates on the merged head b7fff19f (via `exec_in_lane_v4.py`, label `l0sim-fix3-gates`, `MC_JOBS=4`)

| Gate | Exit |
| --- | --- |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` (1,981 passed, 0 failed, 37 ignored) | 0 |
| `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked` | 0 |
| `bash scripts/boundary.sh` | 0 |
| `python3 -I -B scripts/qa/mp1_plants.py --self-test` | 0 |
| `python3 -I -B scripts/qa/mp1_plants.py --check --all` | 0 |

`--check --all` exits 0 with no stale record of mine: no re-pin was needed in this step (the merge touched no file that a plant of mine edits in a way that moves an anchor). I did not re-run the 35 L0-SIM plants red in this step; their last red-and-restored run is `l0sim-fix2-plants` on 0e860dc (35 of 35). NOT TRACED: a red run on b7fff19; `--check` is the anchor and format check the ruling asked for.

## Trace body check (ruling in `errata-pending-6.3.md`: body byte-equal, header reported)

Merged head traces: `l0sim-fix3-hashes` (exit 0). Pre-merge reference: traces rebuilt at bfea57c in a scratch worktree (`l0sim-fix3-trace-pre2`, exit 0, removed after). The whole-file hashes at bfea57c are the original recorded ones (acme 19dcf9a3..., base d4c73ebe..., cedar 0db10e01..., haul 805309b2...). On both sides `tail -n +2 <file>` was written to a file and compared with `cmp`:

| Trace | `cmp` of body | Body sha256 (equal on both sides) | Body lines |
| --- | --- | --- | --- |
| base | equal | d78dc09392e5b96917513772488991b617e8d6bb2b70a26056075c28c4822f0c | 2,897 |
| haul | equal | 887ce95f698017026f79685347446e5240f32770e704af9d097e06eef86b074f | 4,819 |
| acme | equal | ba1c849aaee46dd490b9d7d444c4c431bc324b4970e6f5c6edb31a565bd19856 | 801 |
| cedar | equal | 09f637325c5119727a70286e1b47b0a223cf60ec1a206aebc35853a5ffe540ae | 4,711 |

Header (reported, not compared): the whole-file hashes on the merged head are the same four values as on 0e860dc: base bf2a93edb049fb243391f277f95336c62c21e4731da4115cfb14f79177884158, haul 0fd2e8eb6af2c8410ee2d8b3bddc774cbb6ba269abfebf77d223e868d64e9c31, acme fb13c9695a4705878030903ceef07b5cc153647672b6b73ae91b0445b408db78, cedar 0383a11b2a24bdb739f495858fd8f0d96ab8107e3ca8c2e0d367077dc12e0c32. So the L0-CORE and L0-API2 merges did not change `m2_trace.rs` again.

## Other hashes

- `tests/m3/m2_operation_stream.json` sha256 bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b: equal to the recorded value.
- `probe_neutral` (`l0sim-fix3-hashes`, exit 0): `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`: both equal.

## Acceptance (`l0sim-fix3-acceptance`, `MC_JOBS=4`, one run)

`scripts/acceptance.sh` exit 0, `source_unchanged` true, 30 tests passed, 0 failed. It passed on the first run; there was no failed step and no rerun.

## NOT TRACED

- A red run of the 35 L0-SIM plants on b7fff19 (see above).
- Whether integration moved again after b215c3a (I fetched once, at the start of this step).
- The three acceptance flakes of repair 2 stay unexplained beyond host load; this run was green.
