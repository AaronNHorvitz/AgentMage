#!/usr/bin/env python3
"""Execute synthetic signed-manual-patch verification and recovery fixtures."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_ROOT = ROOT / "fixtures/support/manual-patch"
CASES_PATH = FIXTURE_ROOT / "cases.json"
REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.2/manual-patch-verification-report.json"
OPENSSL_PATH = Path("/usr/bin/openssl")
DOMAIN = b"AgentMage signed manual patch metadata v1\x00"
CURRENT_RELEASE = {
    "release_id": "agentmage-0.1.0",
    "semantic_version": "0.1.0",
    "release_sequence": 1,
    "configuration_sha256": "3333333333333333333333333333333333333333333333333333333333333333",
}
CURRENT_PLATFORM = {
    "os": "fedora",
    "architecture": "x86_64",
    "version": 44,
    "runtime_adapter": "llama-cpp-native",
}
EXPECTED_CASES = (
    ("valid", "verified-can-proceed"),
    ("wrong-signer", "blocked-wrong-signer"),
    ("downgrade", "blocked-downgrade"),
    ("corrupt", "blocked-artifact-corrupt"),
    ("mismatched", "blocked-platform-mismatch"),
    ("interrupted", "interrupted-prior-preserved"),
    ("revoked", "blocked-revoked-release"),
    ("unsupported-version", "blocked-unsupported-metadata-version"),
    ("migration-failure", "blocked-migration-failure"),
    ("rollback", "rollback-complete-prior-restored"),
)
ARTIFACT_IDS = (
    "agentmage-package",
    "release-manifest",
    "provenance",
    "sbom",
    "cryptographic-bom",
    "model-bom",
    "component-inventory",
    "configuration",
    "capability-delta",
    "authority-delta",
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def signature_payload(metadata: dict[str, Any]) -> bytes:
    payload = copy.deepcopy(metadata)
    for signature in payload["signing"]["detached_signatures"]:
        signature["signature_sha256"] = "0" * 64
    return DOMAIN + json.dumps(
        payload, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def safe_path(root: Path, relative: Any) -> Path | None:
    if not isinstance(relative, str) or not relative:
        return None
    parsed = PurePosixPath(relative)
    if parsed.is_absolute() or ".." in parsed.parts or str(parsed) != relative:
        return None
    candidate = root.joinpath(*parsed.parts)
    try:
        candidate.resolve().relative_to(root.resolve())
    except (OSError, ValueError):
        return None
    return candidate


def openssl_identity() -> dict[str, str]:
    version = subprocess.run(
        [str(OPENSSL_PATH), "version"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
        timeout=10,
    )
    if version.returncode != 0 or not OPENSSL_PATH.is_file():
        raise ValueError("OpenSSL verification executable is unavailable")
    return {
        "path": str(OPENSSL_PATH),
        "version": version.stdout.strip(),
        "executable_sha256": sha256_file(OPENSSL_PATH),
    }


def verify_signature(public_key: Path, signature: Path, payload: bytes) -> bool:
    with tempfile.TemporaryDirectory(prefix="agentmage-patch-signature-") as directory:
        payload_path = Path(directory) / "payload.bin"
        payload_path.write_bytes(payload)
        result = subprocess.run(
            [
                str(OPENSSL_PATH),
                "pkeyutl",
                "-verify",
                "-pubin",
                "-inkey",
                str(public_key),
                "-rawin",
                "-in",
                str(payload_path),
                "-sigfile",
                str(signature),
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=10,
        )
        return result.returncode == 0


def verify_signing(metadata: dict[str, Any], trust: dict[str, Path]) -> str | None:
    signing = metadata.get("signing", {})
    signatures = signing.get("detached_signatures", [])
    threshold = signing.get("threshold")
    if (
        signing.get("algorithm") != "ed25519"
        or signing.get("verify_before_extraction") is not True
        or not isinstance(threshold, int)
        or threshold < 1
    ):
        return "blocked-signature-policy"
    verified = set()
    payload = signature_payload(metadata)
    for signature_record in signatures:
        signer_id = signature_record.get("signer_key_id")
        public_key = trust.get(signer_id)
        if public_key is None:
            return "blocked-wrong-signer"
        signature_path = safe_path(FIXTURE_ROOT, signature_record.get("signature_path"))
        if signature_path is None or not signature_path.is_file():
            return "blocked-signature-missing"
        if sha256_file(signature_path) != signature_record.get("signature_sha256"):
            return "blocked-signature-corrupt"
        if verify_signature(public_key, signature_path, payload):
            verified.add(signer_id)
    return None if len(verified) >= threshold else "blocked-signature-invalid"


def verify_artifacts(metadata: dict[str, Any], corrupt_artifact_id: str | None) -> str | None:
    artifacts = metadata.get("artifacts", [])
    if tuple(item.get("artifact_id") for item in artifacts) != ARTIFACT_IDS:
        return "blocked-artifact-closure"
    for artifact in artifacts:
        path = safe_path(FIXTURE_ROOT / "artifacts", artifact.get("path"))
        if path is None or not path.is_file():
            return "blocked-artifact-missing"
        content = path.read_bytes()
        if artifact.get("artifact_id") == corrupt_artifact_id:
            content += b"synthetic-corruption"
        if len(content) != artifact.get("bytes") or sha256_bytes(content) != artifact.get(
            "sha256"
        ):
            return "blocked-artifact-corrupt"
    return None


def verify_patch(
    metadata: dict[str, Any],
    case: dict[str, Any],
    trust: dict[str, Path],
) -> str:
    if metadata.get("schema_version") != 1 or metadata.get("metadata_version") != 1:
        return "blocked-unsupported-metadata-version"
    signing_failure = verify_signing(metadata, trust)
    if signing_failure:
        return signing_failure
    release = metadata.get("release", {})
    current = metadata.get("current_release_precondition", {})
    if (
        current.get("release_id") != CURRENT_RELEASE["release_id"]
        or current.get("semantic_version") != CURRENT_RELEASE["semantic_version"]
        or current.get("release_sequence") != CURRENT_RELEASE["release_sequence"]
        or current.get("configuration_sha256")
        != CURRENT_RELEASE["configuration_sha256"]
        or release.get("release_sequence", 0) <= current.get("release_sequence", 0)
    ):
        return "blocked-downgrade"
    platform = metadata.get("platform", {})
    if (
        platform.get("os") != CURRENT_PLATFORM["os"]
        or platform.get("architecture") != CURRENT_PLATFORM["architecture"]
        or platform.get("runtime_adapter") != CURRENT_PLATFORM["runtime_adapter"]
        or int(platform.get("minimum_version", 10**9)) > CURRENT_PLATFORM["version"]
    ):
        return "blocked-platform-mismatch"
    revocation = metadata.get("revocation", {})
    if release.get("release_id") in revocation.get("revoked_release_ids", []):
        return "blocked-revoked-release"
    artifact_failure = verify_artifacts(metadata, case.get("corrupt_artifact_id"))
    if artifact_failure:
        return artifact_failure
    if case.get("migration_outcome") == "fail":
        return "blocked-migration-failure"
    return "verified-can-proceed"


def execute_transaction(metadata: dict[str, Any], mode: str) -> str:
    prior = FIXTURE_ROOT / "artifacts/prior/agentmage-0.1.0.tar.zst"
    target = FIXTURE_ROOT / "artifacts/packages/agentmage-0.1.1-fedora-x86_64.tar.zst"
    expected_prior = metadata["current_release_precondition"]["package_sha256"]
    with tempfile.TemporaryDirectory(prefix="agentmage-patch-transaction-") as directory:
        root = Path(directory)
        active = root / "active-package"
        stage = root / "staged-package"
        backup = root / "prior-package"
        shutil.copyfile(prior, active)
        if sha256_file(active) != expected_prior:
            return "blocked-prior-precondition"
        shutil.copyfile(target, stage)
        with stage.open("rb") as handle:
            os.fsync(handle.fileno())
        if mode == "interrupt-after-stage":
            return (
                "interrupted-prior-preserved"
                if sha256_file(active) == expected_prior
                else "fail-partial-active-state"
            )
        shutil.copyfile(active, backup)
        os.replace(stage, active)
        if mode == "postcheck-failure-rollback":
            rollback = root / "rollback-candidate"
            shutil.copyfile(backup, rollback)
            os.replace(rollback, active)
            return (
                "rollback-complete-prior-restored"
                if sha256_file(active) == expected_prior
                and sha256_file(backup) == expected_prior
                else "fail-rollback-state"
            )
    return "verified-can-proceed"


def run_case(case: dict[str, Any], trust: dict[str, Path]) -> dict[str, Any]:
    metadata_path = safe_path(FIXTURE_ROOT, case.get("metadata_path"))
    if metadata_path is None or not metadata_path.is_file():
        raise ValueError(f"manual patch case metadata is missing: {case.get('case_id')}")
    metadata = read_json(metadata_path)
    outcome = verify_patch(metadata, case, trust)
    if outcome == "verified-can-proceed" and case.get("transaction_mode"):
        outcome = execute_transaction(metadata, case["transaction_mode"])
    return {
        "case_id": case["case_id"],
        "expected_outcome": case["expected_outcome"],
        "actual_outcome": outcome,
        "status": "pass" if outcome == case["expected_outcome"] else "fail",
        "metadata_path": case["metadata_path"],
        "metadata_sha256": sha256_file(metadata_path),
        "raw_output_retained": False,
    }


def fixture_paths() -> tuple[str, ...]:
    return tuple(
        path.relative_to(ROOT).as_posix()
        for path in sorted(FIXTURE_ROOT.rglob("*"))
        if path.is_file()
    )


def build_report(root: Path = ROOT) -> dict[str, Any]:
    if root != ROOT:
        raise ValueError("manual patch verifier uses the repository fixture root")
    manifest = read_json(CASES_PATH)
    if tuple(
        (case.get("case_id"), case.get("expected_outcome"))
        for case in manifest.get("cases", [])
    ) != EXPECTED_CASES:
        raise ValueError("manual patch verification case closure is invalid")
    trust = {
        "agentmage-release-key-1-fixture": FIXTURE_ROOT
        / "trust/authorized-public.pem"
    }
    results = [run_case(case, trust) for case in manifest["cases"]]
    if any(result["status"] != "pass" for result in results):
        raise ValueError("manual patch verification fixture failed")
    sources = (
        "scripts/manual_patch_verifier.py",
        "tests/test_manual_patch_verifier.py",
        *fixture_paths(),
    )
    return {
        "schema_version": 1,
        "story_id": "3.2",
        "task_id": "3.2.2.1",
        "status": "pass-synthetic-verification-no-product-activation",
        "verification_engine": openssl_identity(),
        "cases": results,
        "summary": {
            "required_case_count": len(EXPECTED_CASES),
            "passed_case_count": len(results),
            "signature_verified_case_count": sum(
                result["actual_outcome"] not in {
                    "blocked-wrong-signer",
                    "blocked-signature-corrupt",
                    "blocked-signature-invalid",
                    "blocked-signature-missing",
                    "blocked-signature-policy",
                }
                and result["case_id"] != "unsupported-version"
                for result in results
            ),
            "prior_state_failure_count": 0,
            "network_authority_count": 0,
        },
        "source_artifacts": [
            {
                "path": relative,
                "sha256": sha256_file(root / relative),
                "bytes": (root / relative).stat().st_size,
            }
            for relative in sources
        ],
        "private_user_data_used": False,
        "network_used": False,
        "production_signer_claim": "none",
        "product_verifier_claim": "none",
        "product_activation_claim": "none",
        "release_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_evidence_substituted": False,
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["manual patch verification report must be an object"]
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        return [f"cannot rebuild manual patch verification report: {error}"]
    failures = []
    if value != expected:
        failures.append("manual patch verification report is stale or non-deterministic")
    if (
        value.get("summary", {}).get("required_case_count") != 10
        or value.get("summary", {}).get("passed_case_count") != 10
        or value.get("summary", {}).get("prior_state_failure_count") != 0
        or value.get("summary", {}).get("network_authority_count") != 0
        or value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("production_signer_claim") != "none"
        or value.get("product_verifier_claim") != "none"
        or value.get("product_activation_claim") != "none"
        or value.get("release_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_evidence_substituted") is not False
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("manual patch verification report made an unsupported claim")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read manual patch verification report: {error}"]
    return validate_report(report, root)


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-manual-patch-verification-", dir=path.parent
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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_atomic(REPORT_PATH, canonical_json(build_report()))
        failures = check_artifact()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Manual patch verification failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"Manual patch verification failed: {failure}")
        return 1
    print("Story 3.2 manual patch adversarial fixtures validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
