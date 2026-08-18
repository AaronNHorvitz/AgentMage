#!/usr/bin/env python3
"""Independently review the committed Story 23.4 coordinator boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-23/story-23.4/coordinator-boundary-review.json"
)
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
ZERO_SHA256: Final = "0" * 64

RUNTIME_LOOP: Final = "kernel/engine/src/runtime_loop.rs"
CODING_CLIENT: Final = "shells/host/src/coding_client.rs"
CLI_CLIENT: Final = "shells/host/src/cli_runtime.rs"
NATIVE_CHAT: Final = "shells/host/src/native_chat_runtime.rs"
PARITY_TESTS: Final = "shells/host/src/runtime_parity_tests.rs"
READ_TESTS: Final = "shells/host/src/runtime_read_tests.rs"
VSCODE_PROVIDER: Final = "shells/vscode/src/provider.ts"
VSCODE_TRANSPORT: Final = "shells/vscode/src/runtime_transport.ts"
ENGINE_MANIFEST: Final = "kernel/engine/Cargo.toml"
READ_MANIFEST: Final = "capabilities/read-only/Cargo.toml"
DEPENDENCY_RULES: Final = "architecture/dependency-rules.json"
MODULE_INVENTORY: Final = "architecture/module-inventory.json"
ARCHITECTURE: Final = "docs/architecture/reusable-runtime-coordinator.md"
RUNTIME_REPORT: Final = (
    "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.json"
)
RUNTIME_LOG: Final = "artifacts/sprints/sprint-23/story-23.4/runtime-evidence.log"

SOURCE_PATHS: Final = (
    RUNTIME_LOOP,
    CODING_CLIENT,
    CLI_CLIENT,
    NATIVE_CHAT,
    PARITY_TESTS,
    READ_TESTS,
    VSCODE_PROVIDER,
    VSCODE_TRANSPORT,
    ENGINE_MANIFEST,
    READ_MANIFEST,
    DEPENDENCY_RULES,
    MODULE_INVENTORY,
    ARCHITECTURE,
    RUNTIME_REPORT,
    RUNTIME_LOG,
    "scripts/dependency_rules.py",
    "scripts/effect_boundary.py",
    "scripts/runtime_coordinator_boundary_review.py",
    "tests/test_runtime_coordinator_boundary_review.py",
)

EXPECTED_CHECK_IDS: Final = (
    "coordinator-client-independent",
    "coordinator-transport-independent",
    "coordinator-no-direct-native-effect",
    "coordinator-no-direct-storage",
    "coordinator-no-mcp-shortcut",
    "model-proposal-is-inert",
    "tool-dispatch-is-grant-gated",
    "shell-is-presentation-only",
    "capability-pack-has-no-platform-edge",
    "exact-profile-has-no-hidden-fallback",
    "client-cannot-certify-success",
    "bypass-campaign-is-retained",
)

LIMITATIONS: Final = (
    "This is an independent automated source-boundary review, not an independent human review.",
    "The review proves declared source structure and retained deterministic fixtures, not an installed native client or model runtime.",
    "Dynamic loader, operating-system process, and post-package observations remain installed-platform evidence.",
    "Persistent crash/restart and physical dependency-fault evidence remain owned by later durable campaigns.",
    "Manual fuzzing remains deferred and was not executed.",
    "No release approval, model enablement, platform support, or deployment authority follows from this review.",
)


class CoordinatorReviewError(ValueError):
    """Raised when a committed coordinator review input is invalid or missing."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise CoordinatorReviewError("runtime.coordinator_review.source_revision")
    return revision


def git_blob(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
        check=False,
    )
    if result.returncode or not result.stdout:
        raise CoordinatorReviewError(
            f"runtime.coordinator_review.source_unavailable.{relative}"
        )
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {"path": path, "bytes": len(content), "sha256": sha256_bytes(content)}
        for path in SOURCE_PATHS
        for content in [git_blob(revision, path)]
    ]


def check(identifier: str, passed: bool, detail: str) -> dict[str, Any]:
    return {"check_id": identifier, "passed": passed, "detail": detail}


def committed_sources(revision: str) -> dict[str, str]:
    return {
        path: git_blob(revision, path).decode("utf-8")
        for path in SOURCE_PATHS
        if path != RUNTIME_LOG
    }


