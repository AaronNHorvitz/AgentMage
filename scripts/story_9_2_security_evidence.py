#!/usr/bin/env python3
"""Build and validate the Story 9.2 product-security evidence map."""

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
from typing import Any, Callable, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.linux_docker_control_disablement_evidence import (
    validate as validate_controls,
)
from scripts.linux_docker_guard_evidence import validate_report as validate_guard
from scripts.linux_docker_kvm_evidence import validate_report as validate_kvm
from scripts.linux_docker_preflight_evidence import (
    validate_report as validate_preflight,
)
from scripts.linux_docker_prerequisite_evidence import (
    validate_report as validate_prerequisites,
)
from scripts.linux_docker_reachability_evidence import (
    validate as validate_reachability,
)
from scripts.linux_docker_runtime_evidence import validate_report as validate_runtime
from scripts.linux_native_runtime_evidence import validate_report as validate_native
from scripts.linux_platform_parity_evidence import validate_report as validate_parity


REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-9/story-9.2/security-evidence-map.json"
)
EXPECTED_REQUIREMENTS: Final = (
    "SR-PLT-001",
    "SR-PLT-006",
    "SR-PLT-007",
    "SR-PLT-010",
    "SR-PLT-012",
    "SR-NET-003",
    "SR-NET-006",
)
REVIEWER_PROTOCOLS: Final = (
    ("RV-02", "partial-linux-candidate-install-and-topology"),
    ("RV-03", "partial-linux-runtime-boundary"),
    ("RV-05", "partial-linux-docker-guard-identity"),
)
INPUT_ARTIFACTS: Final[tuple[
    tuple[str, str, str, str, Callable[[Any], list[str]]], ...
]] = (
    (
        "native-runtime",
        "artifacts/sprints/sprint-9/story-9.2/linux-native-runtime-package.json",
        "linux-native-llama-runtime-package",
        "pass-fedora-native-package-boundary",
        validate_native,
    ),
    (
        "docker-runtime",
        "artifacts/sprints/sprint-9/story-9.2/linux-docker-runtime-profile.json",
        "linux-docker-model-runner-compatibility-profile",
        "pass-pinned-contract-docker-not-installed",
        validate_runtime,
    ),
    (
        "docker-guard",
        "artifacts/sprints/sprint-9/story-9.2/linux-docker-endpoint-guard.json",
        "linux-docker-model-runner-endpoint-guard",
        "pass-guard-contract-no-live-docker",
        validate_guard,
    ),
    (
        "docker-preflight",
        "artifacts/sprints/sprint-9/story-9.2/linux-docker-drift-preflight.json",
        "linux-docker-drift-preflight",
        "pass-preflight-contract-no-live-docker",
        validate_preflight,
    ),
    (
        "docker-prerequisites",
        "artifacts/sprints/sprint-9/story-9.2/linux-docker-production-prerequisites.json",
        "linux-docker-production-prerequisites",
        "pass-packaged-prerequisites-no-live-docker",
        validate_prerequisites,
    ),
    (
        "docker-topology",
        "artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json",
        "linux-docker-kvm-topology",
        "pass-live-topology-no-inference",
        validate_kvm,
    ),
    (
        "docker-reachability",
        "artifacts/sprints/sprint-9/story-9.2/linux-docker-reachability.json",
        "linux-docker-hostile-reachability",
        "pass-hostile-denial-authenticated-guard-control-no-inference",
        validate_reachability,
    ),
    (
        "docker-controls",
        "artifacts/sprints/sprint-9/story-9.2/linux-docker-control-disablement.json",
        "linux-docker-independent-control-disablement",
        "pass-independent-refusal-no-fallback-no-inference",
        validate_controls,
    ),
    (
        "linux-parity",
        "artifacts/sprints/sprint-9/story-9.1/linux-platform-parity.json",
        "linux-fedora-ubuntu-adapter-parity",
        "pass-declared-fedora-ubuntu-parity",
        validate_parity,
    ),
)
EVIDENCE_PATHS: Final = (
    "SECURITY-REVIEW.md",
    "RUNTIME-BOUNDARIES.md",
    "docs/architecture/linux-docker-runtime-topology.md",
    "docs/architecture/strict-local-boundary.md",
    "docs/decisions/0029-closed-linux-native-llama-runtime-package.md",
    "docs/decisions/0030-closed-linux-docker-model-runner-compatibility-profile.md",
    "docs/decisions/0031-private-docker-model-runner-endpoint-guard.md",
    "docs/decisions/0032-fail-closed-docker-topology-preflight.md",
    "docs/decisions/0033-production-docker-guard-and-observer-prerequisite.md",
    "docs/decisions/0034-pin-observed-model-runner-ipv6-wildcard.md",
    "docs/decisions/0035-bind-docker-runtime-peer-identity.md",
    "docs/decisions/0036-observe-network-namespace-through-procfs.md",
    "docs/decisions/0037-admit-canonical-runner-digest-reference.md",
    "docs/decisions/0038-bind-cross-uid-guard-peer-through-challenge.md",
    "docs/decisions/0039-verify-independent-docker-control-disablement.md",
    "model-profiles/runtimes/docker-model-runner-guard-v1-linux-x86_64.json",
    "model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json",
    "model-profiles/runtimes/llama-cpp-b10333-linux-x86_64.json",
    "platforms/linux-inference/src/docker_guard_service.rs",
    "platforms/linux-inference/src/docker_live_collector.rs",
    "platforms/linux-inference/src/docker_preflight.rs",
    "platforms/linux-inference/src/docker_runtime.rs",
    "platforms/linux-inference/src/native_runtime.rs",
    "artifacts/sprints/sprint-9/story-9.1/linux-platform-parity.json",
    "artifacts/sprints/sprint-9/story-9.2/linux-native-runtime-package.json",
    "artifacts/sprints/sprint-9/story-9.2/linux-docker-runtime-profile.json",
    "artifacts/sprints/sprint-9/story-9.2/linux-docker-endpoint-guard.json",
    "artifacts/sprints/sprint-9/story-9.2/linux-docker-drift-preflight.json",
    "artifacts/sprints/sprint-9/story-9.2/linux-docker-production-prerequisites.json",
    "artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json",
    "artifacts/sprints/sprint-9/story-9.2/linux-docker-reachability.json",
    "artifacts/sprints/sprint-9/story-9.2/linux-docker-control-disablement.json",
    "scripts/linux_docker_kvm_evidence.py",
    "scripts/linux_docker_reachability_evidence.py",
    "scripts/linux_docker_control_disablement_evidence.py",
    "scripts/story_9_2_security_evidence.py",
    "tests/test_linux_docker_kvm_evidence.py",
    "tests/test_linux_docker_reachability_evidence.py",
    "tests/test_linux_docker_control_disablement_evidence.py",
    "tests/test_story_9_2_security_evidence.py",
)
MAPPINGS: Final = {
    "SR-PLT-001": {
        "story_contribution": "partial-standard-user-runtime-root-collector-gap",
        "evidence": [
            "docs/architecture/linux-docker-runtime-topology.md",
            "artifacts/sprints/sprint-9/story-9.2/linux-native-runtime-package.json",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json",
        ],
        "demonstrated": "The native adapter and held runtime peer run as the ordinary user, while the Docker guard runs as a distinct capability-free non-root user and the runner has no added capabilities.",
        "remaining": "The optional Docker topology collector currently requires effective UID 0 for each fresh admission. Until an independently reviewed standard-user-compatible observation design exists, Docker compatibility cannot satisfy the no-administrator-after-installation product requirement. Signed integrated packages and three independent first-GA installs per supported platform also remain open.",
    },
    "SR-PLT-006": {
        "story_contribution": "partial-linux-docker-ipc-authentication",
        "evidence": [
            "platforms/linux-inference/src/docker_guard_service.rs",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-endpoint-guard.json",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-reachability.json",
        ],
        "demonstrated": "The Linux Docker guard binds kernel peer credentials, UID, PID, process start, cgroup, challenge-bound executable identity, protocol, and a fresh one-use challenge. A live authenticated control reaches the production request-policy refusal while hostile raw positions remain disconnected.",
        "remaining": "The integrated product kernel-to-guard request path, cancellation, rapid reconnect and resource-exhaustion campaigns, complete audit events, and macOS and Windows IPC identities remain later gates.",
    },
    "SR-PLT-007": {
        "story_contribution": "partial-authority-free-runtime-topology",
        "evidence": [
            "docs/architecture/linux-docker-runtime-topology.md",
            "artifacts/sprints/sprint-9/story-9.2/linux-native-runtime-package.json",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json",
        ],
        "demonstrated": "The packaged native and Docker runtime profiles expose no workspace, tool, grant, credential, Docker-socket, host-root, package-manager, or acquisition authority; live Docker inspection confirms exact mounts, capabilities, namespaces, resources, and zero-egress topology.",
        "remaining": "No inference request was performed. Runtime-originated attempts against files, environment, credentials, sockets, tools, grants, and peer processes during real model inference remain mandatory in Sprint 13 and integrated security testing.",
    },
    "SR-PLT-010": {
        "story_contribution": "partial-linux-runtime-platform-freeze",
        "evidence": [
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-production-prerequisites.json",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-control-disablement.json",
        ],
        "demonstrated": "The Fedora 44 and Ubuntu 26.04 KVM records bind kernels, Docker Engine versions, immutable runtime and model digests, package hashes, process identities, runtime profiles, prepared images, and source revisions.",
        "remaining": "Physical-host identities, signed release packages, graphics and driver tuples, complete VS Code integration, macOS, Windows, and the final supported platform matrix remain open.",
    },
    "SR-PLT-012": {
        "story_contribution": "demonstrated-story-evidence-separation",
        "evidence": [
            "artifacts/sprints/sprint-9/story-9.1/linux-platform-parity.json",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-control-disablement.json",
        ],
        "demonstrated": "Native package, contract-only Docker, packaged prerequisite, Fedora native-kernel KVM, Ubuntu native-kernel KVM, reachability, and control-disablement evidence remain separately identified and hash-bound. The map compares but never substitutes these classes.",
        "remaining": "Later physical-host, inference, model-quality, accelerator, macOS, Windows, signed-release, and integrated-product results must preserve the same non-substitution rule.",
    },
    "SR-NET-003": {
        "story_contribution": "demonstrated-kvm-private-docker-reachability",
        "evidence": [
            "docs/architecture/linux-docker-runtime-topology.md",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-reachability.json",
        ],
        "demonstrated": "Fedora and Ubuntu place the immutable wildcard Model Runner listener only inside the route-free private namespace, expose no host listener, and deny host, LAN, same-user, extension, tool-worker, separate-namespace, and ordinary-container connections. The authenticated Unix guard path is the sole positive control.",
        "remaining": "The real kernel inference request path, long-duration packet capture, physical-host firewall behavior, and complete normal-operation process attribution remain later strict-local and integration gates.",
    },
    "SR-NET-006": {
        "story_contribution": "partial-runtime-confusion-refusal",
        "evidence": [
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-drift-preflight.json",
            "artifacts/sprints/sprint-9/story-9.2/linux-docker-control-disablement.json",
        ],
        "demonstrated": "The preflight pins local runtime identities and rejects ambient proxy injection, management-listener drift, route or interface broadening, image substitution, mutable references, and nonzero egress state without fallback.",
        "remaining": "Independent DNS override, hostile loopback service, container alias, proxy case-variant, resolver race, and integrated runtime confusion campaigns remain in Sprint 10 and later network acceptance work.",
    },
}
PARITY_DIMENSIONS: Final = (
    "target-set",
    "admitted-baseline",
    "daemon-topology",
    "hostile-position-denial",
    "authenticated-guard-control",
    "independent-control-refusal",
    "no-fallback-or-inference",
    "complete-cleanup",
)
REVISION = re.compile(r"^[0-9a-f]{40}$")


