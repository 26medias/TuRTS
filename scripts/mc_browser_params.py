"""Validated row options for the headed MC browser launcher."""

from __future__ import annotations

from typing import Any


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
