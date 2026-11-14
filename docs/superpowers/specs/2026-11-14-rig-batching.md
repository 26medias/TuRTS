# Rig batching and skin binding

The engine reduces model-part work while it preserves the authored geometry and animation.
A glTF node's skin binding selects its skin. Joint attributes alone do not select a skin.
Rigid nodes ignore unused joint attributes. The engine changes private mesh copies only.
Batching leaves the package files unchanged. Public API 0.2.10 and wire protocol 6 stay unchanged.
A separate skirmish content fix can correct an authored effect position without changing this contract.

Opaque and masked parts can share a draw when their material, palette, and vertex layouts match.
Transparent parts keep their separate sort positions.
The engine retains all animation nodes and named muzzle anchors.
It keeps every vertex, UV, triangle, material group, animation clock, and blend weight.
It does not lower animation cadence or hide actors to meet a frame budget.

A rigid attachment can join a skin when its path below one joint is static.
Its baked position uses inverse(inverse_bind) * attachment_from_joint.
It then has one full joint weight. The existing skin shader produces the original world position.
Normals and tangents use the corresponding affine transforms.
The engine checks positive scale paths so tangent handedness stays unchanged.
Unsupported layouts, morphs, independent node animation, reflections, unsafe scale curves,
singular matrices, and nonfinite results keep the existing draw path.
An unsuitable optimization must not reject an otherwise accepted package.
Graph walks are bounded. Source meshes and inverse-bind assets remain unchanged.

Stable root and rig rotations do not mark a transform changed.
A changed rotation still applies normally.
Server orders, movement priority, economy, and match outcomes keep their existing rules.

Acceptance checks:

- Native checks preserve positions, normals, tangents, UVs, triangle order, and palette indices.
- A hierarchy comparison matches the shipped joint paths before and after joint motion.
- Tests cover absent skin bindings, animated attachments, reflections, morphs, and unsupported payloads.
- The real browser retains bolt pixels, spark pixels, white flashes, and team-colour return.
- Record 400- and 2,000-unit High/Low measurements with all actors visible and taking damage.
  High stays at or above 30 FPS at 400 units. Keep the existing scene-read protocol explicit.
- One independent review runs code after the build. Fix its findings without another review loop.
- Final scripted cold probes and the actual launcher pass before delivery.

No main merge or push. Every test browser uses a fresh profile and --alsa-output-device=null.
