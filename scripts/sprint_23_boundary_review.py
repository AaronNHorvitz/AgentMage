#!/usr/bin/env python3
"""Build and validate the gate-owned Sprint 23 source-boundary review."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-23/source-boundary-review.json"
SOURCES: Final = (
    "kernel/engine/src/runtime_loop.rs",
    "shells/host/src/linux_read.rs",
    "shells/host/src/native_chat_runtime.rs",
    "shells/host/src/participant_ingress.rs",
    "shells/host/src/runtime_transport.rs",
    "shells/vscode/src/model_discovery.ts",
    "shells/vscode/src/host_bridge.ts",
    "shells/vscode/src/native_accessibility.ts",
    "shells/vscode/src/participant_ingress.ts",
    "shells/vscode/src/provider.ts",
    "shells/vscode/src/runtime_transport.ts",
    "shells/vscode/src/verified_chat.ts",
)
SECURITY_REQUIREMENTS: Final = (
    "SR-PLT-005",
    "SR-PLT-006",
    "SR-ACC-007",
    "SR-DAT-003",
    "SR-OPS-001",
    "SR-OPS-003",
    "SR-TST-004",
    "SR-AI-013",
    "SR-AI-015",
    "SR-MGM-001",
    "SR-CIV-006",
    "SR-CIV-007",
    "SR-CIV-008",
    "SR-CIV-009",
    "SR-TST-008",
)


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        capture_output=True,
        check=False,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def build_report(revision: str) -> dict[str, Any]:
    sources = {path: git_bytes(revision, path) for path in SOURCES}
    vscode = b"\n".join(value for path, value in sources.items() if "/vscode/" in path)
    client = b"\n".join(
        value
        for path, value in sources.items()
        if "/vscode/" in path and not path.endswith("/host_bridge.ts")
    )
    host = b"\n".join(value for path, value in sources.items() if "/host/" in path)
    engine = sources["kernel/engine/src/runtime_loop.rs"]
    checks = {
        "client_has_no_direct_filesystem_network_or_process_import": all(
            token not in client
            for token in (
                b'from "node:fs"',
                b'from "node:net"',
                b'from "node:child_process"',
            )
        ),
        "client_uses_authenticated_runtime_transport": b"AuthenticatedLinuxHostBridge"
        in vscode
        and b"prepare_runtime" in vscode,
        "host_owns_runtime_transport": b"trait RuntimeTransportPort" in host
        and b"NativeChatRuntimeService" in host,
        "kernel_owns_policy_tool_and_verifier_completion": all(
            token in engine
            for token in (
                b"RuntimePermissionDisposition",
                b"ToolDispatcher",
                b"VerifierRegistry",
            )
        ),
        "participant_sources_are_current_request_only": b"input.references" in client
        and b"workspace.findFiles" not in client,
        "accessibility_failures_are_gate_blocking": b"evaluateNativeAccessibility"
        in client
        and b"blocking" in client,
        "automatic_model_substitution_absent": b"automaticFallback" not in client
        and b"fallbackProfile" not in client,
    }
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-23-source-boundary-review",
        "source_revision": revision,
        "reviewer_identity": "scripts/sprint_23_boundary_review.py",
        "review_class": "independent-automated-source-boundary",
        "independent_human_review_performed": False,
        "source_sha256": {path: sha256(value) for path, value in sources.items()},
        "security_requirement_ids": list(SECURITY_REQUIREMENTS),
        "checks": checks,
        "status": "PASS_SOURCE_BOUNDARY_REVIEW" if all(checks.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated source review, not a human-review claim.",
            "Installed VSIX, assistive-technology, production-model, and supported-platform campaigns remain separate blockers.",
        ],
    }


def validate(value: Any, *, current_revision: str = "HEAD") -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    if value.get("reviewer_identity") != "scripts/sprint_23_boundary_review.py":
        failures.append("reviewer identity drift")
    if value.get("review_class") != "independent-automated-source-boundary":
        failures.append("review class drift")
    if value.get("independent_human_review_performed") is not False:
        failures.append("human review overclaim")
    checks = value.get("checks", {})
    if (
        not isinstance(checks, dict)
        or not checks
        or any(item is not True for item in checks.values())
    ):
        failures.append("boundary check failed or suppressed")
    if value.get("status") != "PASS_SOURCE_BOUNDARY_REVIEW":
        failures.append("source review does not pass")
    try:
        expected = build_report(current_revision)
    except ValueError as error:
        return failures + [str(error)]
    if value != expected:
        failures.append("review is stale, incomplete, reordered, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    value = build_report(args.source_revision)
    if args.write:
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(
            json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    else:
        try:
            value = json.loads(REPORT.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            print(f"cannot read Sprint 23 review: {error}", file=sys.stderr)
            return 1
    failures = validate(value, current_revision=args.source_revision)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 23 gate-owned source-boundary review passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
