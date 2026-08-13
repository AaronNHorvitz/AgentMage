#!/usr/bin/env python3
"""Build and validate the approved and non-approved component inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

try:
    from scripts.clean_build_evidence import (
        validate_report as validate_clean_build_report,
    )
    from scripts.no_install_diagnostic import (
        validate_inventory as validate_optional_inventory,
    )
    from scripts.supply_chain import (
        validate_documents as validate_supply_chain_documents,
    )
except ModuleNotFoundError:
    from clean_build_evidence import validate_report as validate_clean_build_report
    from no_install_diagnostic import validate_inventory as validate_optional_inventory
    from supply_chain import validate_documents as validate_supply_chain_documents


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "architecture/component-inventory-policy.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json"
HASH = re.compile(r"^[0-9a-f]{64}$")
IMAGE_DIGEST = re.compile(r"^sha256:([0-9a-f]{64})$")
EXPECTED_EXECUTABLES = (
    "cargo",
    "clippy-driver",
    "node",
    "npm",
    "python3",
    "rustc",
    "rustfmt",
)
EXPECTED_PLATFORMS = ("fedora-x86_64", "ubuntu-x86_64")
EXPECTED_RUNTIME_CANDIDATES = (
    "linux-native-llama-cpp-b10333",
    "linux-docker-model-runner-b9879",
    "macos-native-llama-cpp-b10333",
)
EXPECTED_UNAPPROVED_COMPONENTS = (
    "linux-bubblewrap",
    "linux-resource-control",
    "linux-secret-service-client",
    "docker-compatibility",
    "rust-development",
    "node-development",
    "python-development",
    "macos-swift-build",
    "macos-xcode-build",
)
EXPECTED_CONTROLS = {
    "approved_records_require_version": True,
    "approved_records_require_cryptographic_hash": True,
    "candidate_identity_is_not_release_approval": True,
    "presence_is_not_approval": True,
    "mutable_runtime_tags_are_release_identities": False,
    "unknown_components_fail_closed": True,
}
EXPECTED_SOURCES = {
    "clean_build_report": "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
    "dependency_provenance": "supply-chain/dependency-provenance.json",
    "dependency_classes": "architecture/dependency-classes.json",
    "optional_components": "architecture/optional-component-inventory.json",
    "linux_native_runtime_package": "model-profiles/runtimes/llama-cpp-b10333-linux-x86_64.json",
    "linux_docker_runtime_profile": "model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json",
    "linux_primary_runtime_candidate": "model-profiles/candidates/gemma-4-e4b/artifact-admission.json",
    "linux_fallback_runtime_candidate": "model-profiles/candidates/gemma-4-12b-unified/artifact-admission.json",
    "macos_runtime_candidate": "model-profiles/runtimes/llama-cpp-b10333-macos-arm64.json",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-component-inventory-", dir=path.parent
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


def validate_policy(policy: Any) -> list[str]:
    if not isinstance(policy, dict):
        return ["component inventory policy must be an object"]
    failures = []
    if (
        policy.get("schema_version") != 1
        or policy.get("task_id") != "3.1.1.6"
        or policy.get("status") != "enforced"
    ):
        failures.append("component inventory policy identity is invalid")
    if tuple(policy.get("approved_build_executable_ids", [])) != EXPECTED_EXECUTABLES:
        failures.append("approved build executable closure is invalid")
    if tuple(policy.get("approved_build_platforms", [])) != EXPECTED_PLATFORMS:
        failures.append("approved build platform closure is invalid")
    if policy.get("approved_product_runtime_ids") != []:
        failures.append("a product runtime was approved before its owning gate")
    if policy.get("approved_optional_dependency_ids") != []:
        failures.append("an optional dependency was approved before its owning gate")
    if tuple(policy.get("candidate_runtime_ids", [])) != EXPECTED_RUNTIME_CANDIDATES:
        failures.append("candidate runtime closure is invalid")
    if tuple(policy.get("unapproved_component_ids", [])) != EXPECTED_UNAPPROVED_COMPONENTS:
        failures.append("unapproved component closure is invalid")
    if policy.get("controls") != EXPECTED_CONTROLS:
        failures.append("component inventory controls were weakened")
    if policy.get("platform_status") != {
        "fedora": "verified-clean-build",
        "ubuntu": "verified-clean-build",
        "macos": "blocked-macos",
    }:
        failures.append("component inventory platform status is invalid")
    if policy.get("sources") != EXPECTED_SOURCES:
        failures.append("component inventory source closure is invalid")
    return failures


def _package_summary(provenance: dict[str, Any]) -> dict[str, Any]:
    identities = []
    classifications: Counter[str] = Counter()
    ecosystems: Counter[str] = Counter()
    for component in provenance["components"]:
        component_id = component.get("component_id")
        version = component.get("version")
        hashes = component.get("hashes")
        if not isinstance(component_id, str) or not component_id:
            raise ValueError("package component identity is missing")
        if not isinstance(version, str) or not version:
            raise ValueError(f"package component version is missing: {component_id}")
        if not isinstance(hashes, list) or not hashes:
            raise ValueError(f"package component hash is missing: {component_id}")
        for item in hashes:
            expected_lengths = {"SHA-256": 64, "SHA-384": 96, "SHA-512": 128}
            expected_length = expected_lengths.get(item.get("algorithm"))
            content = item.get("content", "")
            if expected_length is None or not re.fullmatch(
                rf"[0-9a-f]{{{expected_length}}}", content
            ):
                raise ValueError(
                    f"package component cryptographic hash is invalid: {component_id}"
                )
        identities.append(
            {
                "component_id": component_id,
                "hashes": hashes,
                "version": version,
            }
        )
        classifications[component["classification"]] += 1
        ecosystems[component["ecosystem"]] += 1
    return {
        "source": "supply-chain/dependency-provenance.json",
        "component_count": len(identities),
        "identity_set_sha256": sha256_bytes(
            json.dumps(identities, sort_keys=True, separators=(",", ":")).encode()
        ),
        "classifications": dict(sorted(classifications.items())),
        "ecosystems": dict(sorted(ecosystems.items())),
        "all_versions_present": True,
        "all_cryptographic_hashes_present": True,
    }


def _approved_executables(clean_report: dict[str, Any]) -> list[dict[str, str]]:
    records = []
    for platform_id in EXPECTED_PLATFORMS:
        toolchains = clean_report["platform_runs"][platform_id]["toolchains"]
        if tuple(item.get("id") for item in toolchains) != EXPECTED_EXECUTABLES:
            raise ValueError(f"approved executable closure drifted: {platform_id}")
        for item in toolchains:
            version = item.get("version")
            digest = item.get("executable_sha256")
            if not isinstance(version, str) or not version or not HASH.fullmatch(digest or ""):
                raise ValueError(f"approved executable identity is incomplete: {platform_id}")
            records.append(
                {
                    "component_id": f"executable:{platform_id}:{item['id']}",
                    "executable_id": item["id"],
                    "platform": platform_id,
                    "version": version,
                    "sha256": digest,
                    "approval_scope": "build-and-test-only",
                }
            )
    return records


def _build_environments(clean_report: dict[str, Any]) -> list[dict[str, str]]:
    records = []
    for platform_id in EXPECTED_PLATFORMS:
        run = clean_report["platform_runs"][platform_id]
        image_match = IMAGE_DIGEST.fullmatch(run.get("container_image_id", ""))
        base_match = re.search(r"@sha256:([0-9a-f]{64})$", run.get("base_image", ""))
        if image_match is None or base_match is None:
            raise ValueError(f"build environment identity is incomplete: {platform_id}")
        records.append(
            {
                "component_id": f"build-environment:{platform_id}",
                "platform": platform_id,
                "version": run["platform"]["pretty_name"],
                "sha256": image_match.group(1),
                "base_image_sha256": base_match.group(1),
                "approval_scope": "build-and-test-only",
            }
        )
    return records


def _runtime_candidates(
    linux_native: dict[str, Any],
    linux_native_profile_sha256: str,
    linux_docker: dict[str, Any],
    linux_docker_profile_sha256: str,
    primary: dict[str, Any],
    fallback: dict[str, Any],
    macos: dict[str, Any],
) -> list[dict[str, Any]]:
    for admission in (primary, fallback):
        if admission.get("decision", {}).get("release_approval") is not False:
            raise ValueError("Linux runtime candidate was promoted without its owning gate")
    if macos.get("decision", {}).get("release_approval") is not False:
        raise ValueError("macOS runtime candidate was promoted without its owning gate")
    if linux_native.get("decision") != {
        "enabled_models": 0,
        "inference_implemented": False,
        "release_approval": False,
        "status": "PACKAGE_INPUT_PINNED_NOT_ACTIVATED",
    }:
        raise ValueError("Linux native runtime package was promoted without its owning gate")
    if linux_docker.get("decision") != {
        "docker_engine_directly_tested": False,
        "enabled_models": 0,
        "inference_implemented": False,
        "release_approval": False,
        "status": "COMPATIBILITY_PROFILE_PINNED_NOT_ACTIVATED",
    }:
        raise ValueError("Linux Docker compatibility profile was promoted without its owning gate")
    if (
        HASH.fullmatch(linux_native_profile_sha256) is None
        or linux_native.get("package", {}).get("package_id")
        != "agentmage-llama-cpp-b10333-cpu-linux-x86_64"
    ):
        raise ValueError("Linux native runtime package profile identity is invalid")
    primary_native = primary["native_runtime"]
    fallback_native = fallback["native_runtime"]
    primary_docker = primary["docker_engine"]
    fallback_docker = fallback["docker_engine"]
    for key in ("release", "source_commit", "asset_sha256"):
        if primary_native.get(key) != fallback_native.get(key):
            raise ValueError(f"fallback native runtime identity differs: {key}")
    profile_archive = linux_native.get("source_archive", {})
    if (
        linux_native.get("release") != primary_native.get("release")
        or linux_native.get("source_commit") != primary_native.get("source_commit")
        or profile_archive.get("sha256") != primary_native.get("asset_sha256")
        or linux_native.get("backend") != "cpu"
        or linux_native.get("authority")
        != {
            "credential": False,
            "grant": False,
            "network": False,
            "tool": False,
            "workspace": False,
        }
    ):
        raise ValueError("Linux native runtime package identity differs from retained evidence")
    retained_files = linux_native.get("source_files")
    if not isinstance(retained_files, list) or any(
        not isinstance(item, dict) for item in retained_files
    ):
        raise ValueError("Linux native runtime retained file closure is invalid")
    retained_libraries = {
        item["destination"]: item["sha256"]
        for item in retained_files
        if item.get("type") == "file" and str(item.get("destination", "")).startswith("lib/")
    }
    if (
        not retained_libraries
        or any(
            term in str(item.get("destination", "")).lower()
            for item in retained_files
            for term in ("server", "rpc", "cli", "vulkan", "download")
        )
    ):
        raise ValueError("Linux native runtime package retained a prohibited surface")
    for key in ("digest", "runtime_source_revision", "runtime_version"):
        if primary_docker.get(key) != fallback_docker.get(key):
            raise ValueError(f"fallback Docker runtime identity differs: {key}")
    docker_digest = IMAGE_DIGEST.fullmatch(primary_docker["digest"])
    if docker_digest is None:
        raise ValueError("Docker runtime candidate digest is invalid")
    profile_docker_digest = IMAGE_DIGEST.fullmatch(
        linux_docker.get("engine", {}).get("manifest_digest", "")
    )
    model_digest = IMAGE_DIGEST.fullmatch(
        linux_docker.get("model_artifact", {}).get("manifest_digest", "")
    )
    if (
        HASH.fullmatch(linux_docker_profile_sha256) is None
        or profile_docker_digest is None
        or model_digest is None
        or profile_docker_digest.group(1) != docker_digest.group(1)
        or linux_docker.get("engine", {}).get("runtime_source_revision")
        != primary_docker.get("runtime_source_revision")
        or linux_docker.get("engine", {}).get("runtime_version")
        != primary_docker.get("runtime_version")
        or linux_docker.get("package")
        != {
            "install_authority": "separate-administrator-operation",
            "package_id": "docker-model-plugin",
            "package_version": "1.2.6",
            "replacement": "exact-version-only",
        }
    ):
        raise ValueError("Linux Docker compatibility profile identity is invalid")
    candidates = [
        {
            "component_id": EXPECTED_RUNTIME_CANDIDATES[0],
            "package_id": linux_native["package"]["package_id"],
            "platform": "linux-x86_64",
            "profile_sha256": linux_native_profile_sha256,
            "version": primary_native["release"],
            "source_revision": primary_native["source_commit"],
            "sha256": profile_archive["sha256"],
            "backend": "cpu-library-only",
            "critical_sha256": dict(sorted(retained_libraries.items())),
            "upstream_entrypoints_included": [],
            "approval_status": "candidate-not-approved",
        },
        {
            "component_id": EXPECTED_RUNTIME_CANDIDATES[1],
            "package_id": linux_docker["package"]["package_id"],
            "platform": "linux-x86_64",
            "version": primary_docker["runtime_version"],
            "source_revision": primary_docker["runtime_source_revision"],
            "sha256": docker_digest.group(1),
            "profile_sha256": linux_docker_profile_sha256,
            "model_manifest_sha256": model_digest.group(1),
            "docker_engine_directly_tested": False,
            "approval_status": "candidate-not-approved",
        },
        {
            "component_id": EXPECTED_RUNTIME_CANDIDATES[2],
            "platform": "macos-arm64",
            "version": macos["release"],
            "source_revision": macos["source_commit"],
            "sha256": macos["artifact"]["sha256"],
            "critical_sha256": {
                key.replace("_", "-"): value["sha256"]
                for key, value in sorted(macos["critical_files"].items())
            },
            "approval_status": "candidate-not-approved",
        },
    ]
    for candidate in candidates:
        if not HASH.fullmatch(candidate["sha256"]):
            raise ValueError(f"runtime candidate SHA-256 is invalid: {candidate['component_id']}")
        for digest in candidate.get("critical_sha256", {}).values():
            if not HASH.fullmatch(digest):
                raise ValueError(
                    f"runtime candidate critical SHA-256 is invalid: {candidate['component_id']}"
                )
    return candidates


def build_report(root: Path = ROOT) -> dict[str, Any]:
    policy = read_json(root / "architecture/component-inventory-policy.json")
    failures = validate_policy(policy)
    if failures:
        raise ValueError("; ".join(failures))
    source_paths = list(policy["sources"].values())
    sources = {path: read_json(root / path) for path in source_paths}
    clean = sources[policy["sources"]["clean_build_report"]]
    provenance = sources[policy["sources"]["dependency_provenance"]]
    classes = sources[policy["sources"]["dependency_classes"]]
    optional = sources[policy["sources"]["optional_components"]]
    primary = sources[policy["sources"]["linux_primary_runtime_candidate"]]
    fallback = sources[policy["sources"]["linux_fallback_runtime_candidate"]]
    linux_native = sources[policy["sources"]["linux_native_runtime_package"]]
    linux_docker = sources[policy["sources"]["linux_docker_runtime_profile"]]
    macos = sources[policy["sources"]["macos_runtime_candidate"]]
    source_failures = [
        *validate_clean_build_report(clean, root),
        *validate_optional_inventory(optional),
    ]
    sbom = read_json(root / "supply-chain/sbom.cdx.json")
    hash_text = (root / "supply-chain/dependency-hashes.sha256").read_text(
        encoding="utf-8"
    )
    source_failures.extend(
        validate_supply_chain_documents(provenance, sbom, hash_text, root)
    )
    if source_failures:
        raise ValueError("component inventory source validation failed")
    if classes.get("optional_later_capabilities") != {
        "packages": [],
        "enabled": False,
        "included_in_default_build": False,
        "included_in_release": False,
    }:
        raise ValueError("optional dependency class is not closed and disabled")

    executables = _approved_executables(clean)
    environments = _build_environments(clean)
    packages = _package_summary(provenance)
    candidates = _runtime_candidates(
        linux_native,
        sha256_file(root / policy["sources"]["linux_native_runtime_package"]),
        linux_docker,
        sha256_file(root / policy["sources"]["linux_docker_runtime_profile"]),
        primary,
        fallback,
        macos,
    )
    unapproved = [
        {
            "component_id": item["id"],
            "executable": item["executable"],
            "classification": item["classification"],
            "platforms": item["platforms"],
            "version": None,
            "sha256": None,
            "approval_status": "presence-only-not-approved",
        }
        for item in optional["components"]
    ]
    if tuple(item["component_id"] for item in unapproved) != EXPECTED_UNAPPROVED_COMPONENTS:
        raise ValueError("unapproved optional component closure drifted")
    approved_count = len(executables) + len(environments) + packages["component_count"]
    return {
        "schema_version": 1,
        "task_id": "3.1.1.6",
        "status": "pass-shared-linux-versioned-hashed-component-inventory",
        "source_artifacts": [
            {"path": "architecture/component-inventory-policy.json", "sha256": sha256_file(root / "architecture/component-inventory-policy.json")},
            *[
                {"path": path, "sha256": sha256_file(root / path)}
                for path in source_paths
            ],
            {"path": "supply-chain/sbom.cdx.json", "sha256": sha256_file(root / "supply-chain/sbom.cdx.json")},
            {"path": "supply-chain/dependency-hashes.sha256", "sha256": sha256_file(root / "supply-chain/dependency-hashes.sha256")},
            {"path": "scripts/component_inventory.py", "sha256": sha256_file(root / "scripts/component_inventory.py")},
            {"path": "tests/test_component_inventory.py", "sha256": sha256_file(root / "tests/test_component_inventory.py")},
        ],
        "approved_inventory": {
            "build_executables": executables,
            "build_environments": environments,
            "packages": packages,
            "product_runtimes": [],
            "optional_dependencies": [],
        },
        "non_approved_inventory": {
            "runtime_candidates": candidates,
            "presence_only_components": unapproved,
        },
        "summary": {
            "approved_build_executable_count": len(executables),
            "approved_build_environment_count": len(environments),
            "approved_package_count": packages["component_count"],
            "approved_product_runtime_count": 0,
            "approved_optional_dependency_count": 0,
            "approved_component_count": approved_count,
            "approved_records_missing_version_count": 0,
            "approved_records_missing_hash_count": 0,
            "candidate_runtime_count": len(candidates),
            "presence_only_unapproved_component_count": len(unapproved),
        },
        "controls": policy["controls"],
        "product_runtime_activation_claim": "none",
        "optional_dependency_activation_claim": "none",
        "platform_execution": ["fedora-x86_64", "ubuntu-x86_64"],
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["component inventory report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.1.6"
        or value.get("status")
        != "pass-shared-linux-versioned-hashed-component-inventory"
    ):
        failures.append("component inventory report identity is invalid")
    approved = value.get("approved_inventory", {})
    summary = value.get("summary", {})
    if (
        approved.get("product_runtimes") != []
        or approved.get("optional_dependencies") != []
        or summary.get("approved_product_runtime_count") != 0
        or summary.get("approved_optional_dependency_count") != 0
        or summary.get("approved_records_missing_version_count") != 0
        or summary.get("approved_records_missing_hash_count") != 0
    ):
        failures.append("component inventory made an unsupported approval claim")
    if value.get("controls") != EXPECTED_CONTROLS:
        failures.append("component inventory controls were weakened")
    if (
        value.get("product_runtime_activation_claim") != "none"
        or value.get("optional_dependency_activation_claim") != "none"
        or value.get("platform_execution") != list(EXPECTED_PLATFORMS)
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("component inventory made an unsupported platform or product claim")
    try:
        expected = build_report(root)
    except (KeyError, OSError, TypeError, ValueError) as error:
        failures.append(f"cannot rebuild component inventory report: {error}")
    else:
        if value != expected:
            failures.append("component inventory report is stale or non-deterministic")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        value = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read component inventory report: {error}"]
    return validate_report(value, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_artifact()
    except (KeyError, OSError, TypeError, ValueError) as error:
        print(f"component inventory failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"component inventory failed: {failure}")
        return 1
    print("Story 3.1 versioned and hashed component inventory validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
