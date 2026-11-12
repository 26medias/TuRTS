#!/usr/bin/env python3
"""Prepare step and stale check of the browser runs (MP1 Q-PREBUILD).

A prepare step builds everything a browser run needs (server, packer, wasm client, the packed
archive, the row artefacts) into one folder, OUTSIDE the GPU lock. A prepared run builds nothing:
it first calls `check`, which refuses (exit 66) when the folder does not match the present source.

    mc_prepare.py begin --root R --out D --package P [--row F]
    mc_prepare.py seal  --root R --out D --package P [--row F]
    mc_prepare.py check D --root R --package P [--row F] [--need REL ...]

`begin` records the state of the source before the build. `seal` takes the state again, refuses
when it differs (the source changed while the build ran), hashes every artefact, and writes
`manifest.json` last. `check` compares the manifest with the present source. Python 3 standard
library only.
"""
import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True

SCHEMA = "mc-prepared-v1"
STALE = 66  # exit code of a refused prepared folder; the wrapper uses 2, 3, 75 and 124
# What the builds read. A change here can change a binary or the dist.
SOURCE_ROOTS = ["crates", "vendor", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo"]
# What the prepare step RUNS or READS to make an artefact (Q-PREBUILD review, case c9: the KM1 record is
# written by code in mc-browser.sh; the PER1 reference by per1_native_reference.py, which imports
# hd1_reference.py). One edit of one of them gives exit 66 even when HEAD is the same.
#   mc-browser.sh, t7.sh, m2-browser.sh, m3-browser.sh: the prepare branches and their heredocs
#   mc-pixels.sh: the --prepare form calls mc-browser.sh
#   mc_prepare.py, mc_prepare_lib.sh: this tool and the lock and begin step
#   mc_browser_params.py: row checks that the prepare branch runs
#   per1_native_reference.py, hd1_reference.py: the PER1 native reference (prepare) and its imports
#   tests/browser/package.json and package-lock.json: what `npm ci` installs
PREPARE_TIME_FILES = [
    "scripts/mc-browser.sh", "scripts/mc-pixels.sh", "scripts/t7.sh", "scripts/m2-browser.sh", "scripts/m3-browser.sh",
    "scripts/mc_prepare.py", "scripts/mc_prepare_lib.sh", "scripts/mc_browser_params.py",
    "scripts/per1_native_reference.py", "scripts/hd1_reference.py",
    "tests/browser/package.json", "tests/browser/package-lock.json",
]
BUILD_INPUTS = SOURCE_ROOTS + PREPARE_TIME_FILES
# The build environment that can change an output. An unset variable is a value (None).
ENV_NAMES = ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"]
ENV_PREFIX = "CARGO_PROFILE_"
TOOLS = {"rustc": ("rustc", "--version"), "cargo": ("cargo", "--version"), "trunk": ("trunk", "--version")}
# Files and folders of the prepared folder that `seal` hashes.
ARTEFACT_ROOTS = ["bin", "dist", "pack", "native-reference", "effective-bindings.json", "km1-reserved-pack.json"]


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def digest_path(path):
    """The digest of one file; a symlink gives its target; a missing path gives MISSING."""
    path = Path(path)
    if path.is_symlink():
        return "symlink:" + os.readlink(path)
    if path.is_file():
        return sha256_bytes(path.read_bytes())
    return "MISSING"


def git(root, *args):
    return subprocess.run(["git", "-C", str(root), *args], check=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE).stdout


def source_files(root):
    """Digest of each tracked or untracked (not ignored) file that a build reads."""
    raw = git(root, "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", *BUILD_INPUTS)
    names = sorted({entry.decode() for entry in raw.split(b"\0") if entry})
    return {name: digest_path(Path(root) / name) for name in names}


def package_files(root, package):
    """Digest of each file of the package folder. A missing folder is an error."""
    base = Path(root) / package
    if not base.is_dir():
        raise SystemExit(f"mc_prepare: package folder not found: {package}")
    found = {}
    for current, dirs, files in os.walk(base):
        dirs.sort()
        for name in sorted(files):
            file = Path(current) / name
            found[file.relative_to(base).as_posix()] = digest_path(file)
    return found


def map_sha(mapping):
    return sha256_bytes(json.dumps(mapping, sort_keys=True, separators=(",", ":")).encode())


def row_info(root, row_file):
    """The row facts that decide which artefacts a run needs. None without a row file."""
    if not row_file:
        return None
    path = Path(row_file)
    data = json.loads(path.read_bytes())
    needs = []
    if data.get("effective_bindings") is True:
        needs.append("effective-bindings.json")
    if data.get("row") == "KM1":
        needs.append("km1-reserved-pack.json")
    if data.get("row") == "PER1":
        needs.append("native-reference/native-path-audit.json")
    return {"row": data.get("row"), "sha256": sha256_bytes(path.read_bytes()), "needs": needs}


def state(root, package):
    root = Path(root)
    inputs = source_files(root)
    package_map = package_files(root, package)
    status = git(root, "status", "--porcelain=v1", "--", *BUILD_INPUTS, package).decode()
    return {
        "head": git(root, "rev-parse", "HEAD").decode().strip(),
        "dirty": bool(status.strip()),
        "source_sha256": map_sha(inputs),
        "source_files": inputs,
        "package": package,
        "package_sha256": map_sha(package_map),
    }


def artefact_hashes(out):
    out = Path(out)
    found = {}
    for name in ARTEFACT_ROOTS:
        path = out / name
        if path.is_dir():
            for current, dirs, files in os.walk(path):
                dirs.sort()
                for link in [d for d in dirs if (Path(current) / d).is_symlink()]:
                    full = Path(current) / link  # a symlink to a folder is not followed: it is one entry
                    found[full.relative_to(out).as_posix()] = digest_path(full)
                for file in sorted(files):
                    full = Path(current) / file
                    found[full.relative_to(out).as_posix()] = digest_path(full)
        elif path.exists():
            found[name] = digest_path(path)
    return found


def tool_versions():
    return {name: tool_version(*argv) for name, argv in TOOLS.items()}


def build_env(extra_names=()):
    names = set(ENV_NAMES) | {name for name in os.environ if name.startswith(ENV_PREFIX)} | set(extra_names)
    return {name: os.environ.get(name) for name in sorted(names)}


def tool_version(*argv):
    try:
        done = subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, check=False)
        return done.stdout.strip().replace("\n", " | ")
    except OSError as error:
        return f"unavailable: {error}"


