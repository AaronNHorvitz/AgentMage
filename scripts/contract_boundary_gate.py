#!/usr/bin/env python3
"""Build and validate Task 1.2.5.1 contract-boundary evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Final

_IMPORT_ROOT = Path(__file__).resolve().parents[1]
if str(_IMPORT_ROOT) not in sys.path:
    sys.path.insert(0, str(_IMPORT_ROOT))

from scripts.dependency_dispositions import load_record as load_dispositions
from scripts.dependency_dispositions import validate_record as validate_dispositions
from scripts.dependency_rules import load_rules, validate_rules
from scripts.module_inventory import load_inventory
from scripts.parser_ocr_placement import load_record as load_placement
from scripts.parser_ocr_placement import validate_record as validate_placement
from scripts.runtime_ownership import load_inventory as load_ownership_inventory
from scripts.runtime_ownership import load_ownership, validate_ownership
from scripts.schema_evolution_plan import load_plan, validate_plan
from scripts.status_model import load_json, load_status_model, validate_status_model
from scripts.vscode_api_surfaces import load_surfaces, validate_surfaces


ROOT: Final = _IMPORT_ROOT
REPORT_PATH: Final = (
    ROOT / "artifacts" / "sprints" / "sprint-1" / "story-1.2" / "contract-boundary-report.json"
)
SOURCE_PATHS: Final = [
    "TASKS.md",
    "architecture/dependency-rules.json",
    "architecture/module-inventory.json",
    "architecture/runtime-ownership.json",
    "architecture/vscode-api-surfaces.json",
    "architecture/status-model.json",
    "architecture/dependency-dispositions.json",
    "architecture/schema-evolution-and-rollback.json",
    "architecture/parser-ocr-platform-placement.json",
    "scripts/engineering_runtime_schemas.mjs",
    "scripts/dependency_rules.py",
    "scripts/runtime_ownership.py",
    "scripts/vscode_api_surfaces.py",
    "scripts/status_model.py",
    "scripts/contract_boundary_gate.py",
    "tests/test_engineering_runtime_schemas.mjs",
    "tests/test_dependency_rules.py",
    "tests/test_runtime_ownership.py",
    "tests/test_vscode_api_surfaces.py",
    "tests/test_status_model.py",
    "tests/test_contract_boundary_gate.py",
]
CHECKS: Final = [
    {
        "id": "schema-mutation",
        "command": "node scripts/engineering_runtime_schemas.mjs && node --test tests/test_engineering_runtime_schemas.mjs",
        "result": "pass-local",
        "assertion": "closed Engineering Runtime schemas reject missing, extra, malformed, version, semantic, compatibility, and migration mutations",
    },
    {
        "id": "dependency-direction",
        "command": "python3 scripts/dependency_rules.py && python3 -m unittest tests.test_dependency_rules",
        "result": "pass-local",
        "assertion": "every prohibited reverse edge, undeclared edge, cross-layer edge, package edge, and cycle is rejected",
    },
    {
        "id": "forbidden-duplicate-owner",
        "command": "python3 scripts/runtime_ownership.py && python3 -m unittest tests.test_runtime_ownership",
        "result": "pass-local",
        "assertion": "a second context manager, store, runtime loop, policy engine, dispatcher, verifier, module owner, or role owner is rejected",
    },
    {
        "id": "api-surface",
        "command": "python3 scripts/vscode_api_surfaces.py && python3 -m unittest tests.test_vscode_api_surfaces",
        "result": "pass-local",
        "assertion": "supported-path, experimental-channel, best-effort, degradation, and absence-disclosure mutations fail closed",
    },
    {
        "id": "capability-absence",
        "command": "python3 -m unittest tests.test_contract_boundary_gate",
        "result": "pass-local",
        "assertion": "disabled parser, OCR, MCP, model, platform, package, release, remote-placement, and fallback capabilities cannot be promoted by plan or status mutation",
    },
    {
        "id": "current-versus-planned-truth",
        "command": "python3 scripts/status_model.py && python3 -m unittest tests.test_status_model tests.test_contract_boundary_gate",
        "result": "pass-local",
        "assertion": "current product truth remains separate from accepted plans and rejects implementation, integration, support, platform, model, package, and release overclaims",
    },
]
PRODUCT_TRUTH: Final = {
    "local_contract_gate_passed": True,
    "capability_enabled": False,
    "native_platform_evidence": False,
    "external_review_complete": False,
    "platform_support_claimed": False,
    "release_readiness": False,
}


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _schema_bundle_sha() -> str:
    digest = hashlib.sha256()
    for path in sorted((ROOT / "schemas" / "engineering-runtime").glob("*.schema.json")):
        relative = path.relative_to(ROOT).as_posix().encode("utf-8")
        content = path.read_bytes()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def validate_capability_absence(
    status: Any,
    ownership: Any,
    dispositions: Any,
    placement: Any,
) -> list[str]:
    failures: list[str] = []
    if not all(isinstance(item, dict) for item in (status, ownership, dispositions, placement)):
        return ["capability-absence: truth inputs must be objects"]
    mcp = next((item for item in ownership.get("layers", []) if item.get("id") == "mcp"), {})
    if mcp.get("implemented") is not False or mcp.get("modules") != []:
        failures.append("capability-absence: MCP must remain unimplemented with no modules")
    features = placement.get("package_features", {})
    if features.get("default_enabled") != [] or features.get("ocr_status") != "deferred-unavailable":
        failures.append("capability-absence: parser/OCR package features were promoted")
    for key, value in placement.get("product_truth", {}).items():
        if key != "placement_decided" and value is not False:
            failures.append(f"capability-absence: placement product truth widened: {key}")
    for key, value in dispositions.get("product_truth", {}).items():
        if value is not False:
            failures.append(f"capability-absence: dependency product truth widened: {key}")
    current = status.get("current_product", {})
    exact_product_truth = {
        "integrated_user_workflow": True,
        "integrated_workflow": {
            "id": "story-22.5-deterministic-repository-analysis",
            "scope": "source-level deterministic fake-model repository-analysis vertical slice",
            "evidence_path": "artifacts/sprints/sprint-22/story-22.5/vertical-slice-report.json",
        },
        "enabled_models": [],
        "supported_platforms": [],
        "released_packages": [],
        "release_gate_status": "blocked",
    }
    for key, expected in exact_product_truth.items():
        if current.get(key) != expected:
            failures.append(f"current-versus-planned-truth: current product {key} differs from bound truth")
    return failures


def validate_current_boundary(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    inventory = load_inventory()
    failures.extend(f"dependency-direction: {item}" for item in validate_rules(load_rules(), inventory))
    failures.extend(
        f"runtime-ownership: {item}"
        for item in validate_ownership(load_ownership(), load_ownership_inventory())
    )
    failures.extend(f"api-surface: {item}" for item in validate_surfaces(load_surfaces()))
    failures.extend(f"dependency-disposition: {item}" for item in validate_dispositions(load_dispositions(), root))
    failures.extend(f"schema-evolution: {item}" for item in validate_plan(load_plan(), root))
    placement = load_placement()
    failures.extend(f"placement: {item}" for item in validate_placement(placement, root))
    status = load_status_model()
    matrix = load_json(root / "architecture" / "language-build-matrix.json")
    failures.extend(f"status: {item}" for item in validate_status_model(status, matrix=matrix, root=root))

    failures.extend(
        validate_capability_absence(
            status,
            load_ownership(),
            load_dispositions(),
            placement,
        )
    )
    return failures


def expected_report() -> dict[str, Any]:
    sources = [
        {"path": path, "sha256": _sha(ROOT / path)}
        for path in SOURCE_PATHS
    ]
    sources.append(
        {
            "path": "schemas/engineering-runtime/*.schema.json",
            "sha256": _schema_bundle_sha(),
        }
    )
    return {
        "schema_version": 1,
        "record_type": "contract-boundary-verification-report",
        "decision_id": "ADR-0042",
        "task_id": "1.2.5.1",
        "generated_on": "2026-08-29",
        "status": "pass-local-with-external-evidence-open",
        "checks": [dict(item) for item in CHECKS],
        "sources": sources,
        "product_truth": dict(PRODUCT_TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, ensure_ascii=True) + "\n"


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["contract-boundary report is stale, incomplete, or widened"]
    return validate_current_boundary()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    failures = validate_current_boundary()
    if failures:
        for failure in failures:
            print(f"contract boundary gate failed: {failure}", file=sys.stderr)
        return 1
    expected = expected_report()
    if args.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_text(render(expected), encoding="utf-8")
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"contract boundary gate failed: {error}", file=sys.stderr)
        return 1
    failures = validate_report(actual)
    if failures:
        for failure in failures:
            print(f"contract boundary gate failed: {failure}", file=sys.stderr)
        return 1
    print("Task 1.2.5.1 contract boundary gate passed locally")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