class Story92SecurityEvidenceError(ValueError):
    """Raised when the Story 9.2 security map is incomplete or overclaimed."""


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
        raise Story92SecurityEvidenceError("Story 9.2 source revision is unavailable")
    return revision


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(
        prefix=".agentmage-story-9-2-security-", dir=path.parent
    )
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


def load_inputs(root: Path = ROOT) -> dict[str, dict[str, Any]]:
    values: dict[str, dict[str, Any]] = {}
    for evidence_id, relative, _, _, _ in INPUT_ARTIFACTS:
        try:
            value = json.loads((root / relative).read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise Story92SecurityEvidenceError(
                f"Story 9.2 input artifact is unavailable: {evidence_id}"
            ) from error
        if not isinstance(value, dict):
            raise Story92SecurityEvidenceError(
                f"Story 9.2 input artifact is not an object: {evidence_id}"
            )
        values[evidence_id] = value
    return values


def validate_inputs(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        security = (root / "SECURITY-REVIEW.md").read_text(encoding="utf-8")
        values = load_inputs(root)
    except (OSError, UnicodeError, Story92SecurityEvidenceError) as error:
        return [str(error)]
    for identifier in (*EXPECTED_REQUIREMENTS, *(item[0] for item in REVIEWER_PROTOCOLS)):
        if f"`{identifier}`" not in security:
            failures.append(f"security identifier is missing: {identifier}")
    for evidence_id, _, artifact_id, status, validator in INPUT_ARTIFACTS:
        value = values[evidence_id]
        if value.get("artifact_id") != artifact_id or value.get("status") != status:
            failures.append(f"Story 9.2 input identity changed: {evidence_id}")
            continue
        failures.extend(f"{evidence_id}: {item}" for item in validator(value))
    return failures


def input_records(
    values: dict[str, dict[str, Any]], root: Path = ROOT
) -> list[dict[str, str]]:
    records = []
    for evidence_id, relative, artifact_id, status, _ in INPUT_ARTIFACTS:
        revision = values[evidence_id].get("source_revision")
        if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
            raise Story92SecurityEvidenceError(
                f"Story 9.2 input revision is invalid: {evidence_id}"
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


def parity_report(values: dict[str, dict[str, Any]]) -> dict[str, Any]:
    topology = values["docker-topology"]
    reachability = values["docker-reachability"]
    controls = values["docker-controls"]
    expected_targets = ["fedora-44-x86_64", "ubuntu-26.04-x86_64"]
    topology_targets = [item["target_id"] for item in topology["targets"]]
    reach_targets = [item["target_id"] for item in reachability["targets"]]
    control_targets = [item["target_id"] for item in controls["targets"]]
    reach_shapes = [
        [
            (probe["position"], probe["status"], probe["raw_tcp_connected"])
            for probe in item["observation"]["probes"]
        ]
        for item in reachability["targets"]
    ]
    control_shapes = [
        [
            (case["control"], case["mutation"], case["observed_refusal"])
            for case in item["observation"]["cases"]
        ]
        for item in controls["targets"]
    ]
    dimensions = {
        "target-set": topology_targets == reach_targets == control_targets == expected_targets,
        "admitted-baseline": all(
            item["observation"]["baseline"]["status"] == "admitted"
            for item in controls["targets"]
        ),
        "daemon-topology": all(
            item["observation"]["docker"]["daemon_configuration"]["listener"]
            == "direct-unix-socket"
            and item["observation"]["docker"]["daemon_configuration"][
                "socket_activation_active"
            ]
            is False
            for item in topology["targets"]
        ),
        "hostile-position-denial": len(reach_shapes) == 2 and reach_shapes[0] == reach_shapes[1],
        "authenticated-guard-control": all(
            item["observation"]["positive_control"]["challenge_authenticated"]
            is True
            and item["observation"]["positive_control"]["terminal_guard_status"]
            == "docker-guard.service.request-policy"
            for item in reachability["targets"]
        ),
        "independent-control-refusal": len(control_shapes) == 2
        and control_shapes[0] == control_shapes[1],
        "no-fallback-or-inference": controls["fallback_selected"] is False
        and controls["inference_performed"] is False
        and reachability["inference_performed"] is False
        and topology["claims"]["docker_inference_performed"] is False,
        "complete-cleanup": all(
            all(target["host_cleanup"].values())
            for artifact in (topology, reachability, controls)
            for target in artifact["targets"]
        ),
    }
    if tuple(dimensions) != PARITY_DIMENSIONS:
        raise Story92SecurityEvidenceError("Story 9.2 parity dimensions changed")
    return {
        "scope": "fedora-44-and-ubuntu-26.04-native-kernel-kvm",
        "dimensions": [
            {"dimension": name, "status": "verified" if passed else "blocked"}
            for name, passed in dimensions.items()
        ],
        "verified_dimension_count": sum(dimensions.values()),
        "blocked_dimension_count": len(dimensions) - sum(dimensions.values()),
        "full_declared_kvm_parity": all(dimensions.values()),
        "physical_host_parity_claim": "none",
    }


def build_map(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    failures = validate_inputs(root)
    if failures:
        raise Story92SecurityEvidenceError("; ".join(failures))
    values = load_inputs(root)
    parity = parity_report(values)
    if not parity["full_declared_kvm_parity"]:
        raise Story92SecurityEvidenceError("Story 9.2 Docker parity is incomplete")
    return {
        "schema_version": 1,
        "story_id": "9.2",
        "task_id": "9.2.2.4",
        "source_revision": source_revision,
        "status": "pass-linux-security-mapping-review-open",
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
        "parity_report": parity,
        "summary": {
            "mapped_requirement_count": len(EXPECTED_REQUIREMENTS),
            "demonstrated_story_scope_count": 2,
            "partial_scope_count": len(EXPECTED_REQUIREMENTS) - 2,
            "product_requirements_complete": 0,
            "retained_artifact_count": len(EVIDENCE_PATHS),
            "declared_kvm_parity_dimensions": len(PARITY_DIMENSIONS),
            "technical_tasks_complete": True,
            "story_gate_complete": False,
        },
        "known_blockers": [
            "Docker collector requires effective UID 0 and does not yet satisfy SR-PLT-001",
            "real model inference and runtime-originated authority attacks remain Sprint 13 work",
            "physical-host, signed-release, macOS, and Windows evidence is absent",
            "G-DOD-12 independent critical-boundary review is required",
        ],
        "private_user_data_used": False,
        "network_used_while_mapping": False,
        "external_independent_review_status": "required-not-performed",
        "gate_blocker": "G-DOD-12 independent critical-boundary review",
        "inference_claim": "none",
        "product_requirement_completion_claim": "none",
        "physical_host_certification_claim": "none",
        "release_claim": "none",
        "macos_evidence_substituted": False,
    }


def validate_map(value: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Story 9.2 security evidence map must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("story_id") != "9.2"
        or value.get("task_id") != "9.2.2.4"
        or value.get("status") != "pass-linux-security-mapping-review-open"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Story 9.2 security evidence identity is invalid")
    requirements = value.get("requirements")
    identifiers = (
        [item.get("requirement_id") for item in requirements]
        if isinstance(requirements, list)
        else []
    )
    if identifiers != list(EXPECTED_REQUIREMENTS) or len(identifiers) != len(
        set(identifiers)
    ):
        failures.append("Story 9.2 security requirement closure is invalid")
    else:
        for item in requirements:
            identifier = item["requirement_id"]
            expected = {
                "requirement_id": identifier,
                "product_requirement_status": "not-complete",
                **MAPPINGS[identifier],
            }
            if item != expected:
                failures.append(f"Story 9.2 security mapping changed: {identifier}")
            if any(
                not safe_relative_path(path) or path not in EVIDENCE_PATHS
                for path in item.get("evidence", [])
            ):
                failures.append(
                    f"Story 9.2 security evidence path is invalid: {identifier}"
                )
    expected_protocols = [
        {"protocol_id": identifier, "status": status}
        for identifier, status in REVIEWER_PROTOCOLS
    ]
    if value.get("reviewer_protocols") != expected_protocols:
        failures.append("Story 9.2 reviewer protocol states changed")
    try:
        values = load_inputs(root)
        expected_inputs = input_records(values, root)
        expected_artifacts = evidence_records(root)
        expected_parity = parity_report(values)
    except (OSError, Story92SecurityEvidenceError):
        failures.append("Story 9.2 retained evidence is unavailable")
    else:
        if value.get("input_evidence") != expected_inputs:
            failures.append("Story 9.2 input evidence closure changed")
        if value.get("artifacts") != expected_artifacts:
            failures.append("Story 9.2 retained evidence closure is stale")
        if value.get("parity_report") != expected_parity:
            failures.append("Story 9.2 parity report changed")
    expected_summary = {
        "mapped_requirement_count": len(EXPECTED_REQUIREMENTS),
        "demonstrated_story_scope_count": 2,
        "partial_scope_count": len(EXPECTED_REQUIREMENTS) - 2,
        "product_requirements_complete": 0,
        "retained_artifact_count": len(EVIDENCE_PATHS),
        "declared_kvm_parity_dimensions": len(PARITY_DIMENSIONS),
        "technical_tasks_complete": True,
        "story_gate_complete": False,
    }
    if value.get("summary") != expected_summary:
        failures.append("Story 9.2 security summary is invalid")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used_while_mapping") is not False
        or value.get("external_independent_review_status")
        != "required-not-performed"
        or value.get("gate_blocker")
        != "G-DOD-12 independent critical-boundary review"
        or value.get("inference_claim") != "none"
        or value.get("product_requirement_completion_claim") != "none"
        or value.get("physical_host_certification_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_evidence_substituted") is not False
    ):
        failures.append("Story 9.2 security evidence made an unsupported claim")
    blockers = value.get("known_blockers")
    if not isinstance(blockers, list) or len(blockers) != 4 or not any(
        "effective UID 0" in item for item in blockers
    ):
        failures.append("Story 9.2 known blockers changed")
    return failures


def check_map(root: Path = ROOT) -> list[str]:
    try:
        value = json.loads(
            (root / REPORT_PATH.relative_to(ROOT)).read_text(encoding="utf-8")
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        return [f"cannot read Story 9.2 security evidence map: {error}"]
    return [*validate_inputs(root), *validate_map(value, root)]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
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
            raise Story92SecurityEvidenceError("; ".join(failures))
    except (
        Story92SecurityEvidenceError,
        OSError,
        UnicodeError,
        ValueError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Story 9.2 security evidence failed: {error}", file=sys.stderr)
        return 1
    print("Story 9.2 product-security evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
