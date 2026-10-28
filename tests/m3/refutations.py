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
PROJECT_ROOT = ROOT
REGISTRY = ROOT / "tests/m3/refutations.json"
FROZEN_M2_FIXTURE_SHA256 = "bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b"
EXPECTED_CASES = [f"C{index}-" for index in range(1, 18)]
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
    env["CARGO_PROFILE_DEV_DEBUG"] = "0"
    env["CARGO_PROFILE_TEST_DEBUG"] = "0"
    env["CARGO_INCREMENTAL"] = "0"
    env["RUST_BACKTRACE"] = "0"
    env["M3_SOURCE_COMMIT"] = subprocess.run(
        ["git", "-C", str(ROOT), "rev-parse", "HEAD"], check=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout.strip()
    env["M3_SOURCE_BRANCH"] = subprocess.run(
        ["git", "-C", str(ROOT), "branch", "--show-current"], check=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout.strip()
    source_diff = subprocess.run(
        ["git", "-C", str(ROOT), "diff", "--binary", "HEAD"], check=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout
    env["M3_SOURCE_DIFF_SHA256"] = hashlib.sha256(source_diff).hexdigest()
    env["M3_SOURCE_STATUS"] = subprocess.run(
        ["git", "-C", str(ROOT), "status", "--porcelain=v1"], check=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout
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


def browser_summary(output, case):
    expected_ids = case.get("browser_case_ids", [case["id"]])
    expected_count = case.get("expected_selected", len(expected_ids))
    if expected_count <= 0 or len(expected_ids) != expected_count or len(set(expected_ids)) != expected_count:
        raise ValueError(f'{case["id"]}: browser expected IDs/count are not a nonempty exact set')
    starts = []
    terminals = []
    failures = []
    summaries = []
    for line in output.splitlines():
        if line.startswith("M3_CASE_START "):
            starts.append(line[len("M3_CASE_START "):].strip())
        elif line.startswith("M3_CASE_PASS "):
            terminals.append((line[len("M3_CASE_PASS "):].strip(), "pass", None))
        elif line.startswith("M3_CASE_SEMANTIC_FAIL "):
            remainder = line[len("M3_CASE_SEMANTIC_FAIL "):]
            case_id, separator, encoded = remainder.partition(" ")
            if not separator:
                raise ValueError(f'{case["id"]}: malformed semantic-failure marker')
            try:
                message = json.loads(encoded)
            except json.JSONDecodeError as error:
                raise ValueError(f'{case["id"]}: semantic-failure marker message is not JSON') from error
            if not isinstance(message, str):
                raise ValueError(f'{case["id"]}: semantic-failure marker message must be JSON text')
            terminals.append((case_id, "semantic_fail", message))
            failures.append((case_id, message))
        elif line.startswith("M3_CASE_INFRA_FAIL ") or line.startswith("M3_CASE_TIMEOUT "):
            marker, remainder = line.split(" ", 1)
            case_id, separator, encoded = remainder.partition(" ")
            if not separator:
                raise ValueError(f'{case["id"]}: malformed {marker} marker')
            try:
                message = json.loads(encoded)
            except json.JSONDecodeError as error:
                raise ValueError(f'{case["id"]}: {marker} message is not JSON') from error
            raise ValueError(f'{case["id"]}: invalid browser {marker} for {case_id}: {message}')
        elif line.startswith("M3_CASE_SUMMARY "):
            try:
                summaries.append(json.loads(line[len("M3_CASE_SUMMARY "):]))
            except json.JSONDecodeError as error:
                raise ValueError(f'{case["id"]}: browser summary is not JSON') from error
    if starts != expected_ids or len(starts) != len(set(starts)):
        raise ValueError(f'{case["id"]}: browser start markers do not exactly match expected IDs')
    terminal_ids = [item[0] for item in terminals]
    if len(terminal_ids) != expected_count or set(terminal_ids) != set(expected_ids) or len(set(terminal_ids)) != len(terminal_ids):
        raise ValueError(f'{case["id"]}: browser terminal markers do not exactly match starts')
    if len(summaries) != 1:
        raise ValueError(f'{case["id"]}: expected exactly one browser summary')
    summary = summaries[0]
    passed = sum(item[1] == "pass" for item in terminals)
    failed = sum(item[1] == "semantic_fail" for item in terminals)
    if summary != {
        "selected_count": expected_count,
        "completed_count": expected_count,
        "passed_count": passed,
        "semantic_failed_count": failed,
        "infrastructure_failed_count": 0,
        "timeout_count": 0,
    }:
        raise ValueError(f'{case["id"]}: browser summary counts disagree with markers')
    return expected_count, passed, failed, failures


def require_phase_result(case, phase, result):
    if case.get("runtime", "rust") == "rust":
        require_selected_count(result.stdout, case.get("expected_selected", 1))
        if phase in {"baseline.green", "restored.green"}:
            if result.returncode != 0 or "test result: ok" not in result.stdout:
                raise RuntimeError(f'{case["id"]}: {phase} is not green\n{result.stdout[-5000:]}')
            return
        if "could not compile" in result.stdout or re.search(r"error\[E\d+\]", result.stdout):
            raise RuntimeError(f'{case["id"]}: plant caused a compile failure\n{result.stdout[-5000:]}')
        if result.returncode == 0:
            raise RuntimeError(f'{case["id"]}: planted test remained green\n{result.stdout[-5000:]}')
        semantic_lines = [
            line.strip()
            for line in result.stdout.splitlines()
            if case["expected_failure"] in line
            and not line.lstrip().startswith("thread '")
            and not line.lstrip().startswith("failures:")
            and not line.lstrip().startswith("test ")
        ]
        if not semantic_lines:
            raise RuntimeError(
                f'{case["id"]}: red lacked semantic failure {case["expected_failure"]!r}\n{result.stdout[-5000:]}'
            )
        return semantic_lines[0]

    selected, passed, failed, failures = browser_summary(result.stdout, case)
    if phase in {"baseline.green", "restored.green"}:
        if result.returncode != 0 or passed != selected or failed != 0:
            raise RuntimeError(f'{case["id"]}: {phase} browser cases are not all green\n{result.stdout[-5000:]}')
        return
    if result.returncode == 0 or failed == 0:
        raise RuntimeError(f'{case["id"]}: planted browser case lacked a semantic red\n{result.stdout[-5000:]}')
    semantic = [message for _, message in failures if case["expected_failure"] in message]
    if not semantic:
        raise RuntimeError(
            f'{case["id"]}: browser red lacked semantic failure {case["expected_failure"]!r}\n{result.stdout[-5000:]}'
        )
    return semantic[0]

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
    elif requested in {"ci-native", "ci-browser"}:
        runtime = "rust" if requested == "ci-native" else "browser"
        selected = [case for case in cases if case.get("runtime", "rust") == runtime]
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


def write_case_evidence(
    evidence_dir, case_id, baseline_hash, restored_hash, logs,
    status="passed", planted_hash=None, detail=None, source_hashes=None,
):
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
        "source_sha256": source_hashes or {},
        "detail": detail,
        "logs": sorted(logs),
    }
    (evidence_dir / "case.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


def apply_plant(original, case):
    replacements = case.get("replacements")
    if replacements is None:
        replacements = [{"anchor": case["anchor"], "replacement": case["replacement"]}]
    planted = original
    for item in replacements:
        anchor = item["anchor"]
        count = planted.count(anchor)
        if count != 1:
            raise RuntimeError(f'{case["id"]}: anchor occurs {count} times, expected exactly one')
        planted = planted.replace(anchor, item["replacement"], 1)
    return planted


def run_case(case, scratch, build_dir, evidence_dir):
    def execute(phase, logs):
        for index, command in enumerate(pre_commands(case), 1):
            result = run(command, scratch, build_dir)
            logs[f"{phase}.precommand-{index}.log"] = result.stdout
            write_case_evidence(
                evidence_dir, case["id"], baseline_hash, restored_hash, logs,
                "running", planted_hash, f"{phase} prerequisite {index}", source_hashes,
            )
            if result.returncode != 0:
                raise RuntimeError(
                    f'{case["id"]}: prerequisite {index} failed in {phase}\n{result.stdout[-5000:]}'
                )
        result = run(case["command"], scratch, build_dir)
        logs[f"{phase}.log"] = result.stdout
        write_case_evidence(
            evidence_dir, case["id"], baseline_hash, restored_hash, logs,
            "running", planted_hash, phase, source_hashes,
        )
        return result

    plants = case.get("plants") or [{
        "path": case["plant_path"],
        "anchor": case.get("anchor"),
        "replacement": case.get("replacement"),
        "replacements": case.get("replacements"),
    }]
    originals = {}
    baseline_hashes = {}
    for plant in plants:
        relative = plant["path"]
        path = scratch / relative
        if relative in originals:
            raise ValueError(f'{case["id"]}: source file listed more than once: {relative}')
        originals[relative] = path.read_text()
        baseline_hashes[relative] = digest(path)
    baseline_hash = hashlib.sha256(json.dumps(baseline_hashes, sort_keys=True).encode()).hexdigest()
    planted_hash = None
    restored_hash = None
    logs = {}
    detail = None
    source_hashes = dict(baseline_hashes)
    for relative in case.get("provenance_paths", []):
        provenance_path = Path(relative)
        if provenance_path.is_absolute() or ".." in provenance_path.parts:
            raise ValueError(f'{case["id"]}: provenance path must stay inside source tree')
        source_hashes[relative] = digest(scratch / provenance_path)
    try:
        green = execute("baseline.green", logs)
        require_phase_result(case, "baseline.green", green)

        for plant in plants:
            identified_plant = {**plant, "id": case["id"]}
            (scratch / plant["path"]).write_text(apply_plant(originals[plant["path"]], identified_plant))
        planted_hashes = {relative: digest(scratch / relative) for relative in originals}
        planted_hash = hashlib.sha256(json.dumps(planted_hashes, sort_keys=True).encode()).hexdigest()
        red = execute("plant.red", logs)
        semantic = require_phase_result(case, "plant.red", red)
        detail = f"semantic assertion: {semantic}"
        print(f'{case["id"]}: RED assertion={semantic!r}')

        for relative, original in originals.items():
            (scratch / relative).write_text(original)
        restored_hashes = {relative: digest(scratch / relative) for relative in originals}
        restored_hash = hashlib.sha256(json.dumps(restored_hashes, sort_keys=True).encode()).hexdigest()
        if restored_hash != baseline_hash:
            raise RuntimeError(f'{case["id"]}: source restore hash differs: {baseline_hash} != {restored_hash}')
        restored = execute("restored.green", logs)
        require_phase_result(case, "restored.green", restored)
    except Exception as exc:
        for relative, original in originals.items():
            (scratch / relative).write_text(original)
        restored_hashes = {relative: digest(scratch / relative) for relative in originals}
        restored_hash = hashlib.sha256(json.dumps(restored_hashes, sort_keys=True).encode()).hexdigest()
        write_case_evidence(
            evidence_dir, case["id"], baseline_hash, restored_hash, logs,
            "failed", planted_hash, str(exc), source_hashes,
        )
        raise
    else:
        for relative, expected_hash in source_hashes.items():
            if digest(scratch / relative) != expected_hash:
                raise RuntimeError(f'{case["id"]}: provenance source changed during case: {relative}')
        write_case_evidence(
            evidence_dir, case["id"], baseline_hash, restored_hash, logs,
            "passed", planted_hash, detail, source_hashes,
        )
        print(f'{case["id"]}: RESTORED GREEN source_sha256={restored_hash}')



def source_provenance(root):
    revision = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"], check=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout.strip()
    branch = subprocess.run(
        ["git", "-C", str(root), "branch", "--show-current"], check=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout.strip()
    diff = subprocess.run(
        ["git", "-C", str(root), "diff", "--binary", "HEAD"], check=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout
    status = subprocess.run(
        ["git", "-C", str(root), "status", "--porcelain=v1"], check=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout
    paths = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--", "*.rs", "Cargo.toml", "**/Cargo.toml", "Cargo.lock", "build.rs"],
        check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout.split(b"\0")
    source_hashes = {}
    for raw in paths:
        if not raw:
            continue
        relative = raw.decode()
        source_hashes[relative] = digest(root / relative)
    return {
        "commit": revision,
        "branch": branch,
        "diff_sha256": hashlib.sha256(diff).hexdigest(),
        "status": status,
        "source_files": source_hashes,
        "source_tree_sha256": hashlib.sha256(json.dumps(source_hashes, sort_keys=True).encode()).hexdigest(),
    }

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    parser.add_argument(
        "--evidence-root",
        type=Path,
        default=Path(os.environ.get("TURTS_REFUTATION_EVIDENCE_DIR", PROJECT_ROOT / "target/m3-evidence/t16-builder/refutations")),
    )
    parser.add_argument(
        "--build-root",
        type=Path,
        default=Path(os.environ.get("TURTS_REFUTATION_BUILD_ROOT", PROJECT_ROOT / "target/m3-t16/refutation-builds")),
    )
    args = parser.parse_args()
    fixture = ROOT / "tests/m3/m2_operation_stream.json"
    fixture_hash = digest(fixture)
    if fixture_hash != FROZEN_M2_FIXTURE_SHA256:
        raise ValueError(f"frozen M2 baseline changed: {fixture_hash}")
    all_cases = json.loads(REGISTRY.read_text())
    ids = [case.get("id") for case in all_cases]
    if len(ids) != 17 or any(not isinstance(case_id, str) or not case_id.startswith(prefix) for case_id, prefix in zip(ids, EXPECTED_CASES, strict=True)):
        raise ValueError("registry must contain C1-C17 in SPEC order, exactly once")
    cases = select_cases(all_cases, args.case)
    run_root = create_run_workspace(args.evidence_root, args.build_root)
    (run_root / "frozen-m2-baseline.json").write_text(json.dumps({"path": "tests/m3/m2_operation_stream.json", "sha256": fixture_hash}, indent=2) + "\n")
    (run_root / "source-provenance.json").write_text(json.dumps(source_provenance(ROOT), indent=2) + "\n")
    build_root = Path((run_root / "build-root.txt").read_text().strip())
    (run_root / "selection.json").write_text(json.dumps([case["id"] for case in cases], indent=2) + "\n")
    scratch = run_root / "scratch" / "source"
    build_dir = build_root / "cargo-target"
    copy_source_tree(ROOT, scratch)
    build_dir.mkdir(parents=True, exist_ok=False)
    for case in cases:
        case_id = re.sub(r"[^A-Za-z0-9_.-]", "_", case["id"])
        evidence_dir = run_root / "cases" / case_id
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
