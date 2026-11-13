"""Validated row options for the headed MC browser launcher."""

from __future__ import annotations

from typing import Any
import hashlib
import json
import math
import re
from pathlib import Path


def synthetic_units(row: dict[str, Any]) -> int | None:
    """Return the optional synthetic count, rejecting bools and out-of-range values."""
    if not isinstance(row, dict):
        raise ValueError("row must be a JSON object")
    if "synthetic_units" not in row:
        return None
    value = row["synthetic_units"]
    if type(value) is not int or not 0 <= value <= 0xFFFF_FFFF:
        raise ValueError("row.synthetic_units must be a JSON integer from 0 through 4294967295")
    return value


def capture_order_log(row: dict[str, Any]) -> bool:
    """Enable the existing server order-transition log for the exact GM40 row only."""
    if not isinstance(row, dict):
        raise ValueError("row must be a JSON object")
    if "capture_order_log" not in row:
        return False
    value = row["capture_order_log"]
    if type(value) is not bool:
        raise ValueError("row.capture_order_log must be a boolean")
    if value and row.get("row") != "GM40":
        raise ValueError("capture_order_log is supported only for row GM40")
    if value and row.get("capture_scene_checkpoints") is not True:
        raise ValueError("GM40 capture_order_log requires capture_scene_checkpoints=true")
    return value


def synthetic_server_args(row: dict[str, Any]) -> list[str]:
    """Build only the approved synthetic-unit server arguments."""
    value = synthetic_units(row)
    return [] if value is None else ["--synthetic-units", str(value)]


def add_synthetic_param(params: dict[str, Any], row: dict[str, Any]) -> dict[str, Any]:
    """Retain a declared synthetic count in browser params without changing absent rows."""
    value = synthetic_units(row)
    if value is None:
        return params
    params["synthetic_units"] = value
    return params


TASK16_ROWS = {"KM1", "KM2", "SEL4", "SLOT1", "CAM2", "SH1", "PER1"}
SEL4_MUSTER_FIXTURE = "tests/browser/fixtures/sel4-muster"
SEL4_MUSTER_FIXTURE_SHA256 = "05411bc117419e5321891c807911fd9e7640715e60943a1f242a44fdd3e65230"
TASK16_IO_KEYS = {
    "click": {"x", "y", "button"},
    "drag": {"x", "y", "x2", "y2", "button"},
    "key": {"code", "modifiers", "options"},
    "move": {"x", "y"},
    "reload": set(),
    "use": {"window"},
    "wait": {"ms"},
    "wheel": {"x", "y", "delta"},
}
TASK16_REQUIRED_CHECKS = {
    "KM1": {"keyq-bound-action", "mod-default", "player-override-reload", "reserved-mod-exit", "reserved-player-notice", "labels-guide", "labels-guide-pixels"},
    "KM2": {"join-focus", "contextmenu-prevented-delta", "canvas-blur-overlay", "outside-key-stopped", "p1-attachment", "canvas-blur-pixels"},
    "SEL4": {"ctrl-1-assign", "1-recall", "shift-1-add", "double-tap-centre", "same-frame-destroy-prune", "reload-live-ids", "old-match-prune", "double-tap-pixels"},
    "SLOT1": {"reload-same-id", "same-slot-rejoin", "unclaimed-third-refusal", "copied-tab-terminal-refusal", "owners-stable", "reload-to-authority-removal", "clone-terminal-pixels"},
    "CAM2": {"pointer-lock", "summed-motion-cursor", "edge-band-pan", "f10-menu", "p1-attachment", "cursor-pixels"},
    "SH1": {"load-phases", "wrong-lockfile-guidance", "menu-controls", "conflict-swap-cancel-reset", "guide-effective-label", "scroll-distance-ratio", "ui-scale-rects", "cursor-lock", "connection-lost-reload", "zero-menu-world-orders", "menu-pixels"},
    "PER1": {"browser-key-names", "native-path-names", "compatibility-line", "mod-isolation", "defaults-and-single-notice", "old-match-groups-pruned", "volume-fields-preserved", "notice-pixels"},
}
TASK16_PIXEL_CHECKS = {
    "KM1": "labels-guide-pixels",
    "KM2": "canvas-blur-pixels",
    "SEL4": "double-tap-pixels",
    "SLOT1": "clone-terminal-pixels",
    "CAM2": "cursor-pixels",
    "SH1": "menu-pixels",
    "PER1": "notice-pixels",
}
TASK16_PHYSICAL_CHECKS = {
    "KM1": {"keyq-bound-action"},
    "KM2": {"contextmenu-prevented-delta", "outside-key-stopped"},
    "SEL4": {"ctrl-1-assign", "1-recall", "shift-1-add", "double-tap-centre"},
    "SLOT1": {"copied-tab-terminal-refusal", "reload-same-id"},
    "CAM2": {"pointer-lock", "summed-motion-cursor", "edge-band-pan", "f10-menu"},
    "SH1": {"menu-controls", "conflict-swap-cancel-reset", "cursor-lock", "connection-lost-reload"},
    "PER1": {"browser-key-names", "defaults-and-single-notice"},
}
TASK16_INPUT_CALLS = {
    "KM1": {"keyq-bound-action": "key"},
    "KM2": {"contextmenu-prevented-delta": "click", "outside-key-stopped": "key"},
    "SEL4": {"ctrl-1-assign": "key", "1-recall": "key", "shift-1-add": "key", "double-tap-centre": "key"},
    "SLOT1": {"copied-tab-terminal-refusal": "use", "reload-same-id": "reload"},
    "CAM2": {"pointer-lock": "click", "summed-motion-cursor": "move", "edge-band-pan": "move", "f10-menu": "key"},
    "SH1": {"menu-controls": "key", "conflict-swap-cancel-reset": "click", "cursor-lock": "click", "connection-lost-reload": "reload"},
    "PER1": {"browser-key-names": "click", "defaults-and-single-notice": "click"},
}
TASK16_INPUT_DETAILS = {
    "KM1": {"keyq-bound-action": {"physical_code": "KeyQ", "logical_key": "a"}},
    "KM2": {"contextmenu-prevented-delta": {"select": "contextmenu", "button": "right"}},
    "SEL4": {
        "ctrl-1-assign": {"physical_code": "Digit1", "modifiers": ["Control"]},
        "1-recall": {"physical_code": "Digit1", "modifiers": []},
        "shift-1-add": {"physical_code": "Digit1", "modifiers": ["Shift"]},
    },
    "SH1": {"menu-controls": {"physical_code": "F10", "modifiers": []}},
    "SLOT1": {"copied-tab-terminal-refusal": {"window": 3}, "reload-same-id": {"window": 1}},
}


