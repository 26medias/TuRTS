---
type: decision
date: 2026-11-11
status: open
tags: [turts, mc, ci, ceo]
---

# MC: the hosted CI runner has no GPU, and the M2 and M3 browser proofs cannot pass there

## Facts (RUN134, web-platform-engineer; Root read the hosted logs)

- The MC done condition says: the `check`, `agents` and `browser` jobs are green on the integration branch (brief section 6).
- The hosted runner has 4 vCPU and no GPU. Chrome renders with a software driver.
- T7 on hosted: repaired. The cause was a blank WebGPU canvas with ANGLE on Vulkan on the software driver. With ANGLE on Mesa GL the canvas renders and T7 passes locally under the same condition (4 cores, software rendering).
- The M2 browser proof under software rendering: about 1 frame for each second in each window; the two windows render one after the other, about 22 ticks apart, and show 0 shared ticks in 180 s. No wait can repair this.
- The M3 browser controls under software rendering: the Chrome GPU process grows to about 1.1 GB and stops with exit code 133 after 250 to 310 s. The cause (a client GPU leak, or the software driver) is NOT TRACED.
- History: the hosted `browser` job has failed at "M2 browser proof" on main since before QA0 (brief section 6). So this limit is older than MC; MC made the cause visible.
- A product fact behind it: the browser client runs at about 1 to 4 frames for each second without a GPU. A player with no GPU, or with a blocklisted one, cannot play. The WebGL fallback is an open decision in the specification.

## Options

- a. A GPU runner for the hosted browser jobs: a GitHub GPU runner (paid), or a self-hosted runner on GCP (D57 to D59; paid for each run), or a self-hosted runner on the development machine (no cost, but a self-hosted runner on a public repository can run code from a pull request of a fork, so it needs a restriction to this repository's own branches).
- b. Hosted CI runs T7 and every native job. The M2 and M3 browser proofs and the eight browser faults stay a local gate on the real GPU, with receipts, and the done condition says so. This is what happened in practice since M2.
- c. Make the client fast and stable on software rendering (and decide the WebGL fallback). This is product work of unknown size; it does not fit in MC.

## Recommendation (Root, with the web-platform-engineer result; the qa-lead review RUN142 gives the trade-offs)

Option b for MC, with the honest wording of the done condition: "hosted `check` and `agents` green; hosted `browser` green for T7; the M2 and M3 browser proofs and the browser fault set green on the local GPU gate with receipts". Then a for the long term, when the cost is known, and c as a product question for a later milestone (it is the player without a GPU). The CEO decides, because it changes the MC done condition.

## Correction, 2026-11-12 (orchestrator)

The premise of this decision was wrong for the 11 red browser jobs of hosted runs 43174011989 and 43195887161. Their cause was not the missing GPU. Each job failed at its first cargo build, before a browser started: the browser scripts build with `CARGO_NET_OFFLINE=true`, and the job had no step that fetched the crates (`no matching package named ron found`). Evidence: the job logs and the `server-build.log` artifacts; a local reproduction with a clean cargo home (red, exit 101; green after `cargo fetch --locked`).

Repair: one step `cargo fetch --locked` in the `browser` job (commit 8fb2aa4, merged at 851b39d). No hosted run has reached a browser case of the m2 leg or of the refutation legs before this repair, so the question "do the two-window cases need a GPU" has no hosted evidence yet. The single-window T7 and M3 oracles are green on the software renderer.

This decision stays open, but do not decide it now. The hosted run on 851b39d gives the first real evidence. The orchestrator reports the result and, only if a GPU-related red remains, brings the options back.

## Evidence and ruling, 2026-11-12 (orchestrator; hosted run 43214215645 on 851b39d)

After the fetch repair, 15 of 21 jobs are green on the hosted runner, with 5 of the 10 browser refutation jobs. The 6 red jobs fail in the baseline case, in two families (diagnosis: `target/mc-evidence/reviews/ci4-diagnosis.md`):

- **Economy family (4 jobs).** The test steps run in wall-clock time. On the software renderer one screenshot costs about 3.3 s, so the Cancel click lands after the 5 s (base) or 6 s (cedar) recipe has ended. The cause is the test timing, not a product defect and not a hard need of a GPU.
- **Convergence family (2 jobs).** Each window shows a new tick about every 28 ticks (about 1 frame in 1.4 s). The two windows are out of phase and never show the same tick, so the oracle has 0 shared ticks. A longer limit cannot help. This family needs a real GPU.

Ruling (engineering, no money): the economy family gets a longer recipe in the packed copy on the hosted runner (the same method as `T7_SCOUT_SPEED`, already in use); the convergence family runs in the local gate on the GPU machine with a recorded receipt, and its hosted job says so plainly (not silently skipped). The paid options stay open for the CEO and are not needed for MC: a GitHub GPU runner (`linux_4_core_gpu`, $0.052 each minute, needs an organization on a paid plan) or a self-hosted runner on this machine (no cost; GitHub advises against it for a public repository).
