---
name: m1-test-zeta
description: M1 test-mode run (2026-10-26) of third-party package zeta on release server and headless client; verified messages and doc gaps
metadata:
  type: project
---

Verified by run on main at bcbc5cd/06eb9f1 (2026-10-26), release binaries:
- A 2-unit package (Circle footprints, Capsule + Cuboid visuals), written from design 4.4 + Appendix A only,
  packs, loads, joins slot 0, and both visuals reach the client. No engine change.
- `--move X,Y` moved BOTH owned units (not one). No doc states which units it selects.
- turts-pack validates nothing; all content errors show first at server start.
- Server messages (exit 1): `<pkg>/<file>: <file>: unknown field `x` at line L column C` (file twice,
  no expected-field list); range errors name file + field_path + limit. No Rust type name seen
  (closes the gate-1 leak concern for unknown field and unknown variant).
- Visual for an undefined unit: accepted silently by pack, server, and client.
- Visual unit-id typo: client fails with `unit `zeta:runner` has 0 visuals`, only in --out JSON
  `rejected_reason`; client exit 0, nothing on stderr; no file named.
- turts_client has no --help; CLAUDE.md points to source `turts_client::options::parse_cli` for hooks.

**Why:** consumer verdict for M1 verify. **How to apply:** at M2 re-run these cases; check closed.