def review_checks(sources: dict[str, str]) -> list[dict[str, Any]]:
    runtime_loop = sources[RUNTIME_LOOP]
    coding_client = sources[CODING_CLIENT]
    cli_client = sources[CLI_CLIENT]
    native_chat = sources[NATIVE_CHAT]
    parity = sources[PARITY_TESTS]
    reads = sources[READ_TESTS]
    provider = sources[VSCODE_PROVIDER]
    transport = sources[VSCODE_TRANSPORT]
    engine_manifest = sources[ENGINE_MANIFEST]
    read_manifest = sources[READ_MANIFEST]
    dependency_rules = json.loads(sources[DEPENDENCY_RULES])
    module_inventory = json.loads(sources[MODULE_INVENTORY])
    architecture = sources[ARCHITECTURE]
    runtime_report = json.loads(sources[RUNTIME_REPORT])

    coordinator_client_independent = all(
        token not in runtime_loop
        for token in (
            "Visual Studio Code",
            "NativeChat",
            "InteractiveCli",
            "WorkflowCaller",
            "CodingClient",
        )
    )
    coordinator_transport_independent = all(
        token not in runtime_loop
        for token in (
            "serde_json::Value",
            "Agent Client Protocol",
            "RuntimeHostResponse",
            "RuntimeStepResponse",
        )
    )
    no_direct_native_effect = all(
        token not in runtime_loop
        for token in (
            "std::process",
            "std::fs",
            "std::net",
            "Command::new",
            "TcpStream",
            "UnixStream",
        )
    )
    no_direct_storage = all(
        token not in runtime_loop
        for token in (
            "OperationalStore",
            "rusqlite",
            "Sqlite",
            "File::open",
            "File::create",
        )
    )
    no_mcp_shortcut = all(
        token not in runtime_loop + coding_client + engine_manifest
        for token in ("Mcp", "MCP", "mcp_", "mcp-")
    )
    model_proposal_inert = all(
        marker in runtime_loop
        for marker in (
            "pub trait RuntimeModelPort",
            "Result<ModelRunResult, RuntimePortFailure>",
            "ModelProposalKind::ToolCall",
            "let receipt = ToolDispatcher::new(&self.registry).dispatch",
        )
    )
    grant_gated = all(
        marker in runtime_loop
        for marker in (
            "PreGrantDispatchDisposition::GrantRequired",
            "RuntimePermissionEvaluation::Allow",
            "RuntimePermissionEvaluation::Ask",
            "RuntimePermissionEvaluation::Deny",
            ".execute_with_correctness_events(",
        )
    ) and all(token not in runtime_loop for token in ("CapabilityGrant", "PolicyEngine"))
    shell_presentation_only = all(
        token not in coding_client + provider + transport
        for token in (
            "ToolDispatcher",
            "ToolRegistry",
            "CapabilityGrant",
            "PolicyEngine",
            "OperationalStore",
            "child_process",
            "node:fs",
            "node:net",
        )
    ) and all(
        marker in coding_client
        for marker in (
            "pub trait CodingCoordinatorPort",
            "pub trait CodingApprovalPort",
            "pub trait CodingEventSink",
        )
    )
    read_pack_no_platform = all(
        token not in read_manifest
        for token in (
            "agentmage-platform-linux",
            "agentmage-platform-macos",
            "agentmage-platform-windows",
        )
    )
    inventory_records = {
        item.get("id"): item
        for item in module_inventory.get("modules", [])
        if isinstance(item, dict)
    }
    rule_records = {
        item.get("id"): item
        for item in dependency_rules.get("module_rules", [])
        if isinstance(item, dict)
    }
    read_pack_no_platform = read_pack_no_platform and (
        inventory_records.get("capability-read-only", {}).get("category")
        == "capability-packs"
        and rule_records.get("capability-read-only", {}).get("allowed_imports")
        == ["kernel-contracts"]
    )
    exact_profile_no_fallback = all(
        marker in runtime_loop + cli_client + native_chat + provider + parity
        for marker in (
            "exact_profile",
            "model_profile",
            "model_profile.profile_id",
            "No substitute model or route was used",
            "hidden_retry",
        )
    )
    client_cannot_certify = all(
        marker in runtime_loop + coding_client + parity
        for marker in (
            "VerifierRegistry::new",
            "registry.verify(&candidate)",
            "RuntimeEventSequence::new",
            "story_50_2_read_only_and_coding_packets_are_equal_across_all_three_callers",
        )
    ) and "AgentStateKind::Success" not in coding_client
    coverage = runtime_report.get("coverage", {})
    bypass_retained = (
        runtime_report.get("status") == "pass-current-linux-source-runtime"
        and runtime_report.get("external_network_used") is False
        and runtime_report.get("manual_fuzzing_executed") is False
        and all(
            coverage.get(name) is True
            for name in (
                "client_confusion_denied",
                "effect_mediation_structure",
                "model_claim_cannot_mint_success",
                "workflow_authority_broadening_denied",
            )
        )
        and "Optional MCP adapter" in architecture
        and "A second model loop" in architecture
    )

    return [
        check(
            "coordinator-client-independent",
            coordinator_client_independent,
            "Kernel coordinator has no first-party client dependency.",
        ),
        check(
            "coordinator-transport-independent",
            coordinator_transport_independent,
            "Kernel coordinator has no UI, host-envelope, or ACP transport dependency.",
        ),
        check(
            "coordinator-no-direct-native-effect",
            no_direct_native_effect,
            "Coordinator contains no process, filesystem, or socket effect API.",
        ),
        check(
            "coordinator-no-direct-storage",
            no_direct_storage,
            "Coordinator reaches persistence only through injected narrow ports.",
        ),
        check(
            "coordinator-no-mcp-shortcut",
            no_mcp_shortcut,
            "Coordinator, client, and engine manifest contain no MCP shortcut.",
        ),
        check(
            "model-proposal-is-inert",
            model_proposal_inert,
            "Model port returns an inert result before registry dispatch.",
        ),
        check(
            "tool-dispatch-is-grant-gated",
            grant_gated,
            "Dispatch requires the grant boundary and coordinator cannot mint grants or policy.",
        ),
        check(
            "shell-is-presentation-only",
            shell_presentation_only,
            "Client sources expose presentation, approval, and coordinator ports only.",
        ),
        check(
            "capability-pack-has-no-platform-edge",
            read_pack_no_platform,
            "Read-only pack depends only on contracts and has no platform edge.",
        ),
        check(
            "exact-profile-has-no-hidden-fallback",
            exact_profile_no_fallback,
            "Runtime, host, and client bind the exact profile and deny substitution.",
        ),
        check(
            "client-cannot-certify-success",
            client_cannot_certify,
            "Verifier-owned completion is identical across clients; client code cannot mint success.",
        ),
        check(
            "bypass-campaign-is-retained",
            bypass_retained,
            "Hash-bound source campaign retains confusion, authority, and mediation denials.",
        ),
    ]


