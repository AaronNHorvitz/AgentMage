#!/usr/bin/env python3
"""Build a bounded Fedora/Ubuntu parity matrix from committed evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-9"
    / "story-9.1"
    / "linux-platform-parity.json"
)
SOURCE_PATHS = (
    "package.json",
    "scripts/linux_platform_parity_evidence.py",
    "tests/test_linux_platform_parity_evidence.py",
)
INPUTS = (
    (
        "clean-build",
        "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
        "clean-linux-source-build",
        "pass-linux",
    ),
    (
        "path-conformance",
        "artifacts/sprints/sprint-6/story-6.1/path-platform-conformance.json",
        "shared-logical-path-platform-conformance",
        "pass-all-available-non-macos-platforms",
    ),
    (
        "platform-contract",
        "artifacts/sprints/sprint-7/story-7.1/platform-contract-report.json",
        "platform-adapter-contract-and-manifest-evidence",
        "pass-all-available-non-macos-contract-scope",
    ),
    (
        "package-lifecycle",
        "artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json",
        "linux-clean-package-lifecycle",
        "pass-linux-clean-package-lifecycle",
    ),
    (
        "linux-controls",
        "artifacts/sprints/sprint-9/story-9.1/linux-control-verification.json",
        "linux-ipc-sandbox-control-verification",
        "pass-fedora-only",
    ),
    (
        "sandbox-attacks",
        "artifacts/sprints/sprint-9/story-9.1/linux-cross-distribution-sandbox-attacks.json",
        "linux-cross-distribution-sandbox-attacks",
        "pass-bounded-cross-distribution",
    ),
    (
        "ubuntu-native-controls",
        "artifacts/sprints/sprint-9/story-9.1/linux-native-ubuntu-control-verification.json",
        "linux-native-ubuntu-control-verification",
        "pass-native-ubuntu-kernel-controls",
    ),
    (
        "inactive-inference",
        "artifacts/sprints/sprint-9/story-9.1/linux-native-inference-boundary.json",
        "linux-native-inference-package-boundary",
        "pass-fedora-package-boundary",
    ),
    (
        "clean-image-acceptance",
        "artifacts/sprints/sprint-9/story-9.1/linux-clean-image-acceptance.json",
        "linux-clean-image-graphical-acceptance",
        "pass-clean-fedora-ubuntu-images",
    ),
)
DIMENSIONS = (
    {
        "id": "shared-adapter-contract",
        "fedora": "verified-local",
        "ubuntu": "verified-networkless-container",
        "parity": "verified",
        "evidence": ["platform-contract"],
    },
    {
        "id": "logical-path-policy",
        "fedora": "verified-local",
        "ubuntu": "verified-networkless-container",
        "parity": "verified",
        "evidence": ["path-conformance"],
    },
    {
        "id": "clean-source-build",
        "fedora": "verified-networkless-after-bootstrap",
        "ubuntu": "verified-networkless-after-bootstrap",
        "parity": "verified",
        "evidence": ["clean-build"],
    },
    {
        "id": "candidate-package-payload",
        "fedora": "verified-rpm",
        "ubuntu": "verified-deb",
        "parity": "verified-equivalent-component-manifest",
        "evidence": ["package-lifecycle"],
    },
    {
        "id": "package-lifecycle",
        "fedora": "verified-standard-user",
        "ubuntu": "verified-standard-user",
        "parity": "verified",
        "evidence": ["package-lifecycle"],
    },
    {
        "id": "inactive-inference-process",
        "fedora": "verified-self-check-only",
        "ubuntu": "verified-self-check-only",
        "parity": "verified-inactive-only",
        "evidence": ["inactive-inference", "package-lifecycle"],
    },
    {
        "id": "bubblewrap-seccomp-cgroup",
        "fedora": "verified-live",
        "ubuntu": "verified-live-native-kernel-kvm",
        "parity": "verified",
        "evidence": [
            "linux-controls",
            "sandbox-attacks",
            "ubuntu-native-controls",
        ],
    },
    {
        "id": "authenticated-ipc",
        "fedora": "verified-live",
        "ubuntu": "verified-live-native-kernel-kvm",
        "parity": "verified",
        "evidence": ["linux-controls", "ubuntu-native-controls"],
    },
    {
        "id": "secret-service",
        "fedora": "verified-live-synthetic-items",
        "ubuntu": "verified-live-native-kernel-kvm-synthetic-items",
        "parity": "verified",
        "evidence": ["linux-controls", "ubuntu-native-controls"],
    },
    {
        "id": "graphical-vscode-workflow",
        "fedora": "verified-networkless-clean-image",
        "ubuntu": "verified-networkless-clean-image",
        "parity": "verified-clean-image",
        "evidence": ["clean-image-acceptance"],
    },
)
BLOCKERS: tuple[dict[str, str], ...] = ()
LIMITATIONS = (
    "This report proves all ten declared Fedora/Ubuntu parity dimensions; it does not extend the dimension set or claim physical-host certification.",
    "Fedora controls execute on the local Fedora 44 host; Ubuntu isolation, IPC, Secret Service, startup, syscall, and resource controls execute under an Ubuntu 26.04 kernel in a KVM guest, while the older rootless-container attack artifact remains bounded historical corroboration.",
    "The graphical Visual Studio Code workflow passes in clean rootless Fedora and Ubuntu container images, not on native physical hosts; Chromium's nested sandbox is disabled only inside the outer capability-free, no-new-privileges, networkless acceptance container.",
    "The packaged inference process is inactive: no model is packaged or enabled and no inference is performed.",
    "All packages are unsigned candidates and make no supported-release claim.",
    "macOS is outside this Linux parity matrix and no macOS evidence is substituted.",
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class LinuxPlatformParityError(ValueError):
    """Raised when Linux parity evidence is stale, malformed, or overclaimed."""


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-linux-parity-", dir=path.parent)
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


def git_revision(candidate: str = "HEAD") -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise LinuxPlatformParityError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if completed.returncode != 0:
        raise LinuxPlatformParityError(f"committed source is absent: {relative}")
    return completed.stdout


def source_records(revision: str) -> list[dict[str, str]]:
    records = []
    for relative in SOURCE_PATHS:
        committed = git_file(revision, relative)
        if committed != (ROOT / relative).read_bytes():
            raise LinuxPlatformParityError(f"source differs from revision: {relative}")
        records.append({"path": relative, "sha256": sha256_bytes(committed)})
    return records


def load_inputs() -> dict[str, dict[str, Any]]:
    loaded = {}
    for evidence_id, relative, _, _ in INPUTS:
        try:
            value = json.loads((ROOT / relative).read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise LinuxPlatformParityError(
                f"cannot read parity input: {evidence_id}"
            ) from error
        if not isinstance(value, dict):
            raise LinuxPlatformParityError(f"parity input is not an object: {evidence_id}")
        loaded[evidence_id] = value
    return loaded


def _all_pass(records: Any, expected_count: int) -> bool:
    return (
        isinstance(records, list)
        and len(records) == expected_count
        and all(isinstance(item, dict) and item.get("status") == "pass" for item in records)
    )


def validate_input_evidence(values: dict[str, dict[str, Any]]) -> list[str]:
    failures: list[str] = []
    if set(values) != {item[0] for item in INPUTS}:
        return ["Linux parity input set changed"]

    clean = values["clean-build"]
    clean_runs = clean.get("platform_runs")
    if (
        clean.get("schema_version") != 2
        or clean.get("status") != "pass-linux"
        or not isinstance(clean_runs, dict)
        or set(clean_runs) != {"fedora-x86_64", "ubuntu-x86_64"}
        or any(run.get("status") != "pass" for run in clean_runs.values())
        or any(len(run.get("commands", [])) != 11 for run in clean_runs.values())
        or any(not all(command.get("status") == "pass" for command in run["commands"]) for run in clean_runs.values())
        or clean.get("container_controls", {}).get("runtime_network")
        != "disabled-after-bootstrap"
    ):
        failures.append("clean Linux build parity input is incomplete")

    paths = values["path-conformance"]
    if (
        paths.get("status") != "pass-all-available-non-macos-platforms"
        or paths.get("platform_status")
        != {
            "deterministic_fake": "verified",
            "fedora_44": "verified-local",
            "ubuntu_26_04": "verified-no-network-container",
            "macos": "blocked-macos",
        }
        or paths.get("coverage", {}).get("equivalent_policy_decision_count") != 3
        or paths.get("coverage", {}).get("networked_test_count") != 0
        or paths.get("release_claim") != "none"
        or paths.get("macos_evidence_substituted") is not False
    ):
        failures.append("path policy parity input is incomplete")

    contract = values["platform-contract"]
    conformance = contract.get("conformance", {})
    if (
        contract.get("status") != "pass-all-available-non-macos-contract-scope"
        or contract.get("api", {}).get("operating_system_branches_in_kernel_selector") != 0
        or conformance.get("fedora_contract") != "verified-local"
        or conformance.get("ubuntu_contract") != "verified-no-network-container"
        or conformance.get("networked_test_count") != 0
        or len(contract.get("manifests", [])) != 2
        or {item.get("platform") for item in contract.get("manifests", [])}
        != {"fedora", "ubuntu"}
        or contract.get("release_claim") != "none"
        or contract.get("macos_evidence_substituted") is not False
    ):
        failures.append("shared adapter contract parity input is incomplete")

    package = values["package-lifecycle"]
    lifecycle = package.get("container_lifecycle", {})
    platforms = lifecycle.get("platforms")
    expected_formats = {"fedora-x86_64": "rpm", "ubuntu-x86_64": "deb"}
    expected_payload_shape = [
        ("usr/libexec/agentmage/agentmage-docker-guard", 0o755),
        ("usr/libexec/agentmage/agentmage-docker-topology-collector", 0o755),
        ("usr/libexec/agentmage/agentmage-host", 0o755),
        ("usr/libexec/agentmage/agentmage-native-inference", 0o755),
        ("usr/share/agentmage/agentmage.vsix", 0o644),
        ("usr/share/licenses/agentmage/LICENSE", 0o644),
    ]
    platform_map = {
        item.get("platform_id"): item for item in platforms or [] if isinstance(item, dict)
    }
    manifests = package.get("component_manifests")
    manifest_files = [item.get("manifest", {}).get("files") for item in manifests or []]
    manifest_shapes = [
        [(file.get("path"), file.get("mode")) for file in files]
        for files in manifest_files
        if isinstance(files, list)
    ]
    if (
        package.get("status") != "pass-linux-clean-package-lifecycle"
        or lifecycle.get("network_used") is not False
        or set(platform_map) != set(expected_formats)
        or any(item.get("status") != "pass" for item in platform_map.values())
        or any(
            item.get("package_format") != expected_formats[platform_id]
            for platform_id, item in platform_map.items()
        )
        or any(len(item.get("steps", [])) != 48 for item in platform_map.values())
        or any(
            not all(step.get("status") == "pass" for step in item.get("steps", []))
            for item in platform_map.values()
        )
        or any(not all(item.get("checks", {}).values()) for item in platform_map.values())
        or not isinstance(manifests, list)
        or len(manifests) != 2
        or len(manifest_files) != 2
        or manifest_shapes != [expected_payload_shape, expected_payload_shape]
        or any(
            not all(
                isinstance(file.get("size"), int)
                and file.get("size", 0) > 0
                and SHA256.fullmatch(str(file.get("sha256"))) is not None
                for file in files
            )
            for files in manifest_files
            if isinstance(files, list)
        )
        or package.get("package_verification")
        != {"extracted_payloads": "pass", "mutation_refusal": "pass"}
        or package.get("enabled_models") != 0
        or package.get("inference_available") is not False
        or package.get("release_claim") != "none"
        or package.get("private_values_present") is not False
    ):
        failures.append("clean package lifecycle parity input is incomplete")

    controls = values["linux-controls"]
    if (
        controls.get("status") != "pass-fedora-only"
        or controls.get("platform_status")
        != {
            "fedora_44_x86_64": "verified-local",
            "ubuntu_26_04_x86_64": "verified-native-kernel-evidence-recorded-separately",
            "macos": "blocked-macos",
        }
        or not _all_pass(controls.get("sandbox_tests"), 17)
        or not _all_pass(controls.get("ipc_tests"), 8)
        or not _all_pass(controls.get("secret_service_tests"), 4)
        or not _all_pass(controls.get("secret_service_live_tests"), 3)
        or controls.get("startup_control_ids")
        != [
            "bubblewrap",
            "user-namespaces",
            "seccomp",
            "cgroups-v2",
            "secret-service",
            "descriptor-safe-paths",
            "network-isolation",
        ]
        or controls.get("startup_mutation_statuses") != ["unavailable", "invalid"]
        or controls.get("startup_platform_families") != ["fedora", "ubuntu"]
        or not _all_pass(controls.get("startup_mapping_tests"), 2)
        or not _all_pass(controls.get("kernel_startup_tests"), 1)
        or not _all_pass(controls.get("startup_live_tests"), 1)
        or len(controls.get("attack_coverage", {})) != 16
        or set(controls.get("attack_coverage", {}).values()) != {"pass"}
        or controls.get("release_claim") != "none"
        or controls.get("private_values_present") is not False
    ):
        failures.append("live Linux control parity input is incomplete")

    attacks = values["sandbox-attacks"]
    attack_platforms = attacks.get("platforms", {})
    if not isinstance(attack_platforms, dict):
        attack_platforms = {}
    fedora_attacks = attack_platforms.get("fedora-44-x86_64", {})
    ubuntu_attacks = attack_platforms.get("ubuntu-26.04-x86_64", {})
    attack_results = attacks.get("attack_results")
    if (
        attacks.get("status") != "pass-bounded-cross-distribution"
        or set(attack_platforms) != {"fedora-44-x86_64", "ubuntu-26.04-x86_64"}
        or fedora_attacks.get("execution_context") != "native-standard-user"
        or fedora_attacks.get("native_platform_claim") is not True
        or not _all_pass(fedora_attacks.get("tests"), 11)
        or ubuntu_attacks.get("execution_context")
        != "standard-user-in-rootless-privileged-test-envelope"
        or ubuntu_attacks.get("native_platform_claim") is not False
        or ubuntu_attacks.get("container_controls", {}).get("network") != "none"
        or ubuntu_attacks.get("container_controls", {}).get("podman_rootless")
        is not True
        or ubuntu_attacks.get("container_controls", {}).get("outer_privileged_flag")
        is not True
        or not _all_pass(ubuntu_attacks.get("tests"), 11)
        or not isinstance(attack_results, list)
        or len(attack_results) != 8
        or not all(isinstance(item, dict) for item in attack_results)
        or any(item.get("fedora") != "pass-zero-escape" for item in attack_results)
        or any(
            item.get("ubuntu") != "pass-zero-escape-in-declared-envelope"
            for item in attack_results
        )
        or attacks.get("summary", {}).get("native_ubuntu_isolation_verified")
        is not False
        or attacks.get("release_claim") != "none"
        or attacks.get("private_values_present") is not False
    ):
        failures.append("bounded cross-distribution sandbox input is incomplete")

    native = values["ubuntu-native-controls"]
    native_execution = native.get("execution", {})
    native_platform = native_execution.get("platform", {})
    native_tests = native_execution.get("tests", {})
    if (
        native.get("status") != "pass-native-ubuntu-kernel-controls"
        or native_platform.get("distribution") != "ubuntu"
        or native_platform.get("version") != "26.04"
        or native_platform.get("architecture") != "x86_64"
        or native_platform.get("virtualization") != "kvm"
        or native_platform.get("cgroup_filesystem") != "cgroup2"
        or native_platform.get("uid") != 10001
        or native_platform.get("gid") != 10001
        or native_execution.get("network", {}).get("qemu_restrict_mode") is not True
        or native_execution.get("network", {}).get("external_connection_denied")
        is not True
        or not _all_pass(native_tests.get("sandbox_live"), 11)
        or not _all_pass(native_tests.get("ipc"), 8)
        or not _all_pass(native_tests.get("secret_service_static"), 4)
        or not _all_pass(native_tests.get("secret_service_live"), 3)
        or not _all_pass(native_tests.get("startup_mapping"), 2)
        or not _all_pass(native_tests.get("startup_live"), 1)
        or not _all_pass(native_tests.get("kernel_startup_refusal"), 1)
        or native_execution.get("syscall_trace", {}).get("no_new_privileges") != 1
        or native_execution.get("syscall_trace", {}).get("seccomp_mode") != 2
        or native_execution.get("resource_trace", {}).get("runtime_limit_enforced")
        is not True
        or set(native_execution.get("cleanup", {}).values()) != {True}
        or native.get("summary", {}).get("native_ubuntu_kernel_controls_verified")
        is not True
        or native.get("summary", {}).get("physical_host_certification") is not False
        or native.get("summary", {}).get("test_count") != 30
        or native.get("release_claim") != "none"
        or native.get("private_values_present") is not False
    ):
        failures.append("native Ubuntu control parity input is incomplete")

    inference = values["inactive-inference"]
    process = inference.get("process_boundary", {})
    if (
        inference.get("status") != "pass-fedora-package-boundary"
        or inference.get("enabled_models") != 0
        or inference.get("model_artifacts_packaged") != 0
        or inference.get("inference_started") is not False
        or inference.get("network_used") is not False
        or process.get("accepted_operation") != "self-check-only"
        or process.get("authority_inputs") != []
        or process.get("inference_available") is not False
        or process.get("network_listener") is not False
        or inference.get("release_claim") != "none"
        or inference.get("private_values_present") is not False
    ):
        failures.append("inactive inference parity input is incomplete")

    acceptance = values["clean-image-acceptance"]
    acceptance_platforms = acceptance.get("platforms")
    acceptance_map = {
        item.get("platform_id"): item
        for item in acceptance_platforms or []
        if isinstance(item, dict)
    }
    acceptance_formats = {"fedora-x86_64": "rpm", "ubuntu-x86_64": "deb"}
    expected_steps = [
        "platform-identity",
        "standard-user-identity",
        "system-package-install",
        "package-version",
        "package-files",
        "native-host-launch",
        "inactive-inference-launch",
        "inactive-docker-guard-launch",
        "inactive-docker-collector-launch",
        "vscode-version",
        "extension-install",
        "extension-registration",
        "vscode-launch-and-provider-exercise",
        "standard-user-process-boundary",
        "vscode-shutdown",
        "extension-uninstall",
        "extension-registration-absent",
        "system-package-uninstall",
        "package-record-absent",
        "isolated-profile-cleanup",
        "final-residue-scan",
    ]
    expected_provider = {
        "family": "agentmage-secure-read",
        "id": "secure-local-read",
        "tokenCount": 4,
        "vendor": "agentmage",
        "version": "0.0.0-phase9",
    }
    if (
        acceptance.get("schema_version") != 1
        or acceptance.get("task_ids") != ["9.1.3.4"]
        or acceptance.get("status") != "pass-clean-fedora-ubuntu-images"
        or acceptance.get("rootless_runtime") is not True
        or acceptance.get("acceptance_network_used") is not False
        or acceptance.get("bootstrap_network_separate") is not True
        or acceptance.get("enabled_models") != 0
        or acceptance.get("inference_performed") is not False
        or acceptance.get("private_values_present") is not False
        or acceptance.get("release_claim") != "none"
        or acceptance.get("published_procedure", {}).get("path")
        != "docs/support/linux-clean-image-acceptance.md"
        or set(acceptance_map) != set(acceptance_formats)
        or any(
            item.get("package_format") != acceptance_formats[platform_id]
            or item.get("actors")
            != {
                "agentmage_and_vscode_user": "10001:10001",
                "package_administrator": "0:0",
            }
            or [step.get("id") for step in item.get("steps", [])] != expected_steps
            or not _all_pass(item.get("steps"), 21)
            or item.get("container_controls", {}).get("runtime")
            != "rootless-podman"
            or item.get("container_controls", {}).get("network") != "none"
            or item.get("container_controls", {}).get("privileged") is not False
            or item.get("container_controls", {}).get("capabilities_added") != []
            or not item.get("container_controls", {}).get("capabilities_dropped")
            or item.get("container_controls", {}).get("no_new_privileges") is not True
            or item.get("container_controls", {}).get("seccomp_mode") != 2
            or item.get("container_controls", {}).get("package_mount") != "read-only"
            or item.get("container_controls", {}).get("probe_mount") != "read-only"
            or item.get("process_boundary", {}).get("standard_user_only") is not True
            or item.get("vscode", {}).get("version") != "1.132.0"
            or REVISION.fullmatch(str(item.get("vscode", {}).get("commit"))) is None
            or item.get("vscode", {}).get("extension_id")
            != "agentmage-project.agentmage-vscode-shell"
            or item.get("vscode", {}).get("chromium_sandbox")
            != "disabled-inside-outer-test-container-only"
            or item.get("vscode", {}).get("provider_probe", {}).get("schemaVersion")
            != 1
            or item.get("vscode", {}).get("provider_probe", {}).get("status") != "pass"
            or item.get("vscode", {}).get("provider_probe", {}).get("observed")
            != expected_provider
            or item.get("inference_descriptor", {}).get("enabled_models") != 0
            or item.get("inference_descriptor", {}).get("inference_available") is not False
            or item.get("inference_descriptor", {}).get("authority_inputs") != []
            or set(item.get("residue", {}).values()) != {False}
            or item.get("container_removed") is not True
            for platform_id, item in acceptance_map.items()
        )
    ):
        failures.append("clean graphical image acceptance input is incomplete")
    return failures


def input_revision(evidence_id: str, value: dict[str, Any]) -> str:
    if evidence_id in {
        "clean-build",
        "package-lifecycle",
        "clean-image-acceptance",
    }:
        revision = value.get("source", {}).get("revision")
    elif evidence_id in {"path-conformance", "platform-contract"}:
        revision = value.get("reference_revision")
    else:
        revision = value.get("source_revision")
    if not isinstance(revision, str) or REVISION.fullmatch(revision) is None:
        raise LinuxPlatformParityError(f"parity input revision is invalid: {evidence_id}")
    return revision


def input_records(values: dict[str, dict[str, Any]]) -> list[dict[str, str]]:
    records = []
    for evidence_id, relative, artifact_id, status in INPUTS:
        value = values[evidence_id]
        actual_artifact = value.get("artifact_id", artifact_id)
        if actual_artifact != artifact_id or value.get("status") != status:
            raise LinuxPlatformParityError(f"parity input identity changed: {evidence_id}")
        records.append(
            {
                "id": evidence_id,
                "path": relative,
                "artifact_id": artifact_id,
                "status": status,
                "evidence_revision": input_revision(evidence_id, value),
                "sha256": sha256_bytes((ROOT / relative).read_bytes()),
            }
        )
    return records


def build_report(revision: str) -> dict[str, Any]:
    values = load_inputs()
    failures = validate_input_evidence(values)
    if failures:
        raise LinuxPlatformParityError("; ".join(failures))
    return {
        "schema_version": 1,
        "artifact_id": "linux-fedora-ubuntu-adapter-parity",
        "task_ids": ["9.1.2.4"],
        "status": "pass-declared-fedora-ubuntu-parity",
        "source_revision": revision,
        "sources": source_records(revision),
        "platform_scope": ["fedora-44-x86_64", "ubuntu-26.04-x86_64"],
        "input_evidence": input_records(values),
        "dimensions": list(DIMENSIONS),
        "summary": {
            "dimension_count": 10,
            "verified_parity_dimensions": 10,
            "blocked_parity_dimensions": 0,
            "full_fedora_ubuntu_parity": True,
        },
        "blocking_gates": list(BLOCKERS),
        "private_values_present": False,
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "limitations": list(LIMITATIONS),
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Linux platform parity report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-fedora-ubuntu-adapter-parity"
        or value.get("task_ids") != ["9.1.2.4"]
        or value.get("status") != "pass-declared-fedora-ubuntu-parity"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Linux platform parity report identity changed")
    if value.get("platform_scope") != ["fedora-44-x86_64", "ubuntu-26.04-x86_64"]:
        failures.append("Linux platform parity scope changed")
    records = value.get("input_evidence")
    expected = [(item[0], item[1], item[2], item[3]) for item in INPUTS]
    observed = (
        [
            (item.get("id"), item.get("path"), item.get("artifact_id"), item.get("status"))
            for item in records
        ]
        if isinstance(records, list)
        else []
    )
    if (
        observed != expected
        or any(REVISION.fullmatch(str(item.get("evidence_revision"))) is None for item in records or [])
        or any(SHA256.fullmatch(str(item.get("sha256"))) is None for item in records or [])
    ):
        failures.append("Linux platform parity input closure changed")
    if value.get("dimensions") != list(DIMENSIONS):
        failures.append("Linux platform parity dimensions changed")
    if value.get("summary") != {
        "dimension_count": 10,
        "verified_parity_dimensions": 10,
        "blocked_parity_dimensions": 0,
        "full_fedora_ubuntu_parity": True,
    }:
        failures.append("Linux platform parity summary overclaimed or changed")
    if value.get("blocking_gates") != list(BLOCKERS):
        failures.append("Linux platform parity blockers changed")
    sources = value.get("sources")
    if (
        not isinstance(sources, list)
        or [item.get("path") for item in sources] != list(SOURCE_PATHS)
        or any(SHA256.fullmatch(str(item.get("sha256"))) is None for item in sources)
    ):
        failures.append("Linux platform parity source closure changed")
    if (
        value.get("private_values_present") is not False
        or value.get("macos_evidence_substituted") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Linux platform parity report made an unsupported claim")
    if value.get("limitations") != list(LIMITATIONS):
        failures.append("Linux platform parity limitations changed")
    return failures


def write_report(revision: str) -> None:
    write_atomic(REPORT_PATH, pretty_json(build_report(revision)))


def check_report() -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["source_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise LinuxPlatformParityError(f"cannot read Linux parity report: {error}") from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision):
        failures.append("Linux platform parity report is stale or malformed")
    if failures:
        raise LinuxPlatformParityError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            write_report(git_revision(arguments.source_revision))
        check_report()
    except (OSError, UnicodeError, LinuxPlatformParityError, subprocess.SubprocessError) as error:
        print(f"Linux platform parity evidence failed: {error}", file=sys.stderr)
        return 1
    print("All ten declared Fedora/Ubuntu parity dimensions validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
