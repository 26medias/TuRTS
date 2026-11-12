# Prepared browser runs (Q-PREBUILD)

A browser run held the GPU lock while cargo and trunk built. A prepared run builds first, outside the
lock, and the held command only starts the server, serves the page and drives the browser.

## Use

1. Prepare (no lock): `bash scripts/mc-pixels.sh --prepare <folder> tests/browser/rows/<ROW>.json`.
   Also `scripts/mc-browser.sh <package> --prepare <folder> [--params <row>]`, `scripts/t7.sh --prepare <folder>`,
   `scripts/m3-browser.sh --prepare <folder>`, `scripts/m2-browser.sh --prepare <folder>`.
2. Run (the held command): `bash scripts/mc-pixels.sh --prepared <folder> tests/browser/rows/<ROW>.json`
   (and the same flag for the other scripts). It builds nothing.
3. With no flag each script builds inside the run, as before (CI does this).

A prepared run refuses with exit 66 when the folder does not match the present source. Exit 66 is its own
code: the wrapper uses 2, 3, 75 and 124. One prepare step at a time can write one folder (`<folder>.lock`);
a second one exits 2.

## What makes a folder stale (`scripts/mc_prepare.py check`)

HEAD (strict: any new commit stales the folder; it is the only guard on committed edits of classes that
are not hashed), the dirty state, and the content hash of:

- `crates/`, `vendor/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/`;
- the package folder of the run;
- every script that the prepare step runs or reads: `scripts/mc-browser.sh`, `mc-pixels.sh`, `t7.sh`,
  `m2-browser.sh`, `m3-browser.sh`, `mc_prepare.py`, `mc_prepare_lib.sh`, `mc_browser_params.py`,
  `per1_native_reference.py`, `hd1_reference.py`; and `tests/browser/package.json`, `package-lock.json`;
- the row file, for a row that needs a prepared artefact (effective bindings, KM1, PER1);
- every artefact in the folder: a changed, a missing and an extra file are all refused.

It also compares `rustc`, `cargo` and `trunk` versions and `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS` and
every `CARGO_PROFILE_*` variable (an unset variable is a value). `trunk` downloads its own wasm-bindgen
and wasm-opt: `crates/turts_client_web/Trunk.toml` pins them and is hashed.

A driver or oracle file (`tests/browser/*.js`) is not hashed: it is read when the run starts.

## Hold limits for the lock wrapper (a model, to verify on the first prepared headed runs)

The wrapper limits are interim (`mc_headed_v6.sh` header). With the builds outside the lock, the reviewer of
Q-PREBUILD proposes: table mode 600 s for a row, and 1,700 s for PT1 only; pre-merge mode 900 s for a session
without PT1, and 1,800 s with PT1. This is a model. Measure the first prepared headed runs, then set them.

## The smallest form for a pixel plant

One scratch checkout for each plant, with the plant left as a dirty edit. Prepare there only when the plant
touches a hashed root (a crate, a package, a prepare-time script). Then one headed session runs each
`mc-pixels.sh --prepared` with no build inside the lock. A plant in a driver or an oracle file needs no
prepare: prepare once in that checkout before the plant, and the plant does not stale the folder.

## Not covered

The M2 fault runner (`tests/m2/refutations.py`) still builds inside its held command: it needs a prepare
after each plant, the lock taken for each browser phase, and a no-git fallback in `mc_prepare.py` (its
scratch copy has no `.git`).
