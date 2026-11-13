---
name: mc-ci3-offline-fetch-2026-11-12
description: Hosted run 43195887161 browser m2 and 10 refutation legs red from CARGO_NET_OFFLINE with no fetch (not GPU); repair 8fb2aa4 on codex/mc-fix-ci3
metadata:
  type: project
---

Cause (verified): scripts/m2-browser.sh m2_build (:24-30) builds with CARGO_NET_OFFLINE=true. A fresh
hosted runner has only trunk's crates (cargo install trunk), so cargo says "no matching package named
`ron` found" (required by turts_checks). Run 43174011989 had the same red in m2 (so no hosted run ever
reached a browser case for m2 or the refutation legs). t7-m3 is green because scripts/t7.sh:24 builds
online before scripts/m3-browser.sh:56 builds offline. Refutation legs show only "M2_CASE_INFRA_FAIL ...
runner exited before a browser case summary" (m2-browser.sh:165 trap); the cargo text is in the artifact
server-build.log.

Repro: clean CARGO_HOME + `cargo install --locked trunk --version 0.22.0` killed after downloads
(timeout 100) gives the hosted state; offline build red (same text); `cargo fetch --locked` then offline
build green; online build first then offline build also green. Wrapper receipts under
target/mc-evidence/ci3-*.

Repair: one ci.yml step `cargo fetch --locked` in the browser job. ci.yml `on:` = push main + pull_request,
so pushing codex/mc-fix-ci3 starts no run without a PR.

Open: m2 two-window and refutation browser cases never ran on hosted software WebGPU. t7-m3 PASS
m3-oracle.js there (log line 3363 of job 131106617244).
