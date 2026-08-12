#!/usr/bin/env python3
"""Compose named milestones from exact independent platform lanes."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import (
        EvidenceError,
        atomic_write,
        canonical_json_bytes,
        read_json_object,
        safe_relative_path,
        sha256_file,
    )
except ModuleNotFoundError:
    from evidence_core import (  # type: ignore[no-redef]
        EvidenceError,
        atomic_write,
        canonical_json_bytes,
        read_json_object,
        safe_relative_path,
        sha256_file,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
POLICY_PATH: Final = ROOT / "architecture" / "platform-lane-policy.json"
STATUS_PATH: Final = ROOT / "architecture" / "platform-lane-status.json"
REPORT_PATH: Final = ROOT / "evidence" / "current" / "platform-gate-report.json"
STATES: Final = {"pass", "block", "unsupported", "not-applicable"}


class PlatformGateError(ValueError):
    """Raised when lane evidence or milestone composition is ambiguous."""


def _ordered_unique_strings(value: Any, label: str) -> list[str]:
    if (
        not isinstance(value, list)
        or not all(isinstance(item, str) and item for item in value)
        or value != sorted(set(value))
    ):
        raise PlatformGateError(f"{label}.invalid")
    return value


def validate_policy(policy: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(policy, dict) or set(policy) != {
        "schema_version",
        "policy_id",
        "states",
        "lane_ids",
        "milestones",
    }:
        return ["platform.policy.field_closure"]
    if policy.get("schema_version") != 1 or policy.get("policy_id") != "agentmage-platform-lane-policy-v1":
        failures.append("platform.policy.identity")
    try:
        lane_ids = _ordered_unique_strings(policy.get("lane_ids"), "platform.lanes")
        states = _ordered_unique_strings(policy.get("states"), "platform.states")
    except PlatformGateError as error:
        failures.append(str(error))
        lane_ids, states = [], []
    if set(states) != STATES:
        failures.append("platform.states.closure")
    milestones = policy.get("milestones")
    if not isinstance(milestones, list):
        failures.append("platform.milestones.invalid")
    else:
        identities: list[str] = []
        for milestone in milestones:
            if not isinstance(milestone, dict) or set(milestone) != {
                "milestone_id",
                "required_lanes",
            }:
                failures.append("platform.milestone.field_closure")
                continue
            identity = milestone.get("milestone_id")
            if not isinstance(identity, str) or not identity:
                failures.append("platform.milestone.identity")
                continue
            identities.append(identity)
            try:
                required = _ordered_unique_strings(
                    milestone.get("required_lanes"), f"platform.{identity}.required"
                )
            except PlatformGateError as error:
                failures.append(str(error))
                continue
            if not required or not set(required).issubset(lane_ids):
                failures.append(f"platform.{identity}.unknown_lane")
        if identities != sorted(set(identities)):
            failures.append("platform.milestone.order_or_duplicate")
    return sorted(set(failures))


def validate_status(status: Any, policy: dict[str, Any], root: Path) -> list[str]:
    failures: list[str] = []
    if not isinstance(status, dict) or set(status) != {
        "schema_version",
        "status_id",
        "support_claim",
        "lanes",
    }:
        return ["platform.status.field_closure"]
    if status.get("schema_version") != 1 or status.get("status_id") != "agentmage-current-platform-lanes-v1":
        failures.append("platform.status.identity")
    if status.get("support_claim") != "none-pre-release":
        failures.append("platform.status.support_overclaim")
    lanes = status.get("lanes")
    if not isinstance(lanes, list):
        return [*failures, "platform.status.lanes"]
    identities: list[str] = []
    for lane in lanes:
        if not isinstance(lane, dict) or set(lane) != {
            "lane_id",
            "native_lane",
            "state",
            "reason",
            "evidence_basis",
        }:
            failures.append("platform.lane.field_closure")
            continue
        lane_id = lane.get("lane_id")
        identities.append(str(lane_id))
        if lane.get("native_lane") != lane_id:
            failures.append(f"platform.{lane_id}.evidence_substitution")
        if lane.get("state") not in STATES:
            failures.append(f"platform.{lane_id}.state")
        if not isinstance(lane.get("reason"), str) or not lane.get("reason"):
            failures.append(f"platform.{lane_id}.reason")
        evidence = lane.get("evidence_basis")
        if not isinstance(evidence, list) or not evidence or evidence != sorted(set(evidence)):
            failures.append(f"platform.{lane_id}.evidence")
            continue
        for path in evidence:
            if not safe_relative_path(path):
                failures.append(f"platform.{lane_id}.evidence_path")
                continue
            try:
                sha256_file(root, path)
            except EvidenceError:
                failures.append(f"platform.{lane_id}.evidence_missing")
    if identities != policy.get("lane_ids"):
        failures.append("platform.status.lane_closure")
    return sorted(set(failures))


def compose_milestone(
    milestone: dict[str, Any], lanes: dict[str, dict[str, Any]]
) -> dict[str, Any]:
    """Compose one gate from only its explicitly required lanes."""

    required = milestone["required_lanes"]
    results = []
    for lane_id in required:
        lane = lanes[lane_id]
        results.append(
            {
                "lane_id": lane_id,
                "state": lane["state"],
                "reason": lane["reason"],
                "required": True,
            }
        )
    blockers = [item["lane_id"] for item in results if item["state"] != "pass"]
    return {
        "milestone_id": milestone["milestone_id"],
        "status": "pass" if not blockers else "block",
        "required_lanes": required,
        "lane_results": results,
        "blocking_lanes": blockers,
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    policy = read_json_object(root, POLICY_PATH.relative_to(ROOT).as_posix())
    status = read_json_object(root, STATUS_PATH.relative_to(ROOT).as_posix())
    failures = validate_policy(policy) + validate_status(status, policy, root)
    if failures:
        raise PlatformGateError("; ".join(failures))
    lanes = {lane["lane_id"]: lane for lane in status["lanes"]}
    milestones = [compose_milestone(milestone, lanes) for milestone in policy["milestones"]]
    return {
        "schema_version": 1,
        "report_id": "agentmage-current-platform-gates-v1",
        "inputs": [
            {
                "path": POLICY_PATH.relative_to(ROOT).as_posix(),
                "sha256": sha256_file(root, POLICY_PATH.relative_to(ROOT).as_posix()),
            },
            {
                "path": STATUS_PATH.relative_to(ROOT).as_posix(),
                "sha256": sha256_file(root, STATUS_PATH.relative_to(ROOT).as_posix()),
            },
        ],
        "lanes": status["lanes"],
        "milestones": milestones,
        "support_claim": "none-pre-release",
    }


def check_report(root: Path = ROOT) -> list[str]:
    try:
        expected = build_report(root)
        actual = read_json_object(root, REPORT_PATH.relative_to(ROOT).as_posix())
    except (EvidenceError, PlatformGateError) as error:
        return [str(error)]
    return [] if actual == expected else ["platform.gate_report_stale"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    if args.write:
        try:
            atomic_write(REPORT_PATH, canonical_json_bytes(build_report()))
        except (EvidenceError, PlatformGateError, OSError) as error:
            print(f"platform gates failed: {error}", file=sys.stderr)
            return 1
    failures = check_report()
    if failures:
        for failure in failures:
            print(f"platform gates failed: {failure}", file=sys.stderr)
        return 1
    print("independent platform lanes and named milestone composition validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
