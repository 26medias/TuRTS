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
