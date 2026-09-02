#!/usr/bin/env python3
"""Exercise the Rust build orchestrator through package and verifier neighbors."""

from __future__ import annotations

import argparse
import hashlib
import string
import json
import os
import platform
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final

try:
    from scripts.package_candidate import MANIFEST_PATH
    from scripts.package_lifecycle import extract_deb, extract_rpm
except ModuleNotFoundError:
    from package_candidate import MANIFEST_PATH
    from package_lifecycle import extract_deb, extract_rpm


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT
    / "artifacts/sprints/sprint-1/story-1.1/build-orchestrator-integration-report.json"
)
PRIVATE_KEY_DER_PREFIX: Final = bytes.fromhex("302e020100300506032b657004220420")
SOURCE_PATHS: Final = (
    "release/xtask/src/main.rs",
    "scripts/build_orchestrator_integration.py",
    "scripts/package_candidate.py",
    "scripts/package_release.py",
    "scripts/package_lifecycle.py",
    "shells/host/src/cli.rs",
    "packaging/linux/debian-release-control.in",
    "packaging/linux/agentmage-release.spec.in",
    "tests/test_build_orchestrator_integration.py",
)
ARTIFACT_KEYS: Final = ("deb", "manifest", "rpm", "vsix")


class IntegrationError(ValueError):
    """Raised when the integration exercise or its report fails closed."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run(
    arguments: list[str],
    *,
    input_bytes: bytes | None = None,
    success: bool = True,
) -> subprocess.CompletedProcess[bytes]:
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=600,
    )
    if (result.returncode == 0) != success:
        command = Path(arguments[0]).name
        raise IntegrationError(
            f"build_orchestrator.command_result:{command}:"
            f"exit={result.returncode}:expected_success={str(success).lower()}"
        )
    return result


def raw_public_key(seed: bytes) -> bytes:
    result = run(
        ["openssl", "pkey", "-inform", "DER", "-pubout", "-outform", "DER"],
        input_bytes=PRIVATE_KEY_DER_PREFIX + seed,
    )
    if len(result.stdout) != 44:
        raise IntegrationError("build_orchestrator.public_key_derivation_failed")
    return result.stdout[-32:]


def parse_bundle_output(output: bytes, expected_root: Path) -> dict[str, Path]:
    try:
        value = json.loads(output)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise IntegrationError("build_orchestrator.bundle_output_invalid") from error
    if not isinstance(value, dict) or tuple(sorted(value)) != ARTIFACT_KEYS:
        raise IntegrationError("build_orchestrator.artifact_set_invalid")
    artifacts = {name: Path(path).resolve() for name, path in value.items()}
    expected_root = expected_root.resolve()
    for name, path in artifacts.items():
        try:
            path.relative_to(expected_root)
        except ValueError as error:
            raise IntegrationError(
                f"build_orchestrator.artifact_path_outside_output:{name}"
            ) from error
        if not path.is_file() or path.is_symlink() or path.stat().st_size <= 0:
            raise IntegrationError(f"build_orchestrator.artifact_unavailable:{name}")
    return artifacts


def build_with_orchestrator(output: Path) -> dict[str, Path]:
    result = run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "agentmage-xtask",
            "--locked",
            "--",
            "package-release-bundle",
            "--version",
            "0.1.0",
            "--release-sequence",
            "1",
            "--output",
            str(output),
        ]
    )
    return parse_bundle_output(result.stdout, output)


def compare_bundles(first: dict[str, Path], second: dict[str, Path]) -> dict[str, str]:
    digests = {name: sha256_file(path) for name, path in sorted(first.items())}
    if digests != {name: sha256_file(path) for name, path in sorted(second.items())}:
        raise IntegrationError("build_orchestrator.bundle_nondeterministic")
    return digests


def source_bindings(root: Path = ROOT) -> list[dict[str, str]]:
    bindings = []
    for relative in SOURCE_PATHS:
        path = root / relative
        if not path.is_file() or path.is_symlink():
            raise IntegrationError(f"build_orchestrator.source_unavailable:{relative}")
        bindings.append({"path": relative, "sha256": sha256_file(path)})
    return bindings


def exercise() -> dict[str, Any]:
    required = ("ar", "cargo", "cpio", "node", "npm", "openssl", "rpm2cpio", "tar")
    missing = [command for command in required if shutil.which(command) is None]
    if missing:
        raise IntegrationError(f"build_orchestrator.tool_unavailable:{','.join(missing)}")

    run(["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"])
    run(["cargo", "build", "--release", "-p", "agentmage-host", "--locked"])
    run(
        [
            "cargo",
            "build",
            "--release",
            "-p",
            "agentmage-platform-linux-inference",
            "--bins",
            "--locked",
        ]
    )
    run(["cargo", "build", "-p", "agentmage-xtask", "--locked"])

    host = ROOT / "target/release/agentmage-host"
    seed = hashlib.sha256(b"agentmage build orchestrator integration v1").digest()
    wrong_seed = hashlib.sha256(
        b"agentmage build orchestrator integration wrong key v1"
    ).digest()
    try:
        with tempfile.TemporaryDirectory(
            prefix="agentmage-build-orchestrator-integration-"
        ) as directory:
            temporary = Path(directory)
            first = build_with_orchestrator(temporary / "first")
            second = build_with_orchestrator(temporary / "second")
            artifact_digests = compare_bundles(first, second)

            public_key = temporary / "release.pub"
            wrong_key = temporary / "wrong.pub"
            signature = temporary / "manifest.sig"
            public_key.write_bytes(raw_public_key(seed))
            wrong_key.write_bytes(raw_public_key(wrong_seed))
            run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "-p",
                    "agentmage-xtask",
                    "--locked",
                    "--",
                    "sign-package-manifest",
                    "--manifest",
                    str(first["manifest"]),
                    "--public-key",
                    str(public_key),
                    "--signature",
                    str(signature),
                ],
                input_bytes=seed,
            )

            roots = {"deb": temporary / "deb-root", "rpm": temporary / "rpm-root"}
            for root in roots.values():
                root.mkdir()
            extract_deb(first["deb"], roots["deb"])
            extract_rpm(first["rpm"], roots["rpm"])
            for root in roots.values():
                installed_manifest = root / MANIFEST_PATH
                if installed_manifest.read_bytes() != first["manifest"].read_bytes():
                    raise IntegrationError("build_orchestrator.cross_format_manifest_drift")
                verification = [
                    str(host),
                    "--verify-package-root",
                    str(root),
                    "--signature",
                    str(signature),
                    "--public-key",
                    str(public_key),
                ]
                run(verification)
                run(verification[:-1] + [str(wrong_key)], success=False)
                run(
                    [str(host), "--verify-package-candidate-root", str(root)],
                    success=False,
                )

            changed_manifest = roots["deb"] / MANIFEST_PATH
            value = json.loads(changed_manifest.read_text(encoding="utf-8"))
            value["release_sequence"] = 2
            changed_manifest.write_bytes(canonical_json(value))
            run(
                [
                    str(host),
                    "--verify-package-root",
                    str(roots["deb"]),
                    "--signature",
                    str(signature),
                    "--public-key",
                    str(public_key),
                ],
                success=False,
            )

            return {
                "schema_version": 1,
                "record_type": "build-orchestrator-integration-report",
                "component_id": "build-orchestrator",
                "lifecycle_evidence": "integration",
                "status": "pass-local-integration",
                "environment": {
                    "architecture": platform.machine(),
                    "operating_system": platform.system().lower(),
                },
                "neighbors_exercised": [
                    "vscode-extension-build",
                    "linux-release-package-builder",
                    "package-manifest-signer",
                    "deb-payload-extractor",
                    "rpm-payload-extractor",
                    "agentmage-host-package-verifier",
                ],
                "source_bindings": source_bindings(),
                "artifact_set": list(ARTIFACT_KEYS),
                "artifact_sha256": artifact_digests,
                "checks": {
                    "orchestrator_built_two_release_bundles": "pass",
                    "bundle_outputs_byte_identical": "pass",
                    "orchestrator_signed_exact_manifest": "pass",
                    "deb_and_rpm_manifest_identity": "pass",
                    "host_verified_deb_and_rpm": "pass",
                    "wrong_trust_root_refused": "pass",
                    "candidate_release_confusion_refused": "pass",
                    "manifest_mutation_refused": "pass",
                },
                "retained_private_key_material": False,
                "network_authority": "none",
                "platform_support_claim": "none",
                "release_claim": "none",
                "product_integration_claim": "none",
            }
    finally:
        mutable_seed = bytearray(seed)
        mutable_wrong_seed = bytearray(wrong_seed)
        mutable_seed[:] = b"\0" * len(mutable_seed)
        mutable_wrong_seed[:] = b"\0" * len(mutable_wrong_seed)


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["report must be an object"]
    expected_keys = {
        "artifact_set",
        "artifact_sha256",
        "checks",
        "component_id",
        "environment",
        "lifecycle_evidence",
        "neighbors_exercised",
        "network_authority",
        "platform_support_claim",
        "product_integration_claim",
        "record_type",
        "release_claim",
        "retained_private_key_material",
        "schema_version",
        "source_bindings",
        "status",
    }
    if set(value) != expected_keys:
        failures.append("report fields are not closed")
        return failures
    header = {
        "schema_version": 1,
        "record_type": "build-orchestrator-integration-report",
        "component_id": "build-orchestrator",
        "lifecycle_evidence": "integration",
        "status": "pass-local-integration",
    }
    if any(value.get(key) != expected for key, expected in header.items()):
        failures.append("report identity or status changed")
    if value.get("artifact_set") != list(ARTIFACT_KEYS):
        failures.append("artifact set changed")
    digests = value.get("artifact_sha256")
    if not isinstance(digests, dict) or tuple(sorted(digests)) != ARTIFACT_KEYS or any(
        not isinstance(item, str)
        or len(item) != 64
        or any(character not in string.hexdigits.lower() for character in item)
        for item in digests.values()
    ):
        failures.append("artifact digests are invalid")
    expected_checks = {
        "orchestrator_built_two_release_bundles",
        "bundle_outputs_byte_identical",
        "orchestrator_signed_exact_manifest",
        "deb_and_rpm_manifest_identity",
        "host_verified_deb_and_rpm",
        "wrong_trust_root_refused",
        "candidate_release_confusion_refused",
        "manifest_mutation_refused",
    }
    checks = value.get("checks")
    if not isinstance(checks, dict) or set(checks) != expected_checks or any(
        result != "pass" for result in checks.values()
    ):
        failures.append("integration checks are incomplete")
    if value.get("neighbors_exercised") != [
        "vscode-extension-build",
        "linux-release-package-builder",
        "package-manifest-signer",
        "deb-payload-extractor",
        "rpm-payload-extractor",
        "agentmage-host-package-verifier",
    ]:
        failures.append("neighbor set changed")
    if value.get("retained_private_key_material") is not False:
        failures.append("private key retention was claimed")
    for key in (
        "network_authority",
        "platform_support_claim",
        "release_claim",
        "product_integration_claim",
    ):
        if value.get(key) != "none":
            failures.append(f"{key} exceeded the component integration boundary")
    bindings = value.get("source_bindings")
    if (
        not isinstance(bindings, list)
        or not all(
            isinstance(item, dict) and set(item) == {"path", "sha256"}
            for item in bindings
        )
        or [item["path"] for item in bindings] != list(SOURCE_PATHS)
    ):
        failures.append("source bindings changed")
    else:
        for item in bindings:
            path = root / item["path"]
            digest = item["sha256"]
            if (
                not isinstance(digest, str)
                or len(digest) != 64
                or any(character not in string.hexdigits.lower() for character in digest)
                or not path.is_file()
                or digest != sha256_file(path)
            ):
                failures.append(f"source binding is stale: {item['path']}")
    environment = value.get("environment")
    if not isinstance(environment, dict) or set(environment) != {
        "architecture",
        "operating_system",
    }:
        failures.append("environment identity is invalid")
    return failures


def load_report(path: Path = REPORT_PATH) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            report = exercise()
            failures = validate_report(report)
            if failures:
                raise IntegrationError("; ".join(failures))
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            temporary = REPORT_PATH.with_suffix(".json.tmp")
            temporary.write_bytes(canonical_json(report))
            os.replace(temporary, REPORT_PATH)
        failures = validate_report(load_report())
    except (OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        print(f"build orchestrator integration failed: {error}", file=os.sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"build orchestrator integration failed: {failure}", file=os.sys.stderr)
        return 1
    print("build orchestrator package-release integration validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
