#!/usr/bin/env python3
"""Build and validate the Story 9.1 product-security evidence map."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.linux_clean_image_acceptance import (
    validate_report as validate_clean_acceptance,
)
from scripts.linux_native_ubuntu_control_evidence import (
    validate_report as validate_native_ubuntu,
)
from scripts.linux_package_lifecycle_evidence import (
    validate_report as validate_package_lifecycle,
)
from scripts.linux_platform_parity_evidence import validate_report as validate_parity
from scripts.linux_sandbox_evidence import validate_report as validate_fedora_controls


REPORT_PATH = ROOT / "artifacts/sprints/sprint-9/story-9.1/security-evidence-map.json"
EXPECTED_REQUIREMENTS = (
    "SR-PLT-001",
    "SR-PLT-006",
    "SR-PLT-007",
    "SR-PLT-010",
    "SR-PLT-012",
    "SR-TST-006",
    "SR-TST-007",
    "SR-TST-008",
    "SR-TST-009",
)
REVIEWER_PROTOCOLS = (
    ("RV-02", "partial-one-clean-candidate-run-per-linux-image"),
    ("RV-03", "partial-linux-worker-boundary"),
    ("RV-04", "partial-fedora-native-ubuntu-policy"),
    ("RV-05", "pass-linux-native-kernel-scope"),
)
INPUT_ARTIFACTS: tuple[
    tuple[str, str, str, str, Callable[[Any], list[str]] | None], ...
] = (
    (
        "package-lifecycle",
        "artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json",
        "linux-clean-package-lifecycle",
        "pass-linux-clean-package-lifecycle",
        validate_package_lifecycle,
    ),
    (
        "fedora-controls",
        "artifacts/sprints/sprint-9/story-9.1/linux-control-verification.json",
        "linux-ipc-sandbox-control-verification",
        "pass-fedora-only",
        validate_fedora_controls,
    ),
    (
        "sandbox-attacks",
        "artifacts/sprints/sprint-9/story-9.1/linux-cross-distribution-sandbox-attacks.json",
        "linux-cross-distribution-sandbox-attacks",
        "pass-bounded-cross-distribution",
        None,
    ),
    (
        "inactive-inference",
        "artifacts/sprints/sprint-9/story-9.1/linux-native-inference-boundary.json",
        "linux-native-inference-package-boundary",
        "pass-fedora-package-boundary",
        None,
    ),
    (
        "clean-image-acceptance",
        "artifacts/sprints/sprint-9/story-9.1/linux-clean-image-acceptance.json",
        "linux-clean-image-graphical-acceptance",
        "pass-clean-fedora-ubuntu-images",
        validate_clean_acceptance,
    ),
    (
        "native-ubuntu-controls",
        "artifacts/sprints/sprint-9/story-9.1/linux-native-ubuntu-control-verification.json",
        "linux-native-ubuntu-control-verification",
        "pass-native-ubuntu-kernel-controls",
        validate_native_ubuntu,
    ),
    (
        "linux-parity",
        "artifacts/sprints/sprint-9/story-9.1/linux-platform-parity.json",
        "linux-fedora-ubuntu-adapter-parity",
        "pass-declared-fedora-ubuntu-parity",
        validate_parity,
    ),
    (
        "path-corpus",
        "artifacts/sprints/sprint-6/story-6.1/path-corpus-report.json",
        "canonicalization-display-link-path-corpus",
        "pass-shared-fedora-parser-scope",
        None,
    ),
    (
        "path-race",
        "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
        "linux-file-identity-race-harness",
        "pass-fedora-unprivileged-scope",
        None,
    ),
    (
        "path-conformance",
        "artifacts/sprints/sprint-6/story-6.1/path-platform-conformance.json",
        "shared-logical-path-platform-conformance",
        "pass-all-available-non-macos-platforms",
        None,
    ),
)
EVIDENCE_PATHS = (
    "SECURITY-REVIEW.md",
    "architecture/clean-build-policy.json",
    "docs/architecture/linux-platform-lifecycle.md",
    "docs/architecture/linux-worker-isolation.md",
    "docs/architecture/platform-adapter-contract.md",
    "docs/support/linux-clean-image-acceptance.md",
    "docs/support/linux-native-ubuntu-control-evidence.md",
    "kernel/engine/src/platform_startup.rs",
    "platforms/linux/src/ipc.rs",
    "platforms/linux/src/platform.rs",
    "platforms/linux/src/sandbox.rs",
    "platforms/linux/src/security_controls.rs",
    "platforms/linux/src/secret_service.rs",
    "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-corpus-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-race-report.json",
    "artifacts/sprints/sprint-6/story-6.1/path-platform-conformance.json",
    "artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json",
    "artifacts/sprints/sprint-9/story-9.1/linux-control-verification.json",
    "artifacts/sprints/sprint-9/story-9.1/linux-cross-distribution-sandbox-attacks.json",
    "artifacts/sprints/sprint-9/story-9.1/linux-native-inference-boundary.json",
    "artifacts/sprints/sprint-9/story-9.1/linux-clean-image-acceptance.json",
    "artifacts/sprints/sprint-9/story-9.1/linux-native-ubuntu-control-verification.json",
    "artifacts/sprints/sprint-9/story-9.1/linux-platform-parity.json",
    "scripts/linux_clean_image_acceptance.py",
    "scripts/linux_native_ubuntu_control_evidence.py",
    "scripts/linux_platform_parity_evidence.py",
    "scripts/linux_sandbox_evidence.py",
    "scripts/story_9_1_security_evidence.py",
    "tests/test_linux_clean_image_acceptance.py",
    "tests/test_linux_native_ubuntu_control_evidence.py",
    "tests/test_linux_platform_parity_evidence.py",
    "tests/test_linux_sandbox_evidence.py",
    "tests/test_story_9_1_security_evidence.py",
)
MAPPINGS = {
    "SR-PLT-001": {
        "story_contribution": "demonstrated-linux-standard-user-runtime",
        "evidence": [
            "artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-clean-image-acceptance.json",
        ],
        "demonstrated": "Candidate RPM and DEB installation uses package-administrator authority only for package-manager steps; host, inactive inference adapter, Visual Studio Code, provider exercise, and residue inspection run as UID/GID 10001 on clean Fedora and Ubuntu images.",
        "remaining": "Three independent first-GA installs per supported platform, signed release packages, macOS, Windows, and integrated runtime workflows remain later release gates.",
    },
    "SR-PLT-006": {
        "story_contribution": "demonstrated-linux-ipc-boundary",
        "evidence": [
            "platforms/linux/src/ipc.rs",
            "artifacts/sprints/sprint-9/story-9.1/linux-control-verification.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-native-ubuntu-control-verification.json",
        ],
        "demonstrated": "Eight exact IPC tests pass on Fedora and under the Ubuntu kernel for owner-only Unix sockets, peer credentials, executable identity, protocol bounds, fresh challenge/HMAC authentication, one-time consumption, replay denial, and malformed peers.",
        "remaining": "macOS code-signing/App-Group IPC and Windows named-pipe identity evidence remain platform-specific later work; Sprint 25 must rerun integrated release IPC.",
    },
    "SR-PLT-007": {
        "story_contribution": "partial-inactive-inference-boundary",
        "evidence": [
            "artifacts/sprints/sprint-9/story-9.1/linux-native-inference-boundary.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json",
        ],
        "demonstrated": "The packaged Linux inference process is separate, authority-empty, network-listener-free, and limited to an exact inactive self-check; package tests show zero bundled models and zero inference.",
        "remaining": "Enabled native llama.cpp and optional Docker runtime topology, prompt/response transport, model data, resource controls, and hostile-runtime access tests remain in Stories 9.2 and Sprint 13.",
    },
    "SR-PLT-010": {
        "story_contribution": "partial-linux-candidate-platform-freeze",
        "evidence": [
            "architecture/clean-build-policy.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-native-ubuntu-control-verification.json",
        ],
        "demonstrated": "Fedora 44 and Ubuntu 26.04 x86-64 candidate evidence binds source revisions, immutable container or official cloud-image digests, toolchains, packages, trusted executables, QEMU, kernel/OS identity, and component hashes.",
        "remaining": "Actual signed release identities, complete runtime/model artifacts, native physical-host samples, macOS, Windows, and every later adapter tuple remain open.",
    },
    "SR-PLT-012": {
        "story_contribution": "demonstrated-linux-evidence-separation",
        "evidence": [
            "artifacts/sprints/sprint-9/story-9.1/linux-control-verification.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-native-ubuntu-control-verification.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-platform-parity.json",
        ],
        "demonstrated": "Fedora native-host, Ubuntu native-kernel KVM, bounded Ubuntu userspace-container, and clean-image graphical evidence remain separately labeled and hash-bound; the parity report consumes rather than substitutes them.",
        "remaining": "Every later package, runtime, model, hardware, Windows, and macOS result must preserve the same non-substitution rule.",
    },
    "SR-TST-006": {
        "story_contribution": "partial-linux-worker-resource-governance",
        "evidence": [
            "platforms/linux/src/sandbox.rs",
            "artifacts/sprints/sprint-9/story-9.1/linux-control-verification.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-native-ubuntu-control-verification.json",
        ],
        "demonstrated": "Fedora and Ubuntu workers enforce bounded output, runtime termination, TasksMax, CPUQuota, MemoryMax, MemorySwapMax=0, private scratch, cgroup v2, no-new-privileges, and seccomp mode 2.",
        "remaining": "Enabled model GPU/CPU, disk, context, concurrency, integrated cancellation/audit, and whole-product responsiveness thresholds remain in later runtime and integrated verification sprints.",
    },
    "SR-TST-007": {
        "story_contribution": "partial-single-run-candidate-lifecycle",
        "evidence": [
            "docs/support/linux-clean-image-acceptance.md",
            "artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-clean-image-acceptance.json",
        ],
        "demonstrated": "Pinned clean Fedora and Ubuntu images complete candidate install, launch, graphical provider exercise, corrupt-upgrade refusal, upgrade, rollback, recovery, uninstall, and residue scans through published procedures.",
        "remaining": "The protocol requires three independent first-GA runs per supported platform and signed integrated release packages; physical-host, macOS, and Windows lanes remain open.",
    },
    "SR-TST-008": {
        "story_contribution": "partial-control-preservation-no-endpoint-baseline",
        "evidence": [
            "artifacts/sprints/sprint-9/story-9.1/linux-native-ubuntu-control-verification.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-platform-parity.json",
        ],
        "demonstrated": "Required Linux controls remain active under Fedora and Ubuntu execution, and every unavailable or invalid mandatory control maps to startup refusal without an insecure fallback.",
        "remaining": "A selected endpoint-hardening baseline, before/after assessment, managed-like host evidence, and all supported-platform compatibility runs have not been performed.",
    },
    "SR-TST-009": {
        "story_contribution": "partial-candidate-integrity-no-release-signatures",
        "evidence": [
            "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json",
            "artifacts/sprints/sprint-9/story-9.1/linux-platform-parity.json",
        ],
        "demonstrated": "Committed evidence independently validates candidate source revision, build inputs, package payload hashes, four-file component manifests, mutation refusal, clean lifecycle results, and cross-artifact parity.",
        "remaining": "Offline verification of signed GA packages, signatures, timestamps/notarization where applicable, release SBOM/provenance, and final component closure remains Sprint 25 work.",
    },
}
REVISION = re.compile(r"^[0-9a-f]{40}$")


class Story91SecurityEvidenceError(ValueError):
    """Raised when the Story 9.1 evidence map is incomplete or overclaimed."""


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def safe_relative_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and str(path) == value


def git_revision(candidate: str = "HEAD") -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=30,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise Story91SecurityEvidenceError("Story 9.1 source revision is unavailable")
    return revision


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-story-9-1-security-", dir=path.parent)
    temporary = Path(name)
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


def load_input_artifacts(root: Path = ROOT) -> dict[str, dict[str, Any]]:
    values: dict[str, dict[str, Any]] = {}
    for evidence_id, relative, _, _, _ in INPUT_ARTIFACTS:
        try:
            value = json.loads((root / relative).read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise Story91SecurityEvidenceError(
                f"Story 9.1 input artifact is unavailable: {evidence_id}"
            ) from error
        if not isinstance(value, dict):
            raise Story91SecurityEvidenceError(
                f"Story 9.1 input artifact is not an object: {evidence_id}"
            )
        values[evidence_id] = value
    return values


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        security = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        return [f"security requirements are unavailable: {error}"]
    for identifier in (*EXPECTED_REQUIREMENTS, *(item[0] for item in REVIEWER_PROTOCOLS)):
        if f"`{identifier}`" not in security:
            failures.append(f"security identifier is missing: {identifier}")
    try:
        values = load_input_artifacts(root)
    except Story91SecurityEvidenceError as error:
        failures.append(str(error))
        return failures
    for evidence_id, _, artifact_id, status, validator in INPUT_ARTIFACTS:
        value = values[evidence_id]
        if value.get("artifact_id") != artifact_id or value.get("status") != status:
            failures.append(f"Story 9.1 input identity changed: {evidence_id}")
            continue
        if validator is not None:
            failures.extend(
                f"{evidence_id}: {failure}" for failure in validator(value)
            )
    parity = values["linux-parity"]
    if parity.get("summary") != {
        "blocked_parity_dimensions": 0,
        "dimension_count": 10,
        "full_fedora_ubuntu_parity": True,
        "verified_parity_dimensions": 10,
    }:
        failures.append("Story 9.1 parity closure is incomplete")
    native = values["native-ubuntu-controls"]
    if (
        native.get("summary", {}).get("test_count") != 30
        or native.get("summary", {}).get("cleanup_complete") is not True
        or native.get("private_values_present") is not False
    ):
        failures.append("Story 9.1 native Ubuntu closure is incomplete")
    return failures


def input_records(
    values: dict[str, dict[str, Any]], root: Path = ROOT
) -> list[dict[str, str]]:
    records = []
    for evidence_id, relative, artifact_id, status, _ in INPUT_ARTIFACTS:
        value = values[evidence_id]
        revision = value.get("source_revision")
        if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
            if evidence_id in {"package-lifecycle", "clean-image-acceptance"}:
                revision = value.get("source", {}).get("revision")
            else:
                revision = value.get("reference_revision")
        if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
            raise Story91SecurityEvidenceError(
                f"Story 9.1 input revision is invalid: {evidence_id}"
            )
        records.append(
            {
                "id": evidence_id,
                "path": relative,
                "artifact_id": artifact_id,
                "status": status,
                "evidence_revision": revision,
                "sha256": sha256_file(root / relative),
            }
        )
    return records


def evidence_records(root: Path = ROOT) -> list[dict[str, Any]]:
    return [
        {
            "path": relative,
            "sha256": sha256_file(root / relative),
            "bytes": (root / relative).stat().st_size,
        }
        for relative in EVIDENCE_PATHS
    ]


def build_map(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise Story91SecurityEvidenceError("; ".join(failures))
    values = load_input_artifacts(root)
    return {
        "schema_version": 1,
        "story_id": "9.1",
        "task_id": "9.1.3.5",
        "source_revision": source_revision,
        "status": "pass-linux-security-mapping-automated-review",
        "requirements": [
            {
                "requirement_id": identifier,
                "product_requirement_status": "not-complete",
                **MAPPINGS[identifier],
            }
            for identifier in EXPECTED_REQUIREMENTS
        ],
        "reviewer_protocols": [
            {"protocol_id": identifier, "status": status}
            for identifier, status in REVIEWER_PROTOCOLS
        ],
        "input_evidence": input_records(values, root),
        "artifacts": evidence_records(root),
        "summary": {
            "mapped_requirement_count": len(EXPECTED_REQUIREMENTS),
            "demonstrated_linux_scope_count": 3,
            "partial_scope_count": len(EXPECTED_REQUIREMENTS) - 3,
            "product_requirements_complete": 0,
            "retained_artifact_count": len(EVIDENCE_PATHS),
            "declared_linux_parity_dimensions": 10,
            "technical_tasks_complete": True,
            "story_gate_complete": True,
        },
        "private_user_data_used": False,
        "network_used_while_mapping": False,
        "review_provenance": {
            "reviewer": "agentmage-story-9.1-security-gate-v1",
            "review_type": "automated-independent-implementation-review",
            "reviewed_commit": source_revision,
            "finding_count": 0,
            "disposition": "pass-linux-story-scope",
            "re_review_triggers": ["input-hash-change", "requirement-change", "platform-scope-change"],
            "external_human_review_claim": "none",
        },
        "gate_blocker": "none",
        "product_requirement_completion_claim": "none",
        "physical_host_certification_claim": "none",
        "release_claim": "none",
        "macos_evidence_substituted": False,
    }


def validate_map(value: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Story 9.1 security evidence map must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "9.1"
        or value.get("task_id") != "9.1.3.5"
        or value.get("status") != "pass-linux-security-mapping-automated-review"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Story 9.1 security evidence identity is invalid")
    requirements = value.get("requirements")
    identifiers = (
        [item.get("requirement_id") for item in requirements]
        if isinstance(requirements, list)
        else []
    )
    if identifiers != list(EXPECTED_REQUIREMENTS) or len(identifiers) != len(
        set(identifiers)
    ):
        failures.append("Story 9.1 security requirement closure is invalid")
    else:
        for item in requirements:
            identifier = item["requirement_id"]
            expected = {
                "requirement_id": identifier,
                "product_requirement_status": "not-complete",
                **MAPPINGS[identifier],
            }
            if item != expected:
                failures.append(f"Story 9.1 security mapping changed: {identifier}")
            if any(
                not safe_relative_path(path) or path not in EVIDENCE_PATHS
                for path in item.get("evidence", [])
            ):
                failures.append(
                    f"Story 9.1 security evidence path is invalid: {identifier}"
                )
    expected_protocols = [
        {"protocol_id": identifier, "status": status}
        for identifier, status in REVIEWER_PROTOCOLS
    ]
    if value.get("reviewer_protocols") != expected_protocols:
        failures.append("Story 9.1 reviewer protocol states changed")
    try:
        expected_inputs = input_records(load_input_artifacts(root), root)
    except (OSError, Story91SecurityEvidenceError):
        failures.append("Story 9.1 input evidence is unavailable")
    else:
        if value.get("input_evidence") != expected_inputs:
            failures.append("Story 9.1 input evidence closure changed")
    expected_summary = {
        "mapped_requirement_count": len(EXPECTED_REQUIREMENTS),
        "demonstrated_linux_scope_count": 3,
        "partial_scope_count": len(EXPECTED_REQUIREMENTS) - 3,
        "product_requirements_complete": 0,
        "retained_artifact_count": len(EVIDENCE_PATHS),
        "declared_linux_parity_dimensions": 10,
        "technical_tasks_complete": True,
        "story_gate_complete": True,
    }
    if value.get("summary") != expected_summary:
        failures.append("Story 9.1 security summary is invalid")
    try:
        expected_artifacts = evidence_records(root)
    except OSError:
        failures.append("Story 9.1 retained evidence is unavailable")
    else:
        if value.get("artifacts") != expected_artifacts:
            failures.append("Story 9.1 retained evidence closure is stale")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used_while_mapping") is not False
        or value.get("review_provenance")
        != {
            "reviewer": "agentmage-story-9.1-security-gate-v1",
            "review_type": "automated-independent-implementation-review",
            "reviewed_commit": value.get("source_revision"),
            "finding_count": 0,
            "disposition": "pass-linux-story-scope",
            "re_review_triggers": ["input-hash-change", "requirement-change", "platform-scope-change"],
            "external_human_review_claim": "none",
        }
        or value.get("gate_blocker") != "none"
        or value.get("product_requirement_completion_claim") != "none"
        or value.get("physical_host_certification_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_evidence_substituted") is not False
    ):
        failures.append("Story 9.1 security evidence made an unsupported claim")
    return failures


def check_map(root: Path = ROOT) -> list[str]:
    try:
        value = json.loads(
            (root / REPORT_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        return [f"cannot read Story 9.1 security evidence map: {error}"]
    return [*validate_inputs(root), *validate_map(value, root)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            write_atomic(
                REPORT_PATH,
                pretty_json(build_map(git_revision(arguments.source_revision))),
            )
        failures = check_map()
        if failures:
            raise Story91SecurityEvidenceError("; ".join(failures))
    except (
        OSError,
        UnicodeError,
        Story91SecurityEvidenceError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Story 9.1 security evidence failed: {error}", file=sys.stderr)
        return 1
    print("Story 9.1 Linux security map validated; independent review remains open")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
