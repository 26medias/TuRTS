---
name: spec-register-layout
description: Verified line anchors in RTS-Engine-Project-Specifications.md v0.5/v0.6 for the register, modules, 5.3, 5.5, 6.5, invariants, revision history, and open items
metadata:
  type: project
---

Verified 2026-10-26 (commit 38a947c last touched the spec): register table lines 22-82 (D01 L24 ... D59 L82); 4.1 modules L112-125; 4.3 L142-158; 5.2 L170; 5.3 L176 (D20 text at L182); 5.5 L190; 5.7 L202; 6.4 L230 (layer table L236-243); 6.5 L249-259; 6.6 L261; 6.13 L355; invariants L513-529; Section 15 L590-605 (extra open items at L605); M1 row L571. No Cargo.toml exists.

v0.6 (working tree, 2026-10-26): register L22-82 byte-identical to v0.5 (`cmp`); 5.3 manifest paragraph L182; 5.5 manifest field list L194; 6.5 L249-259; invariants L510-529; Section 15 L588-603; revision history L687-695 (0.6 entry sits at L693, before 0.5 at L695). 4.1 still names no module for client-side mod acquisition; agent design table (docs/superpowers/specs/2026-10-25-studio-agent-team-design.md:142) gives 6.5 to mod-runtime-engineer with web-platform-engineer second.

**Why:** Gate reviews need `path:line` cites for every claim; these anchors save a re-scan.
**How to apply:** Re-check with grep before citing; the spec can move when it is edited.
