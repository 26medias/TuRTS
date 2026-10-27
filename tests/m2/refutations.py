#!/usr/bin/env python3
"""Run exact source mutations in isolated Task 14/15 scratch copies."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "tests/m2/refutations.json"
COPY_IGNORES = (
    ".git", "target", ".superpowers", ".claude", ".codex", ".agents",
    "__pycache__", "node_modules", "dist",
)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command, cwd, build_dir):
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(build_dir)
    env["CARGO_BUILD_JOBS"] = "2"
    env["RUST_BACKTRACE"] = "0"
    return subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )


def selected_count(output):
    match = re.search(r"running (\d+) tests?", output)
    return int(match.group(1)) if match else 0


def require_selected_count(output, expected):
    actual = selected_count(output)
    if actual != expected:
        raise ValueError(f"expected exactly {expected} selected test(s), found {actual}")
    return actual


def select_cases(cases, requested):
    by_id = {case["id"]: case for case in cases}
    if len(by_id) != len(cases):
        raise ValueError("registry contains duplicate case IDs")
    if requested == "all":
        selected = list(cases)
    elif requested == "product":
        selected = [case for case in cases if case.get("kind") == "product"]
    elif requested in {"E7", "E8", "E9"}:
        selected = [case for case in cases if case["id"].startswith(requested + "-")]
        require_case_prefix(selected, requested)
    elif "," in requested:
        wanted = [item.strip() for item in requested.split(",")]
        if not all(wanted) or len(wanted) != len(set(wanted)):
            raise ValueError("case list contains an empty or duplicate case ID")
        unknown = [case_id for case_id in wanted if case_id not in by_id]
        if unknown:
            raise ValueError(f"unknown refutation case(s): {', '.join(unknown)}")
        wanted_set = set(wanted)
        selected = [case for case in cases if case["id"] in wanted_set]
    else:
        selected = [case for case in cases if case["id"] == requested]
        if not selected:
            raise ValueError(f"unknown refutation case: {requested}")
    if not selected:
        raise ValueError("no refutation cases selected")
    return selected


def require_case_prefix(cases, prefix):
    if not cases:
        raise ValueError(f"no refutation cases selected for prefix {prefix}")
    if any(not case["id"].startswith(prefix + "-") for case in cases):
        raise ValueError(f"expected case prefix {prefix} for every selected case")
    return cases


def create_run_workspace(evidence_root, build_root=None):
    evidence_root = Path(evidence_root)
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ") + f"-{os.getpid()}-{uuid.uuid4().hex[:8]}"
    run_root = evidence_root / run_id
    run_root.mkdir(parents=True, exist_ok=False)
    (run_root / "scratch").mkdir()
    (run_root / "cases").mkdir()
    actual_build_root = Path(build_root) / run_id if build_root else run_root / "build"
    actual_build_root.mkdir(parents=True, exist_ok=False)
    # Keep the selected cache root beside evidence as a pointer; cache contents stay isolated.
    (run_root / "build-root.txt").write_text(str(actual_build_root) + "\n")
    return run_root


def copy_source_tree(source, destination):
    shutil.copytree(source, destination, ignore=shutil.ignore_patterns(*COPY_IGNORES))


def pre_commands(case):
    commands = case.get("pre_commands")
    if commands is None:
        command = case.get("pre_command")
        commands = [command] if command else []
    if not isinstance(commands, list) or any(
        not isinstance(command, list) or not command for command in commands
    ):
        raise ValueError(f'{case["id"]}: pre_commands must be an ordered list of commands')
    return commands


def write_case_evidence(evidence_dir, case_id, baseline_hash, restored_hash, logs, status="passed", planted_hash=None, detail=None):
    evidence_dir = Path(evidence_dir)
    evidence_dir.mkdir(parents=True, exist_ok=True)
    for name, output in logs.items():
        (evidence_dir / name).write_text(output)
    manifest = {
        "id": case_id,
        "status": status,
        "baseline_sha256": baseline_hash,
        "planted_sha256": planted_hash,
        "restored_sha256": restored_hash,
        "detail": detail,
        "logs": sorted(logs),
    }
    (evidence_dir / "case.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


def run_case(case, scratch, build_dir, evidence_dir):
    def execute(phase, logs):
        for index, command in enumerate(pre_commands(case), 1):
            result = run(command, scratch, build_dir)
            logs[f"{phase}.precommand-{index}.log"] = result.stdout
            write_case_evidence(
                evidence_dir, case["id"], baseline_hash, restored_hash, logs,
                "running", planted_hash, f"{phase} prerequisite {index}",
            )
            if result.returncode != 0:
                raise RuntimeError(
                    f'{case["id"]}: prerequisite {index} failed in {phase}\n{result.stdout[-5000:]}'
                )
        result = run(case["command"], scratch, build_dir)
        logs[f"{phase}.log"] = result.stdout
        write_case_evidence(
            evidence_dir, case["id"], baseline_hash, restored_hash, logs,
            "running", planted_hash, phase,
        )
        return result

    path = scratch / case["plant_path"]
    original = path.read_text()
    anchor = case["anchor"]
    if original.count(anchor) != 1:
        raise RuntimeError(f'{case["id"]}: anchor occurs {original.count(anchor)} times, expected exactly one')
    baseline_hash = digest(path)
    planted_hash = None
    restored_hash = None
    logs = {}
    detail = None
    try:
        green = execute("baseline.green", logs)
        require_selected_count(green.stdout, case.get("expected_selected", 1))
        if green.returncode != 0 or "test result: ok" not in green.stdout:
            raise RuntimeError(f'{case["id"]}: baseline is not green\n{green.stdout[-5000:]}')

        planted = original.replace(anchor, case["replacement"], 1)
        path.write_text(planted)
        planted_hash = digest(path)
        red = execute("plant.red", logs)
        require_selected_count(red.stdout, case.get("expected_selected", 1))
        if "could not compile" in red.stdout or re.search(r"error\[E\d+\]", red.stdout):
            raise RuntimeError(f'{case["id"]}: plant caused a compile failure\n{red.stdout[-5000:]}')
        if red.returncode == 0:
            raise RuntimeError(f'{case["id"]}: planted test remained green\n{red.stdout[-5000:]}')
        semantic_lines = [
            line.strip()
            for line in red.stdout.splitlines()
            if case["expected_failure"] in line
            and not line.lstrip().startswith("thread '")
            and not line.lstrip().startswith("failures:")
            and not line.lstrip().startswith("test ")
        ]
        if not semantic_lines:
            raise RuntimeError(
                f'{case["id"]}: red lacked semantic failure {case["expected_failure"]!r}\n{red.stdout[-5000:]}'
            )
        detail = f"semantic assertion: {semantic_lines[0]}"
        print(f'{case["id"]}: RED selected={selected_count(red.stdout)} assertion={semantic_lines[0]!r}')

        path.write_text(original)
        restored_hash = digest(path)
        if restored_hash != baseline_hash:
            raise RuntimeError(f'{case["id"]}: source restore hash differs: {baseline_hash} != {restored_hash}')
        restored = execute("restored.green", logs)
        require_selected_count(restored.stdout, case.get("expected_selected", 1))
        if restored.returncode != 0 or "test result: ok" not in restored.stdout:
            raise RuntimeError(f'{case["id"]}: restored source is not green\n{restored.stdout[-5000:]}')
    except Exception as exc:
        path.write_text(original)
        restored_hash = digest(path)
        write_case_evidence(
            evidence_dir, case["id"], baseline_hash, restored_hash, logs,
            "failed", planted_hash, str(exc),
        )
        raise
    else:
        write_case_evidence(
            evidence_dir, case["id"], baseline_hash, restored_hash, logs,
            "passed", planted_hash, detail,
        )
        print(f'{case["id"]}: RESTORED GREEN source_sha256={restored_hash}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    parser.add_argument(
        "--evidence-root",
        type=Path,
        default=Path(os.environ.get("TURTS_REFUTATION_EVIDENCE_DIR", ROOT / "target/m2-refutations")),
    )
    parser.add_argument(
        "--build-root",
        type=Path,
        default=Path(os.environ.get("TURTS_REFUTATION_BUILD_ROOT", ROOT / "target/m2-refutation-builds")),
    )
    args = parser.parse_args()
    cases = select_cases(json.loads(REGISTRY.read_text()), args.case)
    run_root = create_run_workspace(args.evidence_root, args.build_root)
    build_root = Path((run_root / "build-root.txt").read_text().strip())
    (run_root / "selection.json").write_text(json.dumps([case["id"] for case in cases], indent=2) + "\n")
    for case in cases:
        case_id = re.sub(r"[^A-Za-z0-9_.-]", "_", case["id"])
        scratch = run_root / "scratch" / case_id
        build_dir = build_root / case_id
        evidence_dir = run_root / "cases" / case_id
        copy_source_tree(ROOT, scratch)
        build_dir.mkdir(parents=True, exist_ok=False)
        evidence_dir.mkdir(parents=True, exist_ok=False)
        run_case(case, scratch, build_dir, evidence_dir)
    print(f"evidence: {run_root}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"refutations: {exc}", file=sys.stderr)
        raise SystemExit(1)
