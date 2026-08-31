#!/usr/bin/env python3
"""Assemble and verify content-free macOS native release reports."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import plistlib
import re
import stat
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.macos_release_runner_source_contract import (  # noqa: E402
    COMPONENTS,
    EXPECTED_ENTITLEMENTS,
    POLICY_KEYS,
)
from scripts.macos_signed_reference_package import (  # noqa: E402
    MANIFEST_KEYS,
    TERMINAL_KEYS,
    canonical_bytes,
    contains_prohibited_key,
    read_closed_json,
    sha256_file,
    valid_sha256,
    validate_reference_bundle,
    validate_release_policy,
    validate_terminal,
    write_atomic,
)

REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-release-reports-source-contract.json"
)
SOURCE_PATHS = (
    "packaging/macos/RELEASE-REPORTS.md",
    "packaging/macos/release-runner.sh",
    "scripts/macos_release_reports.py",
    "scripts/macos_signed_reference_package.py",
    "tests/test_macos_release_reports.py",
)
REPORT_KEYS = {
    "schema_version",
    "record_type",
    "status",
    "source_revision",
    "version",
    "macos_build",
    "xcode_build",
    "architecture",
    "team_id",
    "package_sha256",
    "entitlements",
    "code_signing",
    "notarization",
    "stapling",
    "gatekeeper",
    "credential_values_present",
    "private_environment_values_present",
    "release_claim",
}
ENTITLEMENT_REPORT_KEYS = {
    "bundle_identifier",
    "values",
    "observed_plist_sha256",
}
CODE_SIGNING_KEYS = {
    "application_identity_sha1",
    "installer_identity_sha1",
    "deep_verify_passed",
    "package_signature_passed",
    "deep_verify_report_sha256",
    "package_signature_report_sha256",
    "components",
}
COMPONENT_SIGNING_KEYS = {
    "bundle_identifier",
    "team_id",
    "designated_requirement",
    "hardened_runtime",
    "component_sha256",
    "identity_report_sha256",
}
NOTARIZATION_KEYS = {
    "submission_id",
    "status",
    "issues",
    "submission_report_sha256",
    "log_report_sha256",
}
STAPLING_KEYS = {"status", "package_sha256", "staple_report_sha256", "validation_report_sha256"}
GATEKEEPER_KEYS = {
    "assessment_type",
    "status",
    "source",
    "team_id",
    "report_sha256",
}
UUID = re.compile(r"[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}")


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def safe_regular(path: Path, maximum: int = 16 * 1024 * 1024) -> list[str]:
    try:
        metadata = path.lstat()
    except OSError:
        return [f"report input unavailable: {path.name}"]
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_nlink != 1
        or metadata.st_uid != os.geteuid()
        or metadata.st_mode & 0o022
        or metadata.st_size > maximum
    ):
        return [f"unsafe report input: {path.name}"]
    return []


def safe_evidence_root(path: Path) -> list[str]:
    try:
        metadata = path.lstat()
    except OSError:
        return ["release evidence directory unavailable"]
    if (
        not path.is_absolute()
        or not stat.S_ISDIR(metadata.st_mode)
        or metadata.st_uid != os.geteuid()
        or metadata.st_mode & 0o077
    ):
        return ["release evidence directory is not owner-only"]
    return []


def one_log(log_root: Path, pattern: str) -> tuple[Path | None, list[str]]:
    matches = sorted(log_root.glob(pattern))
    if len(matches) != 1:
        return None, [f"expected exactly one raw report: {pattern}"]
    failures = safe_regular(matches[0])
    return (None, failures) if failures else (matches[0], [])


def entitlement_values(path: Path) -> tuple[list[str] | None, list[str]]:
    try:
        value = plistlib.loads(path.read_bytes())
    except (OSError, plistlib.InvalidFileException, ValueError) as error:
        return None, [f"invalid entitlement plist {path.name}: {error}"]
    if not isinstance(value, dict):
        return None, [f"invalid entitlement plist object: {path.name}"]
    observed = sorted(str(key) for key, enabled in value.items() if enabled is not False)
    if len(observed) != len(value):
        return None, [f"disabled entitlement entry is not reportable: {path.name}"]
    return observed, []


def load_json_file(path: Path, maximum: int = 4 * 1024 * 1024) -> tuple[Any, list[str]]:
    failures = safe_regular(path, maximum)
    if failures:
        return None, failures
    try:
        return json.loads(path.read_text(encoding="utf-8")), []
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        return None, [f"invalid report JSON {path.name}: {error}"]


def assemble_reports(
    policy_path: Path,
    manifest_path: Path,
    package_path: Path,
    terminal_path: Path,
    evidence_root: Path,
) -> tuple[list[str], dict[str, Any] | None]:
    failures = safe_evidence_root(evidence_root)
    policy, found = read_closed_json(policy_path, POLICY_KEYS, 1024 * 1024)
    failures.extend(found)
    manifest, found = read_closed_json(manifest_path, MANIFEST_KEYS, 4 * 1024 * 1024)
    failures.extend(found)
    terminal, found = read_closed_json(terminal_path, TERMINAL_KEYS, 1024 * 1024)
    failures.extend(found)
    failures.extend(safe_regular(package_path, 16 * 1024 * 1024 * 1024))
    if failures or not isinstance(policy, dict) or not isinstance(manifest, dict) or not isinstance(terminal, dict):
        return failures, None
    reference_failures, _reference = validate_reference_bundle(
        policy_path, manifest_path, package_path, terminal_path
    )
    failures.extend(reference_failures)
    package_sha256 = sha256_file(package_path)
    log_root = evidence_root / "command-logs"
    failures.extend(safe_evidence_root(log_root))
    if failures:
        return failures, None

    fixed_logs: dict[str, Path] = {}
    for name, pattern in {
        "deep": "*-codesign-deep-verify.log",
        "package": "*-package-signature.log",
        "staple": "*-staple.log",
        "staple_validate": "*-staple-validate.log",
        "gatekeeper": "*-gatekeeper-install.log",
    }.items():
        path, found = one_log(log_root, pattern)
        failures.extend(found)
        if path is not None:
            fixed_logs[name] = path

    entitlements: dict[str, Any] = {}
    signing_components: dict[str, Any] = {}
    bundles = policy.get("bundle_identifiers", {})
    for index, component in enumerate(COMPONENTS, start=1):
        identity_path, found = one_log(log_root, f"*-component-{index}-identity.log")
        failures.extend(found)
        plist_path, found = one_log(log_root, f"*-component-{index}-entitlements.plist")
        failures.extend(found)
        if identity_path is None or plist_path is None:
            continue
        values, found = entitlement_values(plist_path)
        failures.extend(found)
        identity_text = identity_path.read_text(encoding="utf-8", errors="replace")
        bundle = bundles.get(component)
        if (
            values != sorted(EXPECTED_ENTITLEMENTS[component])
            or f"TeamIdentifier={policy.get('team_id')}" not in identity_text
            or f"Identifier={bundle}" not in identity_text
            or "runtime" not in identity_text
        ):
            failures.append(f"raw native identity mismatch: {component}")
            continue
        entitlements[component] = {
            "bundle_identifier": bundle,
            "values": EXPECTED_ENTITLEMENTS[component],
            "observed_plist_sha256": sha256_file(plist_path),
        }
        signing_components[component] = {
            "bundle_identifier": bundle,
            "team_id": policy["team_id"],
            "designated_requirement": (
                f"anchor apple generic and identifier {bundle} and "
                f"certificate leaf[subject.OU] = {policy['team_id']}"
            ),
            "hardened_runtime": True,
            "component_sha256": manifest["component_hashes"][component],
            "identity_report_sha256": sha256_file(identity_path),
        }

    submit_path = evidence_root / "notary-submit.json"
    notary_log_path = evidence_root / "notary-log.json"
    submit, found = load_json_file(submit_path)
    failures.extend(found)
    notary_log, found = load_json_file(notary_log_path)
    failures.extend(found)
    if not isinstance(submit, dict) or not isinstance(notary_log, dict):
        failures.append("notary reports are not objects")
    else:
        issues = notary_log.get("issues", [])
        if (
            submit.get("status") != "Accepted"
            or not UUID.fullmatch(str(submit.get("id", "")))
            or notary_log.get("status") != "Accepted"
            or issues != []
        ):
            failures.append("notary reports are not an issue-free acceptance")
    if failures:
        return failures, None

    report = {
        "schema_version": 1,
        "record_type": "macos-native-release-reports",
        "status": "ceremony-passed",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "macos_build": policy["expected_macos_build"],
        "xcode_build": policy["expected_xcode_build"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": package_sha256,
        "entitlements": entitlements,
        "code_signing": {
            "application_identity_sha1": policy["application_identity_sha1"],
            "installer_identity_sha1": policy["installer_identity_sha1"],
            "deep_verify_passed": True,
            "package_signature_passed": True,
            "deep_verify_report_sha256": sha256_file(fixed_logs["deep"]),
            "package_signature_report_sha256": sha256_file(fixed_logs["package"]),
            "components": signing_components,
        },
        "notarization": {
            "submission_id": submit["id"],
            "status": "Accepted",
            "issues": [],
            "submission_report_sha256": sha256_file(submit_path),
            "log_report_sha256": sha256_file(notary_log_path),
        },
        "stapling": {
            "status": "validated",
            "package_sha256": package_sha256,
            "staple_report_sha256": sha256_file(fixed_logs["staple"]),
            "validation_report_sha256": sha256_file(fixed_logs["staple_validate"]),
        },
        "gatekeeper": {
            "assessment_type": "install",
            "status": "accepted",
            "source": "Notarized Developer ID",
            "team_id": policy["team_id"],
            "report_sha256": sha256_file(fixed_logs["gatekeeper"]),
        },
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
    }
    return [], report


def validate_reports(
    policy_path: Path,
    manifest_path: Path,
    package_path: Path,
    terminal_path: Path,
    reports_path: Path,
) -> tuple[list[str], dict[str, Any] | None]:
    failures, reference = validate_reference_bundle(
        policy_path, manifest_path, package_path, terminal_path
    )
    reports, found = read_closed_json(reports_path, REPORT_KEYS, 8 * 1024 * 1024)
    failures.extend(found)
    policy, found = read_closed_json(policy_path, POLICY_KEYS, 1024 * 1024)
    failures.extend(found)
    manifest, found = read_closed_json(manifest_path, MANIFEST_KEYS, 4 * 1024 * 1024)
    failures.extend(found)
    terminal, found = read_closed_json(terminal_path, TERMINAL_KEYS, 1024 * 1024)
    failures.extend(found)
    if failures or not all(isinstance(item, dict) for item in (reference, reports, policy, manifest, terminal)):
        return failures, None
    assert isinstance(reference, dict) and isinstance(reports, dict)
    assert isinstance(policy, dict) and isinstance(manifest, dict) and isinstance(terminal, dict)
    expected_scalars = {
        "schema_version": 1,
        "record_type": "macos-native-release-reports",
        "status": "ceremony-passed",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "macos_build": policy["expected_macos_build"],
        "xcode_build": policy["expected_xcode_build"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": reference["package_sha256"],
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
    }
    for key, value in expected_scalars.items():
        if reports.get(key) != value:
            failures.append(f"native release report mismatch: {key}")
    if contains_prohibited_key(reports):
        failures.append("native release reports contain credential or environment material")

    entitlements = reports.get("entitlements", {})
    if not isinstance(entitlements, dict) or set(entitlements) != set(COMPONENTS):
        failures.append("entitlement report component closure changed")
    else:
        for component in COMPONENTS:
            item = entitlements.get(component, {})
            if (
                not isinstance(item, dict)
                or set(item) != ENTITLEMENT_REPORT_KEYS
                or item.get("bundle_identifier") != policy["bundle_identifiers"][component]
                or item.get("values") != EXPECTED_ENTITLEMENTS[component]
                or not valid_sha256(item.get("observed_plist_sha256"))
            ):
                failures.append(f"entitlement report mismatch: {component}")

    signing = reports.get("code_signing", {})
    if not isinstance(signing, dict) or set(signing) != CODE_SIGNING_KEYS:
        failures.append("code-signing report closure changed")
    else:
        for key in ("application_identity_sha1", "installer_identity_sha1"):
            if signing.get(key) != policy[key]:
                failures.append(f"code-signing certificate mismatch: {key}")
        if signing.get("deep_verify_passed") is not True or signing.get("package_signature_passed") is not True:
            failures.append("code-signing verification did not pass")
        for key in ("deep_verify_report_sha256", "package_signature_report_sha256"):
            if not valid_sha256(signing.get(key)):
                failures.append(f"code-signing raw report hash invalid: {key}")
        components = signing.get("components", {})
        requirements = manifest["code_identity"]["designated_requirements"]
        if not isinstance(components, dict) or set(components) != set(COMPONENTS):
            failures.append("code-signing component closure changed")
        else:
            for component in COMPONENTS:
                item = components.get(component, {})
                if (
                    not isinstance(item, dict)
                    or set(item) != COMPONENT_SIGNING_KEYS
                    or item.get("bundle_identifier") != policy["bundle_identifiers"][component]
                    or item.get("team_id") != policy["team_id"]
                    or item.get("designated_requirement") != requirements[component]
                    or item.get("hardened_runtime") is not True
                    or item.get("component_sha256") != manifest["component_hashes"][component]
                    or not valid_sha256(item.get("identity_report_sha256"))
                ):
                    failures.append(f"code-signing component mismatch: {component}")

    notarization = reports.get("notarization", {})
    if (
        not isinstance(notarization, dict)
        or set(notarization) != NOTARIZATION_KEYS
        or not UUID.fullmatch(str(notarization.get("submission_id", "")))
        or notarization.get("status") != "Accepted"
        or notarization.get("issues") != []
        or not valid_sha256(notarization.get("submission_report_sha256"))
        or not valid_sha256(notarization.get("log_report_sha256"))
    ):
        failures.append("notarization report is not an issue-free acceptance")
    stapling = reports.get("stapling", {})
    if (
        not isinstance(stapling, dict)
        or set(stapling) != STAPLING_KEYS
        or stapling.get("status") != "validated"
        or stapling.get("package_sha256") != reference["package_sha256"]
        or not valid_sha256(stapling.get("staple_report_sha256"))
        or not valid_sha256(stapling.get("validation_report_sha256"))
    ):
        failures.append("stapling report is invalid")
    gatekeeper = reports.get("gatekeeper", {})
    if (
        not isinstance(gatekeeper, dict)
        or set(gatekeeper) != GATEKEEPER_KEYS
        or gatekeeper.get("assessment_type") != "install"
        or gatekeeper.get("status") != "accepted"
        or gatekeeper.get("source") != "Notarized Developer ID"
        or gatekeeper.get("team_id") != policy["team_id"]
        or not valid_sha256(gatekeeper.get("report_sha256"))
    ):
        failures.append("Gatekeeper report is invalid")
    if failures:
        return failures, None
    raw_hashes = {
        "entitlements": {name: entitlements[name]["observed_plist_sha256"] for name in COMPONENTS},
        "code_signing": {
            "deep_verify": signing["deep_verify_report_sha256"],
            "package_signature": signing["package_signature_report_sha256"],
            "components": {name: signing["components"][name]["identity_report_sha256"] for name in COMPONENTS},
        },
        "notarization": {
            "submission": notarization["submission_report_sha256"],
            "log": notarization["log_report_sha256"],
        },
        "stapling": {
            "staple": stapling["staple_report_sha256"],
            "validation": stapling["validation_report_sha256"],
        },
        "gatekeeper": gatekeeper["report_sha256"],
    }
    result = {
        "schema_version": 1,
        "record_type": "macos-native-release-reports-evidence",
        "status": "accepted-candidate-evidence",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": reference["package_sha256"],
        "reports_sha256": sha256_file(reports_path),
        "raw_report_hashes": raw_hashes,
        "native_tools_executed_by_ingestor": False,
        "native_outputs_reconciled": True,
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
        "macos_support_claim": False,
    }
    return [], result


def validate_sources(root: Path = ROOT) -> list[str]:
    failures = [
        f"missing source input: {relative}"
        for relative in SOURCE_PATHS
        if not (root / relative).is_file()
    ]
    if failures:
        return failures
    source = (root / "scripts/macos_release_reports.py").read_text(encoding="utf-8")
    tests = (root / "tests/test_macos_release_reports.py").read_text(encoding="utf-8")
    documentation = (root / "packaging/macos/RELEASE-REPORTS.md").read_text(encoding="utf-8")
    for term in (
        "safe_evidence_root(evidence_root)",
        'one_log(log_root, f"*-component-{index}-identity.log")',
        "entitlement_values(plist_path)",
        "validate_reference_bundle(",
        '"source_revision": policy["source_revision"]',
        'item.get("component_sha256") != manifest["component_hashes"][component]',
        'notarization.get("issues") != []',
        'gatekeeper.get("source") != "Notarized Developer ID"',
        '"native_tools_executed_by_ingestor": False',
        '"macos_support_claim": False',
    ):
        if term not in source:
            failures.append(f"native release-report verifier missing term: {term}")
    for prohibited in (
        'subprocess.run(["code' + 'sign"',
        "notary" + "tool submit",
        "stapler " + "staple",
        "spctl " + "--assess",
        "cu" + "rl ",
        "reque" + "sts.",
    ):
        if prohibited in source:
            failures.append(f"native release-report verifier contains prohibited effect: {prohibited}")
    if "Seven" not in tests or len(re.findall(r"^    def test_", tests, re.MULTILINE)) != 7:
        failures.append("native release-report mutation corpus is not seven closed tests")
    for term in (
        "five reviewable report families",
        "neither\ninvokes `codesign`",
        "makes no macOS support",
        "synthetic source evidence only",
        "`BLOCKED-MACOS`",
    ):
        if term not in documentation:
            failures.append(f"native release-report documentation missing statement: {term}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed native release-report source changed: {relative}")
    return revision, tree


def build_source_report(root: Path = ROOT, *, source_revision: str = "HEAD") -> dict[str, Any]:
    failures = validate_sources(root)
    if failures:
        raise ValueError("; ".join(failures))
    revision, tree = resolve_revision(source_revision, root)
    return {
        "schema_version": 1,
        "record_type": "macos-native-release-reports-source-contract",
        "task_id": "8.1.2.2",
        "status": "prepared-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "report_family_count": 5,
            "component_count": len(COMPONENTS),
            "report_field_count": len(REPORT_KEYS),
            "native_tool_execution_authority": False,
            "network_authority": False,
            "credential_authority": False,
            "package_copy_authority": False,
            "support_promotion_authority": False,
        },
        "execution": {
            "native_reports_assembled": False,
            "entitlements_observed": False,
            "code_signatures_observed": False,
            "notarization_observed": False,
            "stapling_observed": False,
            "gatekeeper_observed": False,
            "independent_review_performed": False,
        },
        "claims": {
            "task_complete": False,
            "native_release_reports_exist": False,
            "release_candidate_exists": False,
            "macos_support": False,
        },
        "remaining_blockers": [
            "No external entitlement report exists for the four signed components.",
            "No external application, component, or installer code-signing report exists.",
            "No external accepted notarization submission and issue-free log exist.",
            "No external stapling and validation reports exist for the exact package.",
            "No external install-type Gatekeeper acceptance report exists for the exact package.",
            "No release-approved policy, signed package, release-derived manifest, or successful terminal record exists.",
            "No independent reviewer has reconciled and retained the native report bundle.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-source-contract", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--assemble", action="store_true")
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--package", type=Path)
    parser.add_argument("--terminal", type=Path)
    parser.add_argument("--evidence-root", type=Path)
    parser.add_argument("--reports-output", type=Path)
    parser.add_argument("--reports", type=Path)
    parser.add_argument("--review-output", type=Path)
    arguments = parser.parse_args()
    try:
        if arguments.assemble:
            required = (arguments.policy, arguments.manifest, arguments.package, arguments.terminal, arguments.evidence_root, arguments.reports_output)
            if arguments.verify or arguments.write_source_contract or any(item is None for item in required):
                raise ValueError("report assembly requires exactly its six path inputs")
            failures, report = assemble_reports(*required[:-1])  # type: ignore[arg-type]
            if failures or report is None:
                raise ValueError("; ".join(failures))
            write_atomic(arguments.reports_output, report)  # type: ignore[arg-type]
            print("macOS native release reports assembled without native effects")
            return 0
        if arguments.verify:
            required = (arguments.policy, arguments.manifest, arguments.package, arguments.terminal, arguments.reports, arguments.review_output)
            if arguments.write_source_contract or any(item is None for item in required):
                raise ValueError("report verification requires exactly its six path inputs")
            failures, result = validate_reports(*required[:-1])  # type: ignore[arg-type]
            if failures or result is None:
                raise ValueError("; ".join(failures))
            write_atomic(arguments.review_output, result)  # type: ignore[arg-type]
            print("macOS native release reports accepted without support promotion")
            return 0
        if any(
            item is not None
            for item in (
                arguments.policy, arguments.manifest, arguments.package,
                arguments.terminal, arguments.evidence_root, arguments.reports_output,
                arguments.reports, arguments.review_output,
            )
        ):
            raise ValueError("report path inputs require --assemble or --verify")
        if arguments.write_source_contract:
            report = build_source_report(source_revision=arguments.source_revision)
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_bytes(canonical_bytes(report))
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained source contract has no source revision")
            expected = canonical_bytes(build_source_report(source_revision=revision))
            if REPORT_PATH.read_bytes() != expected:
                raise ValueError("native release-report source contract is stale")
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS native release-report verification failed: {error}")
        return 1
    print("macOS native release-report source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
