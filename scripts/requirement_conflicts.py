#!/usr/bin/env python3
"""Resolve structured requirement conflicts to the narrower provisional boundary."""

from __future__ import annotations

import json
import math
from copy import deepcopy
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_POLICY: Final = ROOT / "requirements" / "conflict-policy.json"
BOUNDARY_KEYS: Final = {
    "id",
    "allow",
    "deny",
    "required_controls",
    "ceilings",
    "exact",
}
POLICY_KEYS: Final = {
    "automatic_resolution",
    "empty_intersection_behavior",
    "exact_mismatch_behavior",
    "policy_id",
    "required_decision",
    "schema_version",
    "strategies",
    "unknown_field_behavior",
}


class ConflictPolicyError(ValueError):
    """Raised when conflict-policy input cannot be resolved safely."""


def _string_list(value: object, field: str) -> list[str]:
    if not isinstance(value, list) or any(
        not isinstance(item, str) or not item for item in value
    ):
        raise ConflictPolicyError(f"{field} must be a list of non-empty strings")
    if len(value) != len(set(value)):
        raise ConflictPolicyError(f"{field} must not contain duplicates")
    return sorted(value)


def load_policy(path: Path = DEFAULT_POLICY) -> dict[str, object]:
    """Load and validate the supported conflict-policy version."""
    policy = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(policy, dict) or set(policy) != POLICY_KEYS:
        raise ConflictPolicyError("conflict policy has missing or unknown top-level fields")
    if policy["schema_version"] != 1:
        raise ConflictPolicyError("unsupported conflict-policy schema version")
    if policy["automatic_resolution"] != "provisional_narrower_boundary_only":
        raise ConflictPolicyError("automatic resolution must remain provisional and narrower")
    if policy["empty_intersection_behavior"] != "blocked":
        raise ConflictPolicyError("empty allowlist intersections must block")
    if policy["exact_mismatch_behavior"] != "blocked":
        raise ConflictPolicyError("exact-value mismatches must block")
    if policy["unknown_field_behavior"] != "blocked":
        raise ConflictPolicyError("unknown conflict fields must block")

    strategies = policy["strategies"]
    if not isinstance(strategies, dict) or set(strategies) != {
        "allowlist_intersection",
        "ceiling_minimum",
        "denylist_union",
        "exact_match_or_block",
        "required_controls_union",
    }:
        raise ConflictPolicyError("conflict policy has invalid strategy fields")
    for name in (
        "allowlist_intersection",
        "ceiling_minimum",
        "denylist_union",
        "exact_match_or_block",
    ):
        _string_list(strategies[name], f"strategies.{name}")
    if strategies["required_controls_union"] is not True:
        raise ConflictPolicyError("required controls must accumulate")

    decision = policy["required_decision"]
    if not isinstance(decision, dict) or decision != {
        "approval_status": "accepted",
        "must_preserve_superseded_requirements": True,
        "record_type": "requirement_supersession",
    }:
        raise ConflictPolicyError("approved decisions must preserve superseded requirements")
    return policy


def _normalize_map_of_lists(
    value: object,
    *,
    field: str,
    permitted: set[str],
) -> dict[str, list[str]]:
    if not isinstance(value, dict):
        raise ConflictPolicyError(f"{field} must be an object")
    unknown = sorted(set(value) - permitted)
    if unknown:
        raise ConflictPolicyError(f"unknown {field} fields: {', '.join(unknown)}")
    return {
        key: _string_list(items, f"{field}.{key}")
        for key, items in sorted(value.items())
    }


