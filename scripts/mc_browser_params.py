"""Validated row options for the headed MC browser launcher."""

from __future__ import annotations

from typing import Any
import hashlib
import json
import math
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
