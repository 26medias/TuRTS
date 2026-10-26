---
name: gaps-mod-manifest-6-5
description: Gaps filed at gate 1 on spec 6.5 (2026-10-26) and the verified 5.3-vs-5.5 manifest field conflict; check whether a later artefact closed them
metadata:
  type: project
---

Verified by grep on 2026-10-26 (spec v0.5, no Cargo.toml, no runtime):
- Spec 5.3 (line 180) and 5.5 (line 194) give two different manifest field lists.
  5.3: identifier, version, engine/protocol version, exact dependencies, content namespace,
  entry points, package sizes, download URLs, content hashes.
  5.5: namespaced identity, version, package kind, engine API compatibility, dependency
  version constraints, exports/entry points, execution context, requested capabilities, license.
  Only "version" and "entry points" match by name. Status: both Baseline/Decided text; not reconciled.
- No public section defines: identifier format, version scheme, hash algorithm, capability names,
  package-kind enum, cosmetic-only flag (5.4 line 187 requires one), error text for incompatible engine.
- Gaps filed: see scratchpad sample-mod-manifest.md of that session; not yet closed by any artefact.

**Why:** a modder cannot write a manifest from the public text alone.
**How to apply:** at the next 5.x or 6.5 gate, re-run the field checklist first and mark closed gaps.
