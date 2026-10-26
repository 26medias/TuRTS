#!/usr/bin/env bash
# System packages that need root. Run once: sudo bash scripts/setup-sudo.sh
# Everything else (rustup target, cargo tools) installs without sudo; see scripts/setup-dev.sh.
set -euo pipefail
apt-get update
# perf for linux-perf and cargo-flamegraph; heaptrack for heap profiles; valgrind for DHAT and iai-callgrind.
apt-get install -y linux-tools-common "linux-tools-$(uname -r)" heaptrack valgrind
# Let non-root users record perf samples with call graphs.
sysctl -w kernel.perf_event_paranoid=1
echo 'kernel.perf_event_paranoid = 1' > /etc/sysctl.d/60-perf.conf
echo "done"