def cmd_begin(args):
    out = Path(args.out)
    (out / "manifest.json").unlink(missing_ok=True)
    (out / ".begin.json").write_text(json.dumps(state(args.root, args.package), sort_keys=True) + "\n")


def cmd_seal(args):
    out = Path(args.out)
    begin_path = out / ".begin.json"
    if not begin_path.is_file():
        raise SystemExit("mc_prepare: seal without begin")
    begin = json.loads(begin_path.read_text())
    now = state(args.root, args.package)
    for key in ("head", "source_sha256", "package_sha256"):
        if begin[key] != now[key]:
            print(f"mc_prepare: the source changed while the prepare step ran ({key}: {begin[key]} then {now[key]})",
                  file=sys.stderr)
            raise SystemExit(STALE)
    artefacts = artefact_hashes(out)
    row = row_info(args.root, args.row)
    if row:
        for need in row["needs"]:
            if need not in artefacts:
                print(f"mc_prepare: row {row['row']} needs {need}, and the prepare step did not make it", file=sys.stderr)
                raise SystemExit(1)
    manifest = {
        "schema": SCHEMA,
        "prepared_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "head": now["head"],
        "dirty": now["dirty"],
        "source_sha256": now["source_sha256"],
        "source_files": now["source_files"],
        "package": args.package,
        "package_sha256": now["package_sha256"],
        "row": row,
        "artefacts": artefacts,
        "tools": tool_versions(),
        "build_env": build_env(),
        "cargo_target_dir": os.environ.get("CARGO_TARGET_DIR"),
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    begin_path.unlink()
    print(f"mc_prepare: sealed {out} head={now['head'][:12]} dirty={str(now['dirty']).lower()} "
          f"source={now['source_sha256'][:12]} package={now['package_sha256'][:12]} artefacts={len(artefacts)}")


def stale_reasons(prepared, root, package, row_file, need):
    """Each reason why the folder must not be used. An empty list means it matches the source."""
    manifest_path = Path(prepared) / "manifest.json"
    if not manifest_path.is_file():
        return [f"no manifest.json in {prepared} (the prepare step did not finish or never ran)"]
    try:
        manifest = json.loads(manifest_path.read_text())
    except json.JSONDecodeError as error:
        return [f"manifest.json is not JSON: {error}"]
    if manifest.get("schema") != SCHEMA:
        return [f"manifest schema is {manifest.get('schema')!r}, not {SCHEMA!r}"]
    reasons = []
    if manifest["package"] != package:
        reasons.append(f"package differs (prepared {manifest['package']}, run {package})")
    now = state(root, package)
    if manifest["head"] != now["head"]:
        reasons.append(f"HEAD differs (prepared {manifest['head'][:12]}, now {now['head'][:12]})")
    if manifest["source_sha256"] != now["source_sha256"]:
        before, after = manifest["source_files"], now["source_files"]
        changed = sorted(name for name in set(before) | set(after) if before.get(name) != after.get(name))
        shown = ", ".join(changed[:5]) + (f" and {len(changed) - 5} more" if len(changed) > 5 else "")
        reasons.append(f"build source differs ({len(changed)} file(s): {shown})")
    if manifest["package_sha256"] != now["package_sha256"]:
        reasons.append(f"package content differs ({package})")
    row = row_info(root, row_file)
    needs = list(need)
    if row:
        needs += row["needs"]
        if row["needs"] and (not manifest.get("row") or manifest["row"]["sha256"] != row["sha256"]):
            reasons.append(f"row {row['row']} needs artefacts that were made for another row file")
    for name, now_version in tool_versions().items():
        if manifest["tools"].get(name) != now_version:
            reasons.append(f"tool {name} changed (prepared {manifest['tools'].get(name)!r}, now {now_version!r})")
    prepared_env = manifest["build_env"]
    for name, now_value in build_env(prepared_env).items():
        if prepared_env.get(name) != now_value:
            reasons.append(f"build environment differs ({name}: prepared {prepared_env.get(name)!r}, now {now_value!r})")
    artefacts = manifest["artefacts"]
    for rel in needs:
        if rel not in artefacts:
            reasons.append(f"artefact {rel} is not in the manifest")
    present = artefact_hashes(prepared)
    for rel, expected in sorted(artefacts.items()):
        if rel not in present:
            reasons.append(f"artefact {rel} is missing")
        elif present[rel] != expected:
            reasons.append(f"artefact {rel} changed after the prepare step")
    for rel in sorted(set(present) - set(artefacts)):
        reasons.append(f"unexpected file {rel} in the prepared folder (the prepare step did not make it)")
    return reasons


def cmd_check(args):
    reasons = stale_reasons(args.prepared, args.root, args.package, args.row, args.need)
    if reasons:
        for reason in reasons:
            print(f"mc_prepare: STALE {args.prepared}: {reason}", file=sys.stderr)
        print("mc_prepare: refused. A prepared run builds nothing. Run the prepare step again.", file=sys.stderr)
        return STALE
    manifest = json.loads((Path(args.prepared) / "manifest.json").read_text())
    print(f"mc_prepare: OK {args.prepared} head={manifest['head'][:12]} dirty={str(manifest['dirty']).lower()} "
          f"source={manifest['source_sha256'][:12]} package={manifest['package_sha256'][:12]} "
          f"prepared_utc={manifest['prepared_utc']}")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("begin", "seal"):
        item = sub.add_parser(name)
        item.add_argument("--root", required=True)
        item.add_argument("--out", required=True)
        item.add_argument("--package", required=True)
        item.add_argument("--row")
    item = sub.add_parser("check")
    item.add_argument("prepared")
    item.add_argument("--root", required=True)
    item.add_argument("--package", required=True)
    item.add_argument("--row")
    item.add_argument("--need", action="append", default=[])
    args = parser.parse_args(argv)
    if args.command == "check":
        return cmd_check(args)
    {"begin": cmd_begin, "seal": cmd_seal}[args.command](args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
