#!/usr/bin/env python3
"""Create or validate a byte-bound native HD1 report for browser parity."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(root), *args]).decode().strip()


def source_identity(root: Path) -> dict[str, object]:
    paths = git(root, "ls-files", "--cached", "--others", "--exclude-standard", "-z").split("\0")
    files = []
    for relative in sorted(set(path for path in paths if path)):
        path = root / relative
        if any(part.lower() == ".claude" for part in Path(relative).parts) or Path(relative).name.lower() == "claude.md":
            continue
        if not path.is_file():
            continue
        files.append({"path": relative, "sha256": digest(path.read_bytes())})
    return {
        "head": git(root, "rev-parse", "HEAD"),
        "branch": git(root, "branch", "--show-current"),
        "status_porcelain_v1": git(root, "status", "--porcelain=v1", "-uall"),
        "files": files,
        "manifest_sha256": digest(json.dumps(files, sort_keys=True, separators=(",", ":")).encode()),
    }


def tree_hash(path: Path) -> str:
    files = []
    for item in sorted(path.rglob("*")):
        if item.is_file():
            files.append({"path": item.relative_to(path).as_posix(), "sha256": digest(item.read_bytes())})
    return digest(json.dumps(files, sort_keys=True, separators=(",", ":")).encode())


def read_json(path: Path) -> object:
    return json.loads(path.read_text())


def runtime_manifest(run_dir: Path) -> list[dict[str, str]]:
    client = read_json(run_dir / "client-argv.json")
    server = read_json(run_dir / "server-argv.json")
    test_exe = (run_dir / "test-executable.txt").read_text().strip()
    paths = {str(client["executable"]), str(server["executable"]), test_exe}
    entries = []
    for value in sorted(paths):
        path = Path(value).resolve(strict=True)
        entries.append({"path": str(path), "sha256": digest(path.read_bytes())})
    return entries


def execution_evidence(root: Path, run_dir: Path, evidence_dir: Path) -> dict[str, object]:
    before = read_json(evidence_dir / "source-before.json")
    after = read_json(evidence_dir / "source-after.json")
    current = source_identity(root)
    if before != after or after != current:
        raise ValueError("native execution source before/after snapshots differ from current source")
    launcher = read_json(evidence_dir / "launcher.json")
    expected_argv = ["cargo", "test", "--release", "-p", "turts_client", "--test", "hd1", "headless_actions_drive_core_and_report_per_unit_results_on_muster", "--locked", "--", "--ignored", "--exact", "--test-threads=1"]
    if launcher.get("argv") != expected_argv:
        raise ValueError("native launcher argv is not the exact named release process test")
    expected_server_build = ["cargo", "build", "--release", "-p", "turts_server", "--locked"]
    if launcher.get("server_build_argv") != expected_server_build:
        raise ValueError("native launcher did not record the locked release server build")
    env = launcher.get("environment")
    expected_target = str((root / "target/mc-t10").resolve())
    if not isinstance(env, dict) or env.get("CARGO_TARGET_DIR") != expected_target:
        raise ValueError("native launcher did not use the bound mc-t10 target directory")
    if launcher.get("working_directory") != str(root.resolve()):
        raise ValueError("native launcher working directory is not the repository lane")
    if any(env.get(key) != value for key, value in {
        "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_INCREMENTAL": "0", "CARGO_BUILD_JOBS": "4",
    }.items()):
        raise ValueError("native launcher build environment does not match the accepted profile")
    server_build_status = (evidence_dir / "server-build-status.txt").read_text().strip()
    server_build_log = (evidence_dir / "server-build.log").read_bytes()
    if server_build_status != "exit=0" or "Finished `release` profile" not in server_build_log.decode(errors="replace"):
        raise ValueError("locked release server build did not complete successfully")
    status = (evidence_dir / "native-process-status.txt").read_text().strip()
    if status != "exit=0":
        raise ValueError("native process test did not record exit=0")
    log_bytes = (evidence_dir / "native-process.log").read_bytes()
    log = log_bytes.decode(errors="replace")
    if "test headless_actions_drive_core_and_report_per_unit_results_on_muster ... ok" not in log or "test result: ok. 1 passed; 0 failed; 0 ignored;" not in log:
        raise ValueError("native process raw log does not show the named test passing exactly once")
    client = read_json(run_dir / "client-argv.json")
    client_args = client.get("args")
    actions_path = str((root / "tests/browser/fixtures/hd1-actions.json").resolve())
    if not isinstance(client_args, list) or "--headless" not in client_args or "--actions" not in client_args:
        raise ValueError("actual client argv does not include headless --actions")
    index = client_args.index("--actions")
    if index + 1 >= len(client_args) or str(Path(client_args[index + 1]).resolve()) != actions_path:
        raise ValueError("actual client argv did not use the bound HD1 action fixture")
    return {
        "launcher": launcher,
        "source_before": before,
        "source_after": after,
        "native_process_status": status,
        "native_process_log_sha256": digest(log_bytes),
        "server_build_status": server_build_status,
        "server_build_log_sha256": digest(server_build_log),
    }


def binding(root: Path, run_dir: Path, evidence_dir: Path) -> dict[str, object]:
    actions = root / "tests/browser/fixtures/hd1-actions.json"
    package = root / "tests/browser/fixtures/hd1/muster"
    report = run_dir / "out.json"
    test_executable = Path((run_dir / "test-executable.txt").read_text().strip()).resolve(strict=True)
    profile = test_executable.parent.parent.name
    if profile not in {"debug", "release"}:
        raise ValueError(f"cannot determine Cargo profile from test executable path: {test_executable}")
    evidence = execution_evidence(root, run_dir, evidence_dir)
    return {
        "source": evidence["source_after"],
        "execution_evidence": evidence,
        "actions_file": "tests/browser/fixtures/hd1-actions.json",
        "actions_sha256": digest(actions.read_bytes()),
        "package": "tests/browser/fixtures/hd1/muster",
        "package_tree_sha256": tree_hash(package),
        "native_report_sha256": digest(report.read_bytes()),
        "native_argv": {
            "client": read_json(run_dir / "client-argv.json"),
            "server": read_json(run_dir / "server-argv.json"),
            "test_executable": (run_dir / "test-executable.txt").read_text().strip(),
        },
        "environment": read_json(run_dir / "execution-env.json"),
        "runtime_manifest": runtime_manifest(run_dir),
        "profile": {
            "cargo_profile": profile,
            "cargo_version": subprocess.check_output(["cargo", "--version"]).decode().strip(),
            "rustc_version": subprocess.check_output(["rustc", "-Vv"]).decode().strip(),
        },
    }


def save(root: Path, run_dir: Path, destination: Path, evidence_dir: Path) -> None:
    data = binding(root, run_dir, evidence_dir)
    destination.mkdir(parents=True, exist_ok=False)
    shutil.copy2(run_dir / "out.json", destination / "native-report.json")
    for name in ("client-argv.json", "server-argv.json", "execution-env.json", "test-executable.txt"):
        shutil.copy2(run_dir / name, destination / name)
    for name in ("native-process.log", "native-process-status.txt", "server-build.log", "server-build-status.txt", "launcher.json", "source-before.json", "source-after.json"):
        shutil.copy2(evidence_dir / name, destination / name)
    data["native_report_sha256"] = digest((destination / "native-report.json").read_bytes())
    data["binding_sha256"] = digest(json.dumps(data, sort_keys=True, separators=(",", ":")).encode())
    (destination / "binding.json").write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def validate(root: Path, reference: Path) -> None:
    data = read_json(reference / "binding.json")
    if not isinstance(data, dict):
        raise ValueError("native HD1 binding must be a JSON object")
    expected_hash = data.get("binding_sha256")
    unsigned = dict(data)
    unsigned.pop("binding_sha256", None)
    actual_hash = digest(json.dumps(unsigned, sort_keys=True, separators=(",", ":")).encode())
    if expected_hash != actual_hash:
        raise ValueError("native HD1 binding receipt has been modified")
    evidence = data.get("execution_evidence")
    if not isinstance(evidence, dict):
        raise ValueError("native HD1 binding lacks execution provenance")
    if (reference / "native-process-status.txt").read_text().strip() != evidence.get("native_process_status"):
        raise ValueError("native HD1 process status artifact differs from the bound receipt")
    log_bytes = (reference / "native-process.log").read_bytes()
    if digest(log_bytes) != evidence.get("native_process_log_sha256"):
        raise ValueError("native HD1 process log is missing or differs from the bound receipt")
    build_log = (reference / "server-build.log").read_bytes()
    if (reference / "server-build-status.txt").read_text().strip() != evidence.get("server_build_status") or digest(build_log) != evidence.get("server_build_log_sha256"):
        raise ValueError("native HD1 locked server build evidence differs from the bound receipt")
    if read_json(reference / "source-before.json") != evidence.get("source_before") or read_json(reference / "source-after.json") != evidence.get("source_after"):
        raise ValueError("native HD1 source snapshots differ from the bound receipt")
    if evidence.get("source_before") != evidence.get("source_after"):
        raise ValueError("native HD1 source changed during its execution")
    if read_json(reference / "launcher.json") != evidence.get("launcher"):
        raise ValueError("native HD1 launcher invocation differs from the bound receipt")
    report = reference / "native-report.json"
    if not report.is_file() or digest(report.read_bytes()) != data.get("native_report_sha256"):
        raise ValueError("native HD1 report is missing or its hash differs from the binding")
    actions = root / "tests/browser/fixtures/hd1-actions.json"
    if digest(actions.read_bytes()) != data.get("actions_sha256"):
        raise ValueError("native HD1 actions fixture is stale")
    package = root / "tests/browser/fixtures/hd1/muster"
    if tree_hash(package) != data.get("package_tree_sha256"):
        raise ValueError("native HD1 controlled package is stale")
    if source_identity(root) != data.get("source"):
        raise ValueError("native HD1 source HEAD or dirty source manifest is stale")
    run_dir = Path(os.environ.get("CARGO_TARGET_DIR", root / "target/mc-t10")) / "tmp/hd1"
    if runtime_manifest(run_dir) != data.get("runtime_manifest"):
        raise ValueError("native HD1 runtime byte manifest is stale")
    print("native HD1 reference binding is current")


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    create = sub.add_parser("create")
    create.add_argument("repo", type=Path)
    create.add_argument("run_dir", type=Path)
    create.add_argument("destination", type=Path)
    create.add_argument("evidence", type=Path)
    check = sub.add_parser("validate")
    check.add_argument("repo", type=Path)
    check.add_argument("reference", type=Path)
    snapshot = sub.add_parser("snapshot")
    snapshot.add_argument("repo", type=Path)
    snapshot.add_argument("destination", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "create":
            save(args.repo.resolve(), args.run_dir.resolve(), args.destination.resolve(), args.evidence.resolve())
        elif args.command == "snapshot":
            args.destination.write_text(json.dumps(source_identity(args.repo.resolve()), indent=2, sort_keys=True) + "\n")
        else:
            validate(args.repo.resolve(), args.reference.resolve())
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"HD1 native reference setup error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
