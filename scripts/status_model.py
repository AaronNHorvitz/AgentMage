#!/usr/bin/env python3
"""Validate AgentMage's current, non-historical product status."""

from __future__ import annotations

import json
import hashlib
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
STATUS_PATH = ROOT / "architecture" / "status-model.json"
MATRIX_PATH = ROOT / "architecture" / "language-build-matrix.json"

EXPECTED_DIMENSIONS = {
    "lifecycle": (
        "planned",
        "designed",
        "scaffolded",
        "implemented",
        "integrated",
        "packaged",
        "shipped",
    ),
    "verification": (
        "not-run",
        "contract-tested",
        "native-tested",
        "release-verified",
    ),
    "disposition": ("active", "blocked", "rejected", "superseded"),
    "support": ("unsupported-pre-release", "supported", "end-of-support"),
}
EXPECTED_TRANSITIONS = {
    "lifecycle": {
        "planned": ("designed",),
        "designed": ("scaffolded",),
        "scaffolded": ("implemented",),
        "implemented": ("integrated",),
        "integrated": ("packaged",),
        "packaged": ("shipped",),
        "shipped": (),
    },
    "verification": {
        "not-run": ("contract-tested",),
        "contract-tested": ("native-tested",),
        "native-tested": ("release-verified",),
        "release-verified": (),
    },
    "disposition": {
        "active": ("blocked", "rejected", "superseded"),
        "blocked": ("active", "rejected", "superseded"),
        "rejected": ("superseded",),
        "superseded": (),
    },
    "support": {
        "unsupported-pre-release": ("supported",),
        "supported": ("end-of-support",),
        "end-of-support": (),
    },
}
STATUS_FIELDS = {
    "lifecycle": "lifecycle_status",
    "verification": "verification_status",
    "disposition": "disposition_status",
    "support": "support_status",
}
EXPECTED_CURRENT_PRODUCT = {
    "lifecycle_status": "scaffolded",
    "verification_status": "not-run",
    "disposition_status": "active",
    "support_status": "unsupported-pre-release",
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
EXPECTED_PLATFORM_STATES = {
    "macos-arm64": ("scaffolded", "not-run", "blocked", "retained-post-ga"),
    "fedora-x86_64": (
        "scaffolded",
        "contract-tested",
        "active",
        "first-ga-required",
    ),
    "ubuntu-x86_64": ("scaffolded", "not-run", "active", "first-ga-required"),
    "windows-x86_64": ("scaffolded", "not-run", "blocked", "first-ga-required"),
}
EXPECTED_MODEL_DISPOSITIONS = {
    "gemma-4-e4b-it": (
        "model-profiles/candidates/gemma-4-e4b/feasibility-disposition.json",
        "REJECTED",
    ),
    "gemma-4-12b-unified-it": (
        "model-profiles/candidates/gemma-4-12b-unified/feasibility-disposition.json",
        "REJECTED",
    ),
}
DEMO_ACCEPTANCE_REPORTS = {
    "demo-browser-acceptance.json": "agentmage_local_demo_browser_acceptance",
    "demo-offline-acceptance.json": "agentmage_scoped_offline_demo_acceptance",
    "demo-restarted-browser-acceptance.json": "agentmage_local_demo_browser_acceptance",
    "demo-restart-acceptance.json": "agentmage_demo_stop_restart_acceptance",
}
DEMO_BROWSER_CASES = {
    "browser launch and real model ready",
    "multi-document admission and unsupported input reasons",
    "real inference through UI with checked source citation",
    "follow-up and second source grounding",
    "unanswerable question has honest insufficient evidence",
    "context overflow fails before generation without silent truncation",
    "cancel generation then recover with real model answer",
    "model-unavailable error and recovery through interface",
    "bounded six-turn conversation and explicit new-conversation recovery",
    "folder boundary symlink traversal and unsupported/invalid inputs",
    "privileged endpoints reject missing token wrong origin and DNS rebinding host",
}


def _validate_demo_native_evidence(root: Path, failures: list[str]) -> None:
    """Require actual successful, current application acceptance for this demo."""
    for name, record_type in DEMO_ACCEPTANCE_REPORTS.items():
        path = root / "docs" / "verification" / name
        try:
            report = load_json(path)
        except (OSError, ValueError) as error:
            failures.append(f"demo native acceptance report unavailable: {name}: {error}")
            continue
        if not isinstance(report, dict) or report.get("record_type") != record_type:
            failures.append(f"demo native acceptance report identity differs: {name}")
            continue
        bindings = report.get("source_sha256")
        if not isinstance(bindings, dict) or not bindings:
            failures.append(f"demo native acceptance lacks source bindings: {name}")
        else:
            required_inputs = {"scripts/demo_smoke.py"} if name == "demo-restart-acceptance.json" else {
                "scripts/demo.py", "shells/host/src/demo_documents.rs", "demo/model.json",
                "supply-chain/sbom.cdx.json", "supply-chain/dependency-provenance.json",
                "supply-chain/dependency-hashes.sha256",
            }
            if "browser" in name:
                required_inputs |= {"scripts/demo_browser_smoke.mjs", "demo/web/app.js"}
            elif name == "demo-offline-acceptance.json":
                required_inputs.add("scripts/demo_offline_smoke.py")
            if not required_inputs.issubset(bindings):
                failures.append(f"demo native acceptance source binding coverage is incomplete: {name}")
            for relative, expected in bindings.items():
                source = _safe_evidence_path(relative, root)
                try:
                    actual = hashlib.sha256(source.read_bytes()).hexdigest() if source else None
                except OSError:
                    actual = None
                if not isinstance(expected, str) or actual != expected:
                    failures.append(f"demo native acceptance input is stale or unavailable: {name}: {relative}")
        if "browser" in name:
            tests = report.get("tests")
            if not isinstance(tests, list) or not tests or any(
                not isinstance(test, dict) or test.get("passed") is not True for test in tests
            ):
                failures.append(f"demo browser acceptance has failed or missing cases: {name}")
                tests = []
            case_names = {test.get("name") for test in tests if isinstance(test, dict)}
            required = DEMO_BROWSER_CASES if name == "demo-browser-acceptance.json" else {
                "browser launch and real model ready",
                "multi-document admission and unsupported input reasons",
                "real inference through UI with checked source citation",
            }
            if not required.issubset(case_names):
                failures.append(f"demo browser acceptance case coverage is incomplete: {name}")
            interactions = report.get("real_model_interactions")
            if not isinstance(interactions, list) or not interactions or not any(
                isinstance(item, dict) and item.get("answer") and item.get("citations")
                and item.get("insufficient_evidence") is False for item in interactions
            ):
                failures.append(f"demo browser acceptance lacks cited real application inference: {name}")
            if not isinstance(interactions, list):
                interactions = []
            if name == "demo-browser-acceptance.json" and not any(
                isinstance(item, dict) and item.get("insufficient_evidence") is True
                and item.get("citations") == [] for item in interactions
            ):
                failures.append("demo browser acceptance lacks honest insufficient-evidence inference")
        elif report.get("passed") is not True:
            failures.append(f"demo native acceptance failed: {name}")
        if name == "demo-offline-acceptance.json":
            network = report.get("network", {})
            result = report.get("real_application_result", {})
            if not isinstance(network, dict) or network.get("external_connect_blocked") is not True:
                failures.append("demo native acceptance lacks scoped external-network denial")
            if not isinstance(result, dict) or not result.get("answer") or not result.get("citations"):
                failures.append("demo native acceptance lacks real offline application inference")
        if name == "demo-restart-acceptance.json" and (
            report.get("acceptance_reports") != [
                "docs/verification/demo-browser-acceptance.json",
                "docs/verification/demo-offline-acceptance.json",
                "docs/verification/demo-restarted-browser-acceptance.json",
            ] or not isinstance(report.get("final_running_pid"), int) or report["final_running_pid"] <= 0
        ):
            failures.append("demo native acceptance lacks completed stop/restart evidence")


def _validate_demo_milestones(model: dict[str, Any], root: Path, failures: list[str]) -> None:
    records = _index_records(model.get("demo_milestones"), "demo milestone", failures)
    if set(records) != {"fedora-local-document-qa"}:
        failures.append("demo milestone set must bind the owner-delegated Fedora document QA scope")
    for record in records.values():
        _validate_record(record, "demo milestone", model, root, failures)
        for field, expected in {
            "decision_id": "ADR-0053", "platform_id": "fedora-x86_64",
            "inference_local_only": True, "automatic_compaction_enabled": False,
            "production_qualification": False,
            "supported_formats": [".markdown", ".md", ".txt"],
            "model_configuration_path": "demo/model.json",
        }.items():
            if record.get(field) != expected:
                failures.append(f"demo milestone {field} differs from its accepted scope")
        try:
            configuration = load_json(root / "demo" / "model.json")
            for field in ("model_sha256", "runtime_release", "runtime_backend", "quantization"):
                if record.get(field) != configuration.get(field):
                    failures.append(f"demo milestone selected-model binding differs: {field}")
        except (OSError, ValueError) as error:
            failures.append(f"demo milestone model configuration unavailable: {error}")
        if record.get("support_status") != "unsupported-pre-release" or record.get("verification_status") == "release-verified":
            failures.append("demo milestone cannot claim production support or release verification")
        if record.get("verification_status") == "native-tested":
            if record.get("acceptance_status") != "verified-local-demo":
                failures.append("demo native-tested status requires verified-local-demo acceptance")
            _validate_demo_native_evidence(root, failures)
        elif record.get("acceptance_status") != "pending-real-browser-offline-restart":
            failures.append("demo acceptance must remain pending before actual native verification")


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_status_model(path: Path = STATUS_PATH) -> dict[str, Any]:
    return load_json(path)


def transition_is_legal(
    model: dict[str, Any], dimension: str, current: str, proposed: str
) -> bool:
    """Return whether a current record may retain or advance its exact state."""

    if current == proposed:
        return True
    transitions = model.get("legal_transitions", {}).get(dimension, {})
    return proposed in transitions.get(current, [])


def _index_records(records: Any, label: str, failures: list[str]) -> dict[str, Any]:
    if not isinstance(records, list):
        failures.append(f"{label} must be an array")
        return {}
    indexed: dict[str, Any] = {}
    for record in records:
        if not isinstance(record, dict) or not isinstance(record.get("id"), str):
            failures.append(f"every {label} record must have a string id")
            continue
        record_id = record["id"]
        if record_id in indexed:
            failures.append(f"duplicate {label} id: {record_id}")
        indexed[record_id] = record
    return indexed


def _validate_dimensions(model: dict[str, Any], failures: list[str]) -> None:
    dimensions = model.get("dimensions")
    if not isinstance(dimensions, dict):
        failures.append("dimensions must be an object")
        return
    if set(dimensions) != set(EXPECTED_DIMENSIONS):
        failures.append("status dimensions do not match Decision 0012")
    for name, values in EXPECTED_DIMENSIONS.items():
        dimension = dimensions.get(name, {})
        if tuple(dimension.get("values", [])) != values:
            failures.append(f"{name} values do not match Decision 0012")
        if name in ("lifecycle", "verification", "support"):
            if tuple(dimension.get("promotion_order", [])) != values:
                failures.append(f"{name} promotion order is incomplete or reordered")

    transitions = model.get("legal_transitions")
    if not isinstance(transitions, dict):
        failures.append("legal_transitions must be an object")
        return
    for dimension, expected in EXPECTED_TRANSITIONS.items():
        actual = transitions.get(dimension)
        if not isinstance(actual, dict):
            failures.append(f"missing legal transitions for {dimension}")
            continue
        normalized = {key: tuple(value) for key, value in actual.items()}
        if normalized != expected:
            failures.append(f"{dimension} legal transitions do not match Decision 0012")


def _safe_evidence_path(path_text: Any, root: Path) -> Path | None:
    if not isinstance(path_text, str) or not path_text or Path(path_text).is_absolute():
        return None
    candidate = (root / path_text).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError:
        return None
    return candidate


def _validate_record(
    record: dict[str, Any], label: str, model: dict[str, Any], root: Path, failures: list[str]
) -> None:
    record_id = record.get("id", "<unknown>")
    requirements = model.get("evidence_requirements", {})
    evidence = record.get("evidence_basis")
    if not isinstance(evidence, list) or not evidence:
        failures.append(f"{label} {record_id} must have an evidence basis")
        evidence = []

    evidence_kinds: set[str] = set()
    evidence_paths: set[str] = set()
    for item in evidence:
        if not isinstance(item, dict) or not isinstance(item.get("kind"), str):
            failures.append(f"{label} {record_id} has malformed evidence")
            continue
        evidence_kinds.add(item["kind"])
        path_text = item.get("path")
        candidate = _safe_evidence_path(path_text, root)
        if candidate is None:
            failures.append(f"{label} {record_id} has an unsafe evidence path")
            continue
        if path_text in evidence_paths:
            failures.append(f"{label} {record_id} repeats evidence path {path_text}")
        evidence_paths.add(path_text)
        if not candidate.is_file():
            failures.append(f"{label} {record_id} evidence does not exist: {path_text}")

    for dimension, field in STATUS_FIELDS.items():
        value = record.get(field)
        if value not in EXPECTED_DIMENSIONS[dimension]:
            failures.append(f"{label} {record_id} has invalid {field}: {value}")
            continue
        required_kind = requirements.get(dimension, {}).get(value)
        if required_kind and required_kind not in evidence_kinds:
            failures.append(
                f"{label} {record_id} status {value} requires {required_kind} evidence"
            )

    lifecycle = record.get("lifecycle_status")
    verification = record.get("verification_status")
    disposition = record.get("disposition_status")
    support = record.get("support_status")
    if lifecycle in ("planned", "designed", "scaffolded") and verification == "release-verified":
        failures.append(f"{label} {record_id} cannot be release-verified before integration")
    if support == "supported" and (
        lifecycle != "shipped"
        or verification != "release-verified"
        or disposition != "active"
    ):
        failures.append(f"{label} {record_id} cannot be supported without a verified shipment")


def _validate_scope(model: dict[str, Any], root: Path, failures: list[str]) -> None:
    scope = model.get("scope_control")
    if not isinstance(scope, dict):
        failures.append("scope_control must be an object")
        return
    for field in (
        "stabilization_active",
        "new_capability_families_frozen",
        "original_roadmap_paused",
    ):
        if scope.get(field) is not False:
            failures.append(f"scope control {field} must remain false after Decision 0021")
    if scope.get("exception_requires_explicit_approval") is not True:
        failures.append("scope control exception_requires_explicit_approval must remain true")

    tasks = (root / "TASKS.md").read_text(encoding="utf-8")
    registry = load_json(root / "requirements" / "registry.json")
    actual = {
        "accepted_epics": len(
            re.findall(r"^## \[[ xX]\] Epic \d+", tasks, re.MULTILINE)
        ),
        "accepted_foundational_runtime_epics": len(
            re.findall(
                r"^## \[[ xX]\] Foundational Runtime Epic F\d+ - ",
                tasks,
                re.MULTILINE,
            )
        ),
        "accepted_sprints": len(
            re.findall(r"^### \[[ xX]\] Sprint \d+", tasks, re.MULTILINE)
        ),
        "stable_requirements": len(registry.get("requirements", [])),
    }
    expected = {
        "accepted_epics": 17,
        "accepted_foundational_runtime_epics": 4,
        "accepted_sprints": 169,
        "stable_requirements": 294,
    }
    for field, count in expected.items():
        if scope.get(field) != count:
            failures.append(f"scope model {field} must equal {count}")
        if actual[field] != count:
            failures.append(f"repository {field} changed from {count} to {actual[field]}")

    required_impact = {
        "architecture",
        "authority",
        "security",
        "privacy",
        "platforms",
        "tests",
        "evidence",
        "packaging",
        "support",
        "recovery",
        "release-gates",
    }
    if set(scope.get("exception_impact_fields", [])) != required_impact:
        failures.append("scope exception impact fields are incomplete")


def _validate_documents(
    model: dict[str, Any], root: Path, documents: dict[str, str] | None, failures: list[str]
) -> None:
    contract = model.get("documentation_contract")
    if not isinstance(contract, dict):
        failures.append("documentation_contract must be an object")
        return
    document_paths = contract.get("documents")
    markers = contract.get("required_markers")
    if not isinstance(document_paths, list) or not isinstance(markers, list):
        failures.append("documentation contract documents and markers must be arrays")
        return
    loaded = documents or {
        path: (root / path).read_text(encoding="utf-8") for path in document_paths
    }
    for path in document_paths:
        text = loaded.get(path)
        if text is None:
            failures.append(f"missing status document input: {path}")
            continue
        for marker in markers:
            if marker not in text:
                failures.append(f"{path} is missing current-status marker: {marker}")

    readme = loaded.get("README.md", "")
    inventory = loaded.get("Agent-Scaffolding-Inventory.md", "")
    if "| First enabled model |" in readme:
        failures.append("README.md must not describe a first enabled model")
    if "v0.1 targets exactly one enabled local profile" in inventory:
        failures.append("inventory must not describe a rejected candidate as enabled")


def _validate_models(
    records: dict[str, Any], root: Path, failures: list[str]
) -> None:
    missing_historical = sorted(set(EXPECTED_MODEL_DISPOSITIONS) - set(records))
    if missing_historical:
        failures.append(
            "current model set must preserve evaluated historical candidates: "
            + ", ".join(missing_historical)
        )
    for model_id, record in records.items():
        if record.get("enabled") is not False:
            failures.append(f"model {model_id} must remain disabled before admission")
        if record.get("automatic_fallback") is not False:
            failures.append(f"model {model_id} cannot become an automatic fallback")
    for model_id, (path, expected_disposition) in EXPECTED_MODEL_DISPOSITIONS.items():
        record = records.get(model_id, {})
        if record.get("enabled") is not False:
            failures.append(f"rejected model {model_id} must remain disabled")
        if record.get("automatic_fallback") is not False:
            failures.append(f"model {model_id} cannot be an automatic fallback")
        if record.get("disposition_status") != "rejected":
            failures.append(f"model {model_id} current disposition must be rejected")
        disposition = load_json(root / path)
        if disposition.get("decision", {}).get("status") != expected_disposition:
            failures.append(f"model {model_id} disposition record does not remain rejected")
        if disposition.get("decision", {}).get("candidate_enabled") is not False:
            failures.append(f"model {model_id} historical disposition enables the candidate")


def _validate_matrix_references(
    matrix: dict[str, Any],
    components: dict[str, Any],
    platforms: dict[str, Any],
    failures: list[str],
) -> None:
    matrix_components = _index_records(
        matrix.get("product_components"), "matrix component", failures
    )
    matrix_platforms = _index_records(matrix.get("platform_targets"), "matrix platform", failures)
    for record_id, record in matrix_components.items():
        expected = f"architecture/status-model.json#component={record_id}"
        if record_id not in components or record.get("status_ref") != expected:
            failures.append(f"matrix component {record_id} has a stale status reference")
    for record_id, record in matrix_platforms.items():
        expected = f"architecture/status-model.json#platform={record_id}"
        if record_id not in platforms or record.get("status_ref") != expected:
            failures.append(f"matrix platform {record_id} has a stale status reference")


def validate_status_model(
    model: Any,
    matrix: dict[str, Any] | None = None,
    documents: dict[str, str] | None = None,
    root: Path = ROOT,
) -> list[str]:
    failures: list[str] = []
    if not isinstance(model, dict):
        return ["status model must be an object"]
    if model.get("schema_version") != 1:
        failures.append("status model schema_version must equal 1")
    if model.get("decision_id") != "ADR-0012" or model.get("status") != "accepted":
        failures.append("status model must bind accepted Decision 0012")
    if model.get("amendment_decision_ids") != [
        "ADR-0043",
        "ADR-0044",
        "ADR-0045",
        "ADR-0046",
        "ADR-0047",
        "ADR-0048",
        "ADR-0052",
        "ADR-0053",
    ]:
        failures.append("status model engineering-runtime amendments are incomplete")
    if (
        model.get("reference_contract")
        != "architecture/status-model.json#<collection>=<record-id>"
    ):
        failures.append("status model reference contract is missing or unsupported")

    _validate_dimensions(model, failures)
    components = _index_records(model.get("components"), "component status", failures)
    platforms = _index_records(model.get("platforms"), "platform status", failures)
    models = _index_records(model.get("models"), "model status", failures)
    current_product = model.get("current_product")
    if not isinstance(current_product, dict):
        failures.append("current_product must be an object")
    else:
        for field, value in EXPECTED_CURRENT_PRODUCT.items():
            if current_product.get(field) != value:
                failures.append(f"current product {field} must equal {value!r}")
        _validate_record(current_product, "product", model, root, failures)

    for label, records in (
        ("component", components),
        ("platform", platforms),
        ("model", models),
    ):
        for record in records.values():
            _validate_record(record, label, model, root, failures)

    if set(platforms) != set(EXPECTED_PLATFORM_STATES):
        failures.append("platform status set does not match Decision 0012")
    for platform_id, expected in EXPECTED_PLATFORM_STATES.items():
        record = platforms.get(platform_id, {})
        actual = (
            record.get("lifecycle_status"),
            record.get("verification_status"),
            record.get("disposition_status"),
            record.get("release_lane"),
        )
        if actual != expected:
            failures.append(f"platform {platform_id} status does not match current truth")

    _validate_models(models, root, failures)
    _validate_demo_milestones(model, root, failures)
    _validate_scope(model, root, failures)
    _validate_documents(model, root, documents, failures)
    selected_matrix = matrix if matrix is not None else load_json(MATRIX_PATH)
    _validate_matrix_references(selected_matrix, components, platforms, failures)
    return failures


def main() -> int:
    try:
        model = load_status_model()
        matrix = load_json(MATRIX_PATH)
    except (OSError, json.JSONDecodeError) as error:
        print(f"status model validation failed: {error}", file=sys.stderr)
        return 1

    failures = validate_status_model(model, matrix=matrix)
    if failures:
        for failure in failures:
            print(f"status model validation failed: {failure}", file=sys.stderr)
        return 1

    print("status model validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
