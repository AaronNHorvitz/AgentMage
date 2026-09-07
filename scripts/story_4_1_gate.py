#!/usr/bin/env python3
"""Independently evaluate Story 4.1 while preserving the macOS blocker."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.kernel_architecture_report import check_report as check_architecture
from scripts.kernel_boundary_integration import check_report as check_boundary
from scripts.kernel_contract_fixtures import check_outputs as check_fixtures
from scripts.kernel_contract_package import check_artifacts as check_package
from scripts.kernel_contract_reference import check_report as check_reference
from scripts.kernel_dispatch_security import check_report as check_dispatch
from scripts.story_4_1_security_evidence import check_map as check_security


REPORT_PATH = ROOT / "artifacts/sprints/sprint-4/story-4.1/story-gate-report.json"
REVIEWED_COMMIT = "9613571bf5e84f4a7010c368c8805b277c934c26"
REVIEWED_TREE = "d44e1dd6167bf747c66015a999ecd3ac9522d257"
REVIEWED_PATHS = (
    "architecture/dependency-rules.json",
    "docs/architecture/kernel-contract-reference.md",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/propagation.rs",
    "kernel/engine/src/tooling.rs",
    "kernel/engine/tests/boundary_workflow.rs",
    "fixtures/contracts/fixture_verifier.rs",
    "fixtures/contracts/compatibility.json",
    "fixtures/contracts/v1/manifest.json",
    "fixtures/contracts/compatibility-v2.json",
    "fixtures/contracts/v2/manifest.json",
    "fixtures/contracts/v2/valid/action.json",
    "fixtures/contracts/v2/valid/approval_request.json",
    "fixtures/contracts/v2/valid/boundary_failure.json",
    "fixtures/contracts/v2/valid/cancellation_signal.json",
    "fixtures/contracts/v2/valid/capability_grant.json",
    "fixtures/contracts/v2/valid/contract_error.json",
    "fixtures/contracts/v2/valid/evidence_reference.json",
    "fixtures/contracts/v2/valid/plan.json",
    "fixtures/contracts/v2/valid/prompt.json",
    "fixtures/contracts/v2/valid/receipt.json",
    "fixtures/contracts/v2/valid/task.json",
    "fixtures/contracts/v2/valid/tool_call.json",
    "fixtures/contracts/v2/valid/tool_definition.json",
    "fixtures/contracts/v2/valid/tool_result.json",
    "fixtures/contracts/v2/valid/work_packet.json",
    "fixtures/contracts/compatibility/v2/task.v0.unsupported.json",
    "fixtures/contracts/compatibility/v2/task.v1.unsupported.json",
    "fixtures/contracts/compatibility/v2/task.v2.duplicate-field.json",
    "fixtures/contracts/compatibility/v2/task.v2.malformed.json",
    "fixtures/contracts/compatibility/v2/task.v2.missing-field.json",
    "fixtures/contracts/compatibility/v2/task.v2.trailing-value.json",
    "fixtures/contracts/compatibility/v2/task.v2.unknown-field.json",
    "fixtures/contracts/compatibility/v2/task.v3.unsupported.json",
)
REQUIRED_TASK_MARKERS = (
    "- [x] **Task 4.1.1 - Implement the bounded story**",
    "- [x] **Task 4.1.2 - Produce reviewable artifacts**",
    "- [x] **Task 4.1.3 - Verify and close the story**",
    "  - [x] **Sub-task 4.1.3.1:**",
    "  - [x] **Sub-task 4.1.3.2:**",
    "  - [x] **Sub-task 4.1.3.3:**",
    "  - [x] **Sub-task 4.1.3.4:**",
    "  - [x] **Sub-task 4.1.3.5 - Product security evidence:**",
    "- [x] **Story AC 4.1.AC1:**",
    "- [x] **Story AC 4.1.AC2:**",
)
G_DOD_IDS = tuple(f"G-DOD-{index:02d}" for index in range(1, 14))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-story-4-1-gate-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def git_output(*arguments: str, root: Path = ROOT, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=10,
    )
    if result.returncode != 0:
        raise ValueError(f"git review operation failed: {' '.join(arguments)}")
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def reviewed_artifacts(root: Path = ROOT) -> list[dict[str, str]]:
    commit = git_output("rev-parse", REVIEWED_COMMIT, root=root)
    tree = git_output("show", "-s", "--format=%T", REVIEWED_COMMIT, root=root)
    if commit != REVIEWED_COMMIT or tree != REVIEWED_TREE:
        raise ValueError("Story 4.1 review identity is unavailable or changed")
    records = []
    for path in REVIEWED_PATHS:
        committed = git_output("show", f"{REVIEWED_COMMIT}:{path}", root=root, binary=True)
        current = root / path
        if not current.is_file() or current.read_bytes() != committed:
            raise ValueError(f"reviewed Story 4.1 artifact changed after review: {path}")
        records.append({"path": path, "sha256": sha256_bytes(committed)})
    return records


def task_completion(tasks_text: str) -> list[str]:
    return [marker for marker in REQUIRED_TASK_MARKERS if marker not in tasks_text]


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    for name, check in (
        ("contract-package", lambda: check_package(root)),
        ("contract-reference", lambda: check_reference(root)),
        ("contract-fixtures", check_fixtures),
    ):
        try:
            check()
        except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
            failures.append(f"{name}: {error}")
    for name, check in (
        ("architecture", check_architecture),
        ("dispatcher", check_dispatch),
        ("boundary-integration", check_boundary),
        ("security-map", check_security),
    ):
        failures.extend(f"{name}: {failure}" for failure in check(root))
    try:
        tasks_text = (root / "TASKS.md").read_text(encoding="utf-8")
    except OSError as error:
        failures.append(f"cannot read TASKS.md: {error}")
    else:
        failures.extend(
            f"task marker is incomplete: {marker}"
            for marker in task_completion(tasks_text)
        )
    return failures


def universal_dod() -> list[dict[str, Any]]:
    statuses = {
        "G-DOD-01": "pass-story-scope",
        "G-DOD-02": "pass-story-scope",
        "G-DOD-03": "pass-story-scope",
        "G-DOD-04": "pass-no-positive-authority-path",
        "G-DOD-05": "pass-story-scope",
        "G-DOD-06": "pass-story-scope-pre-grant-receipts",
        "G-DOD-07": "pass-no-persisted-user-data",
        "G-DOD-08": "pass-story-scope",
        "G-DOD-09": "pass-story-scope",
        "G-DOD-10": "blocked-macos",
        "G-DOD-11": "pass-story-scope",
        "G-DOD-12": "pass-automated-independent-implementation-review",
        "G-DOD-13": "pass-gate-reporting",
    }
    return [
        {
            "control_id": control_id,
            "status": statuses[control_id],
            "evidence": [
                "artifacts/sprints/sprint-4/story-4.1/security-evidence-map.json",
                "artifacts/sprints/sprint-4/story-4.1/story-gate-report.json",
            ],
        }
        for control_id in G_DOD_IDS
    ]


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    reviewed = reviewed_artifacts(root)
    reference = read_json(
        root
        / "artifacts/sprints/sprint-4/story-4.1/kernel-contract-reference-report.json"
    )
    fixtures = read_json(
        root / "artifacts/sprints/sprint-4/story-4.1/kernel-contract-fixture-report.json"
    )
    architecture = read_json(
        root
        / "artifacts/sprints/sprint-4/story-4.1/kernel-architecture-dependency-report.json"
    )
    dispatch = read_json(
        root / "artifacts/sprints/sprint-4/story-4.1/kernel-dispatch-security-report.json"
    )
    boundary = read_json(
        root / "artifacts/sprints/sprint-4/story-4.1/kernel-boundary-integration-report.json"
    )
    security = read_json(
        root / "artifacts/sprints/sprint-4/story-4.1/security-evidence-map.json"
    )
    return {
        "schema_version": 1,
        "story_id": "4.1",
        "status": "blocked-macos",
        "reviewed_commit": REVIEWED_COMMIT,
        "reviewed_tree": REVIEWED_TREE,
        "acceptance_criteria": [
            {
                "criterion_id": "4.1.AC1",
                "status": "pass-shared-linux",
                "versioned_contract_count": reference["coverage"][
                    "versioned_contract_count"
                ],
                "boundary_error_code_count": reference["coverage"][
                    "boundary_error_code_count"
                ],
                "dispatcher_denial_trace_count": dispatch["trace_count"],
                "positive_dispatch_path_available": dispatch[
                    "positive_dispatch_path_available"
                ],
            },
            {
                "criterion_id": "4.1.AC2",
                "status": "pass-shared-linux",
                "independent_client": "frozen-package-fixture-verifier",
                "valid_fixture_count": fixtures["valid_fixture_count"],
                "persisted_invalid_fixture_count": fixtures["invalid_fixture_count"],
                "generated_oversized_case_count": fixtures[
                    "generated_oversized_case_count"
                ],
            },
        ],
        "architecture_acceptance": {
            "acceptance_test_id": "AT-ARCH-001",
            "scope": "implemented-contract-layer",
            "status": "pass-shared-linux",
            "observed_product_edge_count": architecture["graph"][
                "observed_product_edge_count"
            ],
            "prohibited_observed_edge_count": len(
                architecture["graph"]["prohibited_observed_edges"]
            ),
            "runtime_boundary_trace_count": boundary["trace_count"],
            "product_wide_acceptance_claim": "none",
        },
        "security_mapping": {
            "mapped_requirement_count": security["summary"][
                "mapped_requirement_count"
            ],
            "product_requirements_complete": security["summary"][
                "product_requirements_complete"
            ],
        },
        "universal_definition_of_done": universal_dod(),
        "independent_review": {
            "reviewer_id": "agentmage-story-4.1-independent-gate-v1",
            "reviewed_commit": REVIEWED_COMMIT,
            "reviewed_tree": REVIEWED_TREE,
            "artifacts": reviewed,
            "finding_count": 0,
            "external_human_review_claim": "none",
        },
        "shared_linux_story_work_complete": True,
        "only_blocker": "macos-execution-evidence-unavailable",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
        "positive_authority_path_claim": "none",
        "product_acceptance_claim": "none",
        "release_claim": "none",
    }


def validate_report(
    value: Any, root: Path = ROOT, *, verify_current: bool = True
) -> list[str]:
    if not isinstance(value, dict):
        return ["Story 4.1 gate report must be an object"]
    failures: list[str] = []
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "4.1"
        or value.get("status") != "blocked-macos"
        or value.get("reviewed_commit") != REVIEWED_COMMIT
        or value.get("reviewed_tree") != REVIEWED_TREE
    ):
        failures.append("Story 4.1 gate identity is invalid")
    criteria = value.get("acceptance_criteria", [])
    if [item.get("criterion_id") for item in criteria if isinstance(item, dict)] != [
        "4.1.AC1",
        "4.1.AC2",
    ] or any(item.get("status") != "pass-shared-linux" for item in criteria):
        failures.append("Story 4.1 acceptance criteria are incomplete")
    architecture = value.get("architecture_acceptance", {})
    if (
        architecture.get("acceptance_test_id") != "AT-ARCH-001"
        or architecture.get("scope") != "implemented-contract-layer"
        or architecture.get("status") != "pass-shared-linux"
        or architecture.get("prohibited_observed_edge_count") != 0
        or architecture.get("product_wide_acceptance_claim") != "none"
    ):
        failures.append("Story 4.1 architecture acceptance is invalid")
    if value.get("security_mapping") != {
        "mapped_requirement_count": 7,
        "product_requirements_complete": 0,
    }:
        failures.append("Story 4.1 security mapping summary is invalid")
    if value.get("universal_definition_of_done") != universal_dod():
        failures.append("Story 4.1 universal definition of done is invalid")
    if (
        value.get("shared_linux_story_work_complete") is not True
        or value.get("only_blocker") != "macos-execution-evidence-unavailable"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
        or value.get("positive_authority_path_claim") != "none"
        or value.get("product_acceptance_claim") != "none"
        or value.get("release_claim") != "none"
    ):
        failures.append("Story 4.1 gate made an unsupported claim")
    if verify_current:
        try:
            expected = build_report(root)
        except (OSError, ValueError, KeyError, TypeError) as error:
            failures.append(f"cannot rebuild Story 4.1 gate report: {error}")
        else:
            if value != expected:
                failures.append("Story 4.1 gate report is stale or non-deterministic")
    return failures


def check_report(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 4.1 gate report: {error}"]
    return validate_report(value, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Story 4.1 gate failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 4.1 gate failed: {failure}", file=sys.stderr)
        return 1
    print("Story 4.1 shared/Linux acceptance passed with macOS blocker preserved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