def build_report(revision: str) -> dict[str, Any]:
    checks = review_checks(committed_sources(revision))
    return {
        "schema_version": 1,
        "artifact_id": "story-23.4-coordinator-boundary-review",
        "source_revision": revision,
        "status": (
            "pass-independent-automated-source-boundary-review"
            if all(item["passed"] for item in checks)
            else "fail-source-boundary-review"
        ),
        "task_ids": ["23.4.3.2", "RV-05", "RV-18"],
        "checks": checks,
        "sources": source_records(revision),
        "independent_human_review_performed": False,
        "installed_runtime_observed": False,
        "manual_fuzzing_executed": False,
        "release_approved": False,
        "limitations": list(LIMITATIONS),
        "report_sha256": ZERO_SHA256,
    }


def seal_report(report: dict[str, Any]) -> dict[str, Any]:
    sealed = dict(report)
    sealed["report_sha256"] = ZERO_SHA256
    sealed["report_sha256"] = sha256_bytes(canonical_json_bytes(sealed))
    return sealed


def validate_report(report: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["runtime.coordinator_review.report_type"]
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        return ["runtime.coordinator_review.report_revision"]
    expected = seal_report(build_report(revision))
    if report != expected:
        failures.append("runtime.coordinator_review.report_drift")
    checks = report.get("checks")
    if (
        not isinstance(checks, list)
        or [item.get("check_id") for item in checks] != list(EXPECTED_CHECK_IDS)
        or not all(item.get("passed") is True for item in checks)
    ):
        failures.append("runtime.coordinator_review.checks")
    digest = report.get("report_sha256")
    if not isinstance(digest, str) or SHA256.fullmatch(digest) is None:
        failures.append("runtime.coordinator_review.digest")
    for field in (
        "independent_human_review_performed",
        "installed_runtime_observed",
        "manual_fuzzing_executed",
        "release_approved",
    ):
        if report.get(field) is not False:
            failures.append(f"runtime.coordinator_review.{field}")
    return failures


def read_report() -> Any:
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise CoordinatorReviewError(
            "runtime.coordinator_review.report_unavailable"
        ) from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    report = seal_report(build_report(revision))
    if arguments.write:
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    failures = validate_report(read_report() if REPORT_PATH.is_file() else report)
    if failures:
        raise CoordinatorReviewError("; ".join(failures))
    print("Story 23.4 coordinator boundary review: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