def normalize_boundary(
    boundary: dict[str, object], policy: dict[str, object]
) -> dict[str, object]:
    """Validate and normalize one structured requirement boundary."""
    if not isinstance(boundary, dict) or set(boundary) != BOUNDARY_KEYS:
        raise ConflictPolicyError("boundary has missing or unknown top-level fields")
    if not isinstance(boundary["id"], str) or not boundary["id"]:
        raise ConflictPolicyError("boundary.id must be a non-empty string")

    strategies = policy["strategies"]
    allow_fields = set(strategies["allowlist_intersection"])
    deny_fields = set(strategies["denylist_union"])
    ceiling_fields = set(strategies["ceiling_minimum"])
    exact_fields = set(strategies["exact_match_or_block"])

    ceilings = boundary["ceilings"]
    if not isinstance(ceilings, dict):
        raise ConflictPolicyError("ceilings must be an object")
    unknown_ceilings = sorted(set(ceilings) - ceiling_fields)
    if unknown_ceilings:
        raise ConflictPolicyError(
            f"unknown ceiling fields: {', '.join(unknown_ceilings)}"
        )
    normalized_ceilings: dict[str, int | float] = {}
    for key, value in sorted(ceilings.items()):
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or value < 0
        ):
            raise ConflictPolicyError(f"ceilings.{key} must be finite and non-negative")
        normalized_ceilings[key] = value

    exact = boundary["exact"]
    if not isinstance(exact, dict):
        raise ConflictPolicyError("exact must be an object")
    unknown_exact = sorted(set(exact) - exact_fields)
    if unknown_exact:
        raise ConflictPolicyError(f"unknown exact fields: {', '.join(unknown_exact)}")
    if any(not isinstance(value, str) or not value for value in exact.values()):
        raise ConflictPolicyError("exact values must be non-empty strings")

    return {
        "id": boundary["id"],
        "allow": _normalize_map_of_lists(
            boundary["allow"], field="allow", permitted=allow_fields
        ),
        "deny": _normalize_map_of_lists(
            boundary["deny"], field="deny", permitted=deny_fields
        ),
        "required_controls": _string_list(
            boundary["required_controls"], "required_controls"
        ),
        "ceilings": normalized_ceilings,
        "exact": dict(sorted(exact.items())),
    }


def resolve_conflict(
    left: dict[str, object],
    right: dict[str, object],
    policy: dict[str, object] | None = None,
) -> dict[str, object]:
    """Return the narrower provisional boundary without mutating either input."""
    active_policy = load_policy() if policy is None else deepcopy(policy)
    normalized_left = normalize_boundary(deepcopy(left), active_policy)
    normalized_right = normalize_boundary(deepcopy(right), active_policy)
    left_constraints = {
        key: value for key, value in normalized_left.items() if key != "id"
    }
    right_constraints = {
        key: value for key, value in normalized_right.items() if key != "id"
    }
    if left_constraints == right_constraints:
        source_ids = sorted({normalized_left["id"], normalized_right["id"]})
        unchanged_boundary = deepcopy(normalized_left)
        if len(source_ids) > 1:
            unchanged_boundary["id"] = "equivalent:" + "+".join(source_ids)
        return {
            "status": "unchanged",
            "requires_approved_decision": False,
            "policy_id": active_policy["policy_id"],
            "sources": source_ids,
            "unresolved_fields": [],
            "boundary": unchanged_boundary,
        }

    unresolved: list[str] = []
    allow: dict[str, list[str]] = {}
    allow_keys = set(normalized_left["allow"]) | set(normalized_right["allow"])
    for key in sorted(allow_keys):
        left_values = set(normalized_left["allow"].get(key, []))
        right_values = set(normalized_right["allow"].get(key, []))
        values = sorted(left_values & right_values)
        allow[key] = values
        if not values:
            unresolved.append(f"allow.{key}")

    deny: dict[str, list[str]] = {}
    deny_keys = set(normalized_left["deny"]) | set(normalized_right["deny"])
    for key in sorted(deny_keys):
        deny[key] = sorted(
            set(normalized_left["deny"].get(key, []))
            | set(normalized_right["deny"].get(key, []))
        )

    ceiling_keys = set(normalized_left["ceilings"]) | set(normalized_right["ceilings"])
    ceilings: dict[str, int | float] = {}
    for key in sorted(ceiling_keys):
        candidates = [
            boundary["ceilings"][key]
            for boundary in (normalized_left, normalized_right)
            if key in boundary["ceilings"]
        ]
        ceilings[key] = min(candidates)

    exact: dict[str, str] = {}
    exact_keys = set(normalized_left["exact"]) | set(normalized_right["exact"])
    for key in sorted(exact_keys):
        left_value = normalized_left["exact"].get(key)
        right_value = normalized_right["exact"].get(key)
        if left_value is None:
            exact[key] = right_value
        elif right_value is None:
            exact[key] = left_value
        elif left_value == right_value:
            exact[key] = left_value
        else:
            unresolved.append(f"exact.{key}")

    source_ids = sorted({normalized_left["id"], normalized_right["id"]})
    boundary = {
        "id": "provisional:" + "+".join(source_ids),
        "allow": allow,
        "deny": deny,
        "required_controls": sorted(
            set(normalized_left["required_controls"])
            | set(normalized_right["required_controls"])
        ),
        "ceilings": ceilings,
        "exact": exact,
    }
    return {
        "status": "blocked" if unresolved else "provisional",
        "requires_approved_decision": True,
        "policy_id": active_policy["policy_id"],
        "sources": source_ids,
        "unresolved_fields": sorted(unresolved),
        "boundary": boundary,
    }