def _finite_number(value: Any) -> bool:
    return type(value) in (int, float) and math.isfinite(value)


def validate_task16_plan(row: dict[str, Any]) -> dict[str, Any] | None:
    """Validate the bounded physical plan nested under the existing stages pass-through.

    This is deliberately separate from ``actions_file``: HD1 semantic actions keep their
    existing schema and meaning. A missing plan is allowed only for the retained preparation
    packet, which the shell driver refuses to execute.
    """
    row_id = row.get("row")
    if row_id not in TASK16_ROWS:
        return None
    stages = row.get("stages")
    if not isinstance(stages, dict) or set(stages) != {"task16"}:
        raise ValueError(f"{row_id} requires stages.task16; physical actions are not HD1 actions")
    plan = stages["task16"]
    if not isinstance(plan, dict):
        raise ValueError(f"{row_id} stages.task16 must be an object")
    allowed = {"status", "phases", "actions", "captures", "checks", "fixture", "gpu_device_audit", "startup_raf_probe", "client_settings"}
    extra = set(plan) - allowed
    if extra:
        raise ValueError(f"{row_id} stages.task16 has unknown fields: {', '.join(sorted(extra))}")
    status = plan.get("status")
    if not isinstance(status, str) or status not in {"preparation", "candidate", "executable"}:
        raise ValueError(f"{row_id} stages.task16.status must be preparation, candidate, or executable")
    gpu_device_audit = plan.get("gpu_device_audit", False)
    if type(gpu_device_audit) is not bool:
        raise ValueError(f"{row_id} stages.task16.gpu_device_audit must be a boolean")
    if gpu_device_audit and (row_id != "SLOT1" or status not in {"candidate", "executable"}):
        raise ValueError("GPU device audit is limited to SLOT1 physical runs")
    startup_raf_probe = plan.get("startup_raf_probe", False)
    if type(startup_raf_probe) is not bool:
        raise ValueError(f"{row_id} stages.task16.startup_raf_probe must be a boolean")
    if startup_raf_probe and (row_id != "SLOT1" or status != "candidate" or not gpu_device_audit):
        raise ValueError("startup RAF probe is limited to guarded SLOT1 diagnostics with GPU device audit enabled")
    # A stored player setting for every window of the row (tests/browser/harness/client-settings-seed.js).
    # Only SLOT1 (three clients on one host) stores one; the value range is the client range (pacing.rs).
    client_settings = plan.get("client_settings")
    if client_settings is not None:
        if row_id != "SLOT1":
            raise ValueError("stored client settings are limited to SLOT1")
        if not isinstance(client_settings, dict) or set(client_settings) != {"frame_limit", "reason"}:
            raise ValueError("SLOT1 stages.task16.client_settings requires exactly frame_limit and reason")
        frame_limit = client_settings["frame_limit"]
        if type(frame_limit) is not int or not (frame_limit == 0 or 10 <= frame_limit <= 240):
            raise ValueError("SLOT1 client_settings.frame_limit must be 0 or 10 to 240")
        reason = client_settings["reason"]
        if not isinstance(reason, str) or not reason.strip():
            raise ValueError("SLOT1 client_settings.reason must be a nonempty text")
    actions = plan.get("actions", [])
    captures = plan.get("captures", [])
    checks = plan.get("checks", [])
    phases = plan.get("phases", [])
    if not isinstance(actions, list) or not isinstance(captures, list) or not isinstance(checks, list):
        raise ValueError(f"{row_id} stages.task16 actions, captures, and checks must be arrays")
    if not isinstance(phases, list) or not phases or any(not isinstance(phase, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", phase) for phase in phases) or len(set(phases)) != len(phases):
        raise ValueError(f"{row_id} stages.task16.phases must contain unique nonempty names")
    if status == "preparation" and (actions or captures or checks):
        raise ValueError(f"{row_id} preparation plans cannot contain unreviewed physical actions or oracles")
    if status in {"candidate", "executable"} and (not actions or not captures or not checks):
        raise ValueError(f"{row_id} {status} plan needs nonempty actions, captures, and checks")
    if status == "candidate":
        if row_id == "SLOT1":
            if not str(row.get("instrument_status", "")).startswith("SOURCE CANDIDATE ONLY"):
                raise ValueError("SLOT1 candidate plans require guarded source preparation status")
        elif row_id == "PER1":
            if not str(row.get("instrument_status", "")).startswith("SOURCE CANDIDATE ONLY"):
                raise ValueError("PER1 candidate plans require guarded source preparation status")
        else:
            raise ValueError(f"{row_id} candidate plans are reserved for guarded source preparation")
    plan_has_work = status in {"candidate", "executable"}

    windows = row.get("windows", 1)
    if type(windows) is not int or not 1 <= windows <= 3:
        raise ValueError(f"{row_id}.windows must be an integer from 1 through 3")
    capture_names: set[str] = set()
    previous_capture_after: dict[str, int] = {}
    phase_ordinals = {phase: index for index, phase in enumerate(phases)}
    previous_capture_phase = -1
    for index, capture in enumerate(captures):
        if not isinstance(capture, dict) or set(capture) != {"name", "after_action", "window", "phase"}:
            raise ValueError(f"{row_id} capture {index + 1} requires name, after_action, window, and phase")
        name, after, window, phase = (capture["name"], capture["after_action"], capture["window"], capture["phase"])
        if not isinstance(name, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", name) or name in capture_names:
            raise ValueError(f"{row_id} capture names must be nonempty and unique")
        if phase not in phases or type(after) is not int or after < 0:
            raise ValueError(f"{row_id} capture {name} has an invalid or unordered after_action")
        if phase_ordinals.get(phase, -1) < previous_capture_phase:
            raise ValueError(f"{row_id} captures must follow declared phase order")
        previous_capture_phase = phase_ordinals.get(phase, -1)
        if after < previous_capture_after.get(phase, -1):
            raise ValueError(f"{row_id} capture {name} has an invalid or unordered after_action")
        if type(window) is not int or not 1 <= window <= windows:
            raise ValueError(f"{row_id} capture {name} selects a window outside 1..{windows}")
        if not isinstance(phase, str) or not phase:
            raise ValueError(f"{row_id} capture {name} needs a phase name")
        capture_names.add(name)
        previous_capture_after[phase] = after

    action_phases: set[str] = set()
    action_counts: dict[str, int] = {}
    for index, action in enumerate(actions):
        if not isinstance(action, dict) or not isinstance(action.get("call"), str):
            raise ValueError(f"{row_id} action {index + 1} must declare a known IO call")
        call = action["call"]
        if call not in TASK16_IO_KEYS:
            raise ValueError(f"{row_id} action {index + 1} uses unsupported IO call {call!r}")
        if set(action) - ({"call", "phase"} | TASK16_IO_KEYS[call]):
            raise ValueError(f"{row_id} action {index + 1} has unknown fields")
        phase = action.get("phase")
        if not isinstance(phase, str) or not phase:
            raise ValueError(f"{row_id} action {index + 1} needs a phase name")
        action_phases.add(phase)
        if phase not in phases:
            raise ValueError(f"{row_id} action {index + 1} uses undeclared phase {phase!r}")
        action_counts[phase] = action_counts.get(phase, 0) + 1
        if call in {"click", "drag", "move", "wheel"}:
            point_fields = ("x", "y", "x2", "y2") if call == "drag" else (("x", "y") if call in {"click", "move", "wheel"} else ())
            for field in point_fields:
                if field not in action or not _finite_number(action[field]):
                    raise ValueError(f"{row_id} action {index + 1} {field} must be finite")
        if call == "wheel" and not _finite_number(action.get("delta")):
            raise ValueError(f"{row_id} action {index + 1} delta must be finite")
        if call == "wait" and (type(action.get("ms")) is not int or not 0 <= action["ms"] <= 60_000):
            raise ValueError(f"{row_id} action {index + 1} ms must be an integer in 0..60000")
        if call == "use" and (type(action.get("window")) is not int or not 1 <= action["window"] <= windows):
            raise ValueError(f"{row_id} action {index + 1} selects a window outside 1..{windows}")
        if call == "key":
            if not isinstance(action.get("code"), str) or not action["code"]:
                raise ValueError(f"{row_id} action {index + 1} needs a physical code")
            if "modifiers" in action and (not isinstance(action["modifiers"], list) or any(not isinstance(item, str) for item in action["modifiers"])):
                raise ValueError(f"{row_id} action {index + 1} modifiers must be strings")
            options = action.get("options", {})
            if not isinstance(options, dict) or set(options) - {"key", "hold_ms", "action"}:
                raise ValueError(f"{row_id} action {index + 1} has invalid key options")
            if "hold_ms" in options and (type(options["hold_ms"]) is not int or not 0 <= options["hold_ms"] <= 60_000):
                raise ValueError(f"{row_id} action {index + 1} key hold_ms must be an integer in 0..60000")
            if "action" in options and (not isinstance(options["action"], str) or options["action"] not in {"press", "down", "up"}):
                raise ValueError(f"{row_id} action {index + 1} key action must be press, down, or up")
        if call in {"click", "drag"} and "button" in action and (not isinstance(action["button"], str) or action["button"] not in {"left", "right", "middle"}):
            raise ValueError(f"{row_id} action {index + 1} button is unsupported")
    if set(action_phases) - set(phases):
        raise ValueError(f"{row_id} physical actions use undeclared phases")
    for capture in captures:
        if capture["after_action"] > action_counts.get(capture["phase"], 0):
            raise ValueError(f"{row_id} capture {capture['name']} after_action is outside its phase")
    capture_phases = {capture["phase"] for capture in captures}
    if plan_has_work and (not action_phases or capture_phases != action_phases):
        raise ValueError(f"{row_id} {status} plan needs actions and captures in every declared physical phase")
    if plan_has_work:
        if not capture_names or set(phases) != action_phases:
            raise ValueError(f"{row_id} {status} plan needs physical actions for every declared phase")
        if not all(isinstance(check, str) and check for check in checks):
            raise ValueError(f"{row_id} checks must be harness-owned clause ID strings")
        check_ids = checks
        if len(set(check_ids)) != len(check_ids):
            raise ValueError(f"{row_id} check ids must be unique")
        if set(check_ids) != TASK16_REQUIRED_CHECKS[row_id]:
            missing = sorted(TASK16_REQUIRED_CHECKS[row_id] - set(check_ids))
            extra_ids = sorted(set(check_ids) - TASK16_REQUIRED_CHECKS[row_id])
            raise ValueError(f"{row_id} clause checks mismatch; missing={missing}, extra={extra_ids}")
        # A row selects fixed oracle clauses by ID only. Paths, operators and expected values
        # are private harness semantics and cannot be supplied by the physical plan.
        pixel_check = TASK16_PIXEL_CHECKS[row_id]
        if pixel_check not in check_ids:
            raise ValueError(f"{row_id} requires calibrated capture clause {pixel_check}")
        for physical_check in TASK16_PHYSICAL_CHECKS[row_id]:
            if physical_check not in check_ids:
                raise ValueError(f"{row_id} requires physical input clause {physical_check}")
        for check_id, expected_call in TASK16_INPUT_CALLS[row_id].items():
            if not any(action["call"] == expected_call for action in actions):
                raise ValueError(f"{row_id} {check_id} requires a physical {expected_call} action")
        for check_id, expected_fields in TASK16_INPUT_DETAILS.get(row_id, {}).items():
            call = TASK16_INPUT_CALLS[row_id][check_id]
            def action_has_contract(action_index: int, action: dict[str, Any]) -> bool:
                if action["call"] != call:
                    return False
                for key, value in expected_fields.items():
                    if key == "physical_code" and action.get("code") != value:
                        return False
                    if key == "logical_key" and action.get("options", {}).get("key") != value:
                        return False
                    if key == "modifiers" and action.get("modifiers", []) != value:
                        return False
                    if key == "button" and action.get("button", "left") != value:
                        return False
                    if key == "window":
                        selected = action.get("window")
                        if selected is None and action["call"] == "reload":
                            selected = next((candidate["window"] for candidate in reversed(actions[:action_index])
                                             if candidate["call"] == "use"), None)
                        if selected != value:
                            return False
                    if key not in {"physical_code", "logical_key", "modifiers", "button", "select"}:
                        if key != "window":
                            return False
                return True
            if not any(action_has_contract(index, action) for index, action in enumerate(actions)):
                raise ValueError(f"{row_id} {check_id} lacks its fixed physical input action")
        if row_id == "SLOT1":
            copy_actions = [action for action in actions if action.get("phase") == "copy-refusal"]
            copied_focuses = [action.get("window") for action in copy_actions if action.get("call") == "use"]
            if len(copied_focuses) < 30 or len(copied_focuses) % 3 != 0 or any(copied_focuses[index:index + 3] != [1, 2, 3] for index in range(0, len(copied_focuses), 3)):
                raise ValueError("SLOT1 copy-refusal requires repeated physical use(1), use(2), use(3) rotations to keep all real sessions live")
            if any(action.get("call") == "wait" and action.get("ms", 0) > 400 for action in copy_actions):
                raise ValueError("SLOT1 copy-refusal waits must be at most 400 ms to keep both owners foregrounded")
            if sum(action.get("ms", 0) for action in copy_actions if action.get("call") == "wait") < 12_000:
                raise ValueError("SLOT1 copy-refusal needs at least 12000 ms of declared visible-window wait time")
            copy_captures = [capture for capture in captures if capture["phase"] == "copy-refusal"]
            final_capture_after = max(capture["after_action"] for capture in copy_captures)
            trailing_actions = copy_actions[final_capture_after:]
            if not any(capture["window"] == 3 for capture in copy_captures):
                raise ValueError("SLOT1 copied-tab phase needs an actual copied-window capture")
            for owner_window in (1, 2):
                owner_captures = [capture for capture in copy_captures if capture["window"] == owner_window]
                if len(owner_captures) < 2 or min(capture["after_action"] for capture in owner_captures) == max(capture["after_action"] for capture in owner_captures):
                    raise ValueError(f"SLOT1 copy-refusal requires beginning and ending actual owner frames for window {owner_window}")
            if sum(action.get("ms", 0) for action in trailing_actions if action.get("call") == "wait") < 1000:
                raise ValueError("SLOT1 must retain one second of physical observation after its final copied-tab capture")
            # The oracle binds these clauses to the current private server boot and complete
            # observer stream. A row cannot select another source or redefine their meaning.
    fixture = plan.get("fixture")
    if row_id == "SLOT1":
        if fixture != "slot1-authentic-copy-v1":
            raise ValueError("SLOT1 requires the harness-owned slot1-authentic-copy-v1 fixture")
        if windows != 3:
            raise ValueError("SLOT1 authentic copied-session fixture requires three simultaneous windows")
        if phases != ["capacity", "copy-refusal", "reload-rejoin"]:
            raise ValueError("SLOT1 phase order must be capacity, copy-refusal, reload-rejoin")
    elif row_id == "SEL4":
        if fixture != "sel4-authored-muster-death-v1":
            raise ValueError("SEL4 requires the isolated authored Muster combat fixture")
        if windows != 1:
            raise ValueError("SEL4 authored Muster combat fixture requires one player window")
    elif row_id == "PER1":
        if fixture not in {"per1-settings-corrupt-v1", "per1-invalid-binding-v1"}:
            raise ValueError("PER1 requires one of the two reviewed single-corrupt-value fixtures")
        if windows != 1:
            raise ValueError("PER1 storage fixture requires one browser window")
    elif row_id == "SH1":
        if fixture != "sh1-real-acquisition-v1" or windows != 1:
            raise ValueError("SH1 requires the bounded real-acquisition fixture in one window")
    elif fixture is not None:
        raise ValueError(f"{row_id} does not support a fixture")
    return plan


def validate_task16_fixture(row: dict[str, Any], repository_root: Path, package_arg: str) -> None:
    """Bind the approved SEL4 fixture row to its immutable one-placement package copy."""
    if row.get("row") != "SEL4":
        return
    package = row.get("package")
    if package != SEL4_MUSTER_FIXTURE or package_arg != SEL4_MUSTER_FIXTURE:
        raise ValueError("SEL4 must package the isolated tests/browser/fixtures/sel4-muster tree")
    if row.get("fixture_tree_sha256") != SEL4_MUSTER_FIXTURE_SHA256:
        raise ValueError("SEL4 fixture_tree_sha256 differs from the reviewed authored Muster copy")
    fixture_root = (repository_root / SEL4_MUSTER_FIXTURE).resolve(strict=True)
    root = repository_root.resolve(strict=True)
    if not fixture_root.is_relative_to(root):
        raise ValueError("SEL4 fixture path escaped the repository")
    digest = hashlib.sha256()
    for path in sorted(candidate for candidate in fixture_root.rglob("*") if candidate.is_file()):
        relative = path.relative_to(fixture_root).as_posix()
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(hashlib.sha256(path.read_bytes()).digest())
    if digest.hexdigest() != SEL4_MUSTER_FIXTURE_SHA256:
        raise ValueError("SEL4 authored Muster fixture tree differs from its recorded copy hash")


def add_task16_param(params: dict[str, Any], row: dict[str, Any]) -> dict[str, Any]:
    plan = validate_task16_plan(row)
    if plan is not None:
        params["task16"] = plan
    return params


def check_by_id_has_source(checks: list[dict[str, Any]], check_id: str, kind: str) -> bool:
    return any(check.get("id") == check_id and check.get("assertion", {}).get("source", {}).get("kind") == kind for check in checks)


def actions_file(row: dict[str, Any], fixture_root: Path) -> tuple[str, list[dict[str, Any]], str] | None:
    """Load an authored action list strictly from the browser fixture tree."""
    if "actions" in row:
        raise ValueError("inline actions are not supported; use actions_file")
    if "actions_file" not in row:
        return None
    value = row["actions_file"]
    if not isinstance(value, str) or not value or Path(value).is_absolute():
        raise ValueError("row.actions_file must be a relative path under tests/browser/fixtures")
    try:
        root = fixture_root.resolve(strict=True)
        candidate = (root / value).resolve(strict=True)
    except OSError as error:
        raise ValueError(f"row.actions_file is missing or cannot be resolved under tests/browser/fixtures: {error}") from error
    if not candidate.is_file() or not candidate.is_relative_to(root):
        raise ValueError("row.actions_file must resolve to a file under tests/browser/fixtures")
    raw = candidate.read_bytes()
    try:
        actions = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"row.actions_file is not valid JSON: {error}") from error
    if not isinstance(actions, list):
        raise ValueError("row.actions_file must contain a JSON action list")
    previous = 0
    for index, action in enumerate(actions):
        if not isinstance(action, dict):
            raise ValueError(f"action {index + 1} must be an object")
        if "tick" in action:
            tick = action["tick"]
            if type(tick) is not int or tick < 0 or tick > 0xFFFF_FFFF:
                raise ValueError(f"action {index + 1} tick must be an integer in 0..={0xFFFF_FFFF}")
            if index > 0 and tick < previous:
                raise ValueError(f"action {index + 1} has a tick before the previous action")
            previous = tick
        else:
            action["tick"] = previous
        if action.get("do") not in {"box_select", "click_select", "smart_command", "order", "group_assign", "group_recall"}:
            raise ValueError(f"action {index + 1} has an unsupported do value")
        kind = action["do"]
        fields = {
            "box_select": {"tick", "do", "world_rect", "queue"},
            "click_select": {"tick", "do", "world_point"},
            "smart_command": {"tick", "do", "world_point", "queue"},
            "order": {"tick", "do", "order", "world_point", "content_id", "job_id", "units", "queue", "ui_point"},
            "group_assign": {"tick", "do", "group"},
            "group_recall": {"tick", "do", "group"},
        }[kind]
        extra = set(action) - fields
        if extra:
            raise ValueError(f"action {index + 1} has unknown fields: {', '.join(sorted(extra))}")
        for coordinate in ("world_point", "ui_point"):
            if coordinate in action and (not isinstance(action[coordinate], list) or len(action[coordinate]) != 2 or any(type(value) not in (int, float) or not math.isfinite(value) for value in action[coordinate])):
                raise ValueError(f"action {index + 1} {coordinate} must be a pair of finite numbers")
        if "world_rect" in action and (not isinstance(action["world_rect"], list) or len(action["world_rect"]) != 4 or any(type(value) not in (int, float) or not math.isfinite(value) for value in action["world_rect"])):
            raise ValueError(f"action {index + 1} world_rect must contain four finite numbers")
        if "queue" in action and type(action["queue"]) is not bool:
            raise ValueError(f"action {index + 1} queue must be a boolean")
        if kind in {"group_assign", "group_recall"} and (type(action.get("group")) is not int or not 1 <= action["group"] <= 9):
            raise ValueError(f"action {index + 1} group must be an integer from 1 through 9")
        if kind == "order":
            order = action.get("order")
            if not isinstance(order, str) or not order or order.count(":") != 1:
                raise ValueError(f"action {index + 1} order must be a qualified catalogue order id")
            content_id = action.get("content_id")
            job_id = action.get("job_id")
            if content_id is not None and job_id is not None:
                raise ValueError(f"action {index + 1} cannot contain both content_id and job_id")
            if content_id is not None or job_id is not None:
                if "world_point" in action or not isinstance(action.get("units"), list) or not action["units"]:
                    raise ValueError(f"action {index + 1} typed Content/Job order requires units and no world_point")
                if content_id is not None and (not isinstance(content_id, str) or content_id.count(":") != 1):
                    raise ValueError(f"action {index + 1} content_id must be a qualified id")
                if job_id is not None and (type(job_id) is not int or not 0 <= job_id <= 0xFFFF_FFFF_FFFF_FFFF):
                    raise ValueError(f"action {index + 1} job_id must be an unsigned 64-bit integer")
                if action.get("ui_point") is None:
                    raise ValueError(f"action {index + 1} typed Content/Job order requires ui_point for browser input")
            if "units" in action and content_id is None and job_id is None:
                raise ValueError(f"action {index + 1} units are only valid for Content or Job orders")
            if "units" in action and (not isinstance(action["units"], list) or any(type(unit) is not int or not 0 <= unit <= 0xFFFF_FFFF for unit in action["units"])):
                raise ValueError(f"action {index + 1} units must be unsigned 32-bit integer ids")
        required = {"box_select": "world_rect", "click_select": "world_point", "smart_command": "world_point", "group_assign": "group", "group_recall": "group"}.get(kind)
        if required and required not in action:
            raise ValueError(f"action {index + 1} {kind} requires {required}")
        previous = action["tick"]
    return candidate.relative_to(root).as_posix(), actions, hashlib.sha256(raw).hexdigest()


def add_actions_param(params: dict[str, Any], row: dict[str, Any], fixture_root: Path) -> dict[str, Any]:
    loaded = actions_file(row, fixture_root)
    if loaded is None:
        return params
    relative_path, actions, digest = loaded
    params["actions"] = actions
    params["actions_file"] = relative_path
    params["actions_sha256"] = digest
    return params
