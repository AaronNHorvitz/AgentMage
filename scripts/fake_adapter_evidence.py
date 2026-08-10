#!/usr/bin/env python3
"""Validate deterministic fake adapter contracts and canonical evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from fixtures.fake_adapters import FakeMode, baseline_trace


CONTRACT_PATH = ROOT / "fixtures" / "fake-adapter-contract.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "fake-adapter-report.json"
)
EXPECTED_MODES = tuple(mode.value for mode in FakeMode)
EXPECTED_ADAPTERS = (
    ("fake-model", ("generate",)),
    ("fake-tool", ("invoke",)),
    ("fake-inference-runtime", ("start", "infer", "stop")),
    ("fake-connector", ("list", "read")),
    ("fake-clock", ("now", "advance")),
    ("fake-secret-store", ("store", "load", "delete")),
    ("crash-injector", ("checkpoint",)),
)
EXPECTED_SIDE_EFFECTS = {
    "executes_external_commands": False,
    "uses_network": False,
    "reads_real_workspace": False,
    "persists_state": False,
    "stores_real_secret": False,
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_contract(contract: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(contract, dict):
        return ["fake adapter contract must be an object"]
    if (
        contract.get("schema_version") != 1
        or contract.get("contract_id") != "agentmage-fake-adapters-v1"
        or contract.get("status") != "test-only"
    ):
        failures.append("fake adapter contract identity is invalid")
    if tuple(contract.get("modes", [])) != EXPECTED_MODES:
        failures.append("fake adapter mode closure drifted")
    adapters = tuple(
        (item.get("id"), tuple(item.get("operations", [])))
        for item in contract.get("adapters", [])
    )
    if adapters != EXPECTED_ADAPTERS:
        failures.append("fake adapter operation closure drifted")
    if contract.get("fixed_clock_epoch") != 1704067200:
        failures.append("fake adapter clock is not pinned")
    if contract.get("side_effect_contract") != EXPECTED_SIDE_EFFECTS:
        failures.append("fake adapter side-effect contract was weakened")
    if contract.get("product_adapter_claim") != "none":
        failures.append("fake adapter contract claimed product implementation")
    if contract.get("platform_support_claim") != "none":
        failures.append("fake adapter contract claimed platform support")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    contract_path = root / "fixtures/fake-adapter-contract.json"
    contract = read_json(contract_path)
    failures = validate_contract(contract)
    if failures:
        raise ValueError("; ".join(failures))
    trace = baseline_trace()
    return {
        "schema_version": 1,
        "task_id": "2.1.1.4",
        "status": "pass",
        "contract_sha256": sha256_file(contract_path),
        "adapter_count": len(EXPECTED_ADAPTERS),
        "mode_count": len(EXPECTED_MODES),
        "adapters": [
            {"id": adapter_id, "operations": list(operations)}
            for adapter_id, operations in EXPECTED_ADAPTERS
        ],
        "modes": list(EXPECTED_MODES),
        "baseline_trace": trace,
        "side_effect_contract": contract["side_effect_contract"],
        "product_adapter_claim": "none",
        "platform_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["fake adapter report must be an object"]
    failures = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.1.4":
        failures.append("fake adapter report identity is invalid")
    if report.get("status") != "pass":
        failures.append("fake adapter report did not pass")
    serialized = json.dumps(report, sort_keys=True)
    if "AM_SYNTHETIC_SECRET_" in serialized:
        failures.append("fake adapter report exposed a synthetic secret")
    if report.get("product_adapter_claim") != "none":
        failures.append("fake adapter report claimed product implementation")
    if report.get("platform_support_claim") != "none":
        failures.append("fake adapter report claimed platform support")
    if report != build_report(root):
        failures.append("fake adapter report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    output = root / REPORT_PATH.relative_to(ROOT)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(build_report(root), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read fake adapter report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        failures = check_report()
    except (OSError, ValueError) as error:
        print(f"fake adapter evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"fake adapter evidence failed: {failure}", file=sys.stderr)
        return 1
    print("deterministic fake model, tool, runtime, connector, clock, secret, and crash adapters validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
