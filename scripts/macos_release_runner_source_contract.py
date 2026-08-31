#!/usr/bin/env python3
"""Validate and retain the source-only macOS isolated release-runner contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-release-runner-source-contract.json"
)
POLICY_PATH = "packaging/macos/release-runner-policy.contract-fixture.json"
RUNNER_PATH = "packaging/macos/release-runner.sh"
SOURCE_PATHS = (
    "packaging/macos/README.md",
    POLICY_PATH,
    RUNNER_PATH,
    "release/platform-manifests/macos/v1/contract-fixture.json",
    "scripts/macos_release_runner_source_contract.py",
    "tests/test_macos_release_runner_source_contract.py",
)
COMPONENTS = (
    "kernel_host",
    "vscode_bridge",
    "xpc_tool_helper",
    "metal_inference_service",
)
POLICY_KEYS = {
    "schema_version",
    "record_type",
    "status",
    "ceremony_id",
    "source_revision",
    "version",
    "expected_macos_build",
    "expected_xcode_build",
    "architecture",
    "team_id",
    "application_identity_sha1",
    "installer_identity_sha1",
    "bundle_identifiers",
    "app_group_identifier",
    "keychain_access_group",
    "entitlements",
    "previous_package_path",
    "previous_package_sha256",
    "notary_keychain_profile",
    "credential_values_present",
    "private_environment_values_present",
    "release_claim",
}
EXPECTED_ENTITLEMENTS = {
    "kernel_host": [
        "com.apple.security.app-sandbox",
        "com.apple.security.application-groups",
        "com.apple.security.files.bookmarks.app-scope",
        "com.apple.security.files.user-selected.read-only",
        "keychain-access-groups",
    ],
    "vscode_bridge": [
        "com.apple.security.app-sandbox",
        "com.apple.security.application-groups",
    ],
    "xpc_tool_helper": [
        "com.apple.security.app-sandbox",
        "com.apple.security.files.bookmarks.app-scope",
    ],
    "metal_inference_service": ["com.apple.security.app-sandbox"],
}
EXPECTED_FAILURES = {
    "usage",
    "absolute-path-required",
    "policy-unavailable",
    "evidence-destination-not-new",
    "source-root",
    "evidence-inside-source",
    "policy-field",
    "policy-not-release-approved",
    "policy-release-claim",
    "policy-field-closure",
    "policy-file-metadata",
    "policy-entitlement-closure",
    "credential-value-in-policy",
    "private-environment-in-policy",
    "notary-profile",
    "source-revision",
    "version",
    "team-id",
    "application-identity",
    "installer-identity",
    "identity-class-collision",
    "bundle-identifier",
    "bundle-identity-collision",
    "keychain-team-binding",
    "previous-package",
    "not-macos",
    "not-arm64",
    "root-forbidden",
    "elevated-identity-forbidden",
    "administrator-group-forbidden",
    "home-owner-mismatch",
    "macos-build",
    "xcode-build",
    "wrong-source-revision",
    "source-not-detached",
    "dirty-source",
    "release-project-unavailable",
    "release-scheme-unavailable",
    "previous-package-digest",
    "install-path",
    "preexisting-install",
    "evidence-create",
    "resolved-evidence-inside-source",
    "private-staging-path",
    "application-identity-query",
    "installer-identity-query",
    "application-identity-not-exact",
    "installer-identity-not-exact",
    "release-cargo-unavailable",
    "release-npm-unavailable",
    "archived-app-unavailable",
    "component-closure",
    "component-team",
    "component-identifier",
    "component-entitlements",
    "component-entitlement-closure",
    "hardened-runtime",
    "get-task-allow",
    "prohibited-entitlement",
    "sandbox-entitlement-closure",
    "app-group-entitlement-closure",
    "bookmark-entitlement-closure",
    "read-only-entitlement-closure",
    "keychain-entitlement-closure",
    "app-group-value",
    "keychain-group-value",
    "notary-submit",
    "notary-not-accepted",
    "notary-id",
    "notary-log-status",
    "notary-log-issues",
    "candidate-install-postcondition",
    "uninstall-path",
    "uninstall-postcondition",
    "uninstall-residue",
    "candidate-reinstall-postcondition",
    "rollback-remove-path",
    "rollback-install-postcondition",
    "credential-shaped-evidence",
}


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_policy(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict) or set(value) != POLICY_KEYS:
        return ["release policy field closure changed"]
    scalar_expectations = {
        "schema_version": 1,
        "record_type": "macos-release-runner-policy",
        "status": "contract-fixture",
        "architecture": "arm64",
        "team_id": "AAAAAAAAAA",
        "notary_keychain_profile": "agentmage-release-notary-v1",
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "none",
    }
    for key, expected in scalar_expectations.items():
        if value.get(key) != expected:
            failures.append(f"synthetic policy disposition changed: {key}")
    if set(value.get("bundle_identifiers", {})) != set(COMPONENTS):
        failures.append("release policy component closure changed")
    bundles = list(value.get("bundle_identifiers", {}).values())
    if len(set(bundles)) != len(COMPONENTS) or any(
        not isinstance(item, str)
        or not item.startswith("com.example.agentmage.contractfixture.")
        for item in bundles
    ):
        failures.append("synthetic bundle identities changed")
    if value.get("entitlements") != EXPECTED_ENTITLEMENTS:
        failures.append("release policy entitlement closure changed")
    if value.get("app_group_identifier") != "group.com.example.agentmage.contractfixture":
        failures.append("synthetic App Group changed")
    if value.get("keychain_access_group") != (
        "AAAAAAAAAA.com.example.agentmage.contractfixture"
    ):
        failures.append("synthetic Keychain group changed")
    for key in ("source_revision", "application_identity_sha1", "installer_identity_sha1"):
        if not re.fullmatch(r"[0-9a-f]{40}", str(value.get(key, ""))):
            failures.append(f"synthetic policy hash invalid: {key}")
    if not re.fullmatch(r"[0-9a-f]{64}", str(value.get("previous_package_sha256", ""))):
        failures.append("synthetic prior-package digest invalid")
    if value.get("previous_package_path") != "/nonexistent/agentmage-previous.pkg":
        failures.append("synthetic prior-package path changed")
    prohibited_keys = {
        "password",
        "private_key",
        "api_key",
        "apple_id",
        "issuer_id",
        "auth_token",
        "environment",
    }
    if prohibited_keys.intersection(value):
        failures.append("release policy admits credential or environment material")
    return failures


def validate_sources(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    for relative in SOURCE_PATHS:
        if not (root / relative).is_file():
            failures.append(f"missing source input: {relative}")
    if failures:
        return failures
    try:
        policy = json.loads((root / POLICY_PATH).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"release policy is not valid JSON: {error}"]
    failures.extend(validate_policy(policy))

    runner = (root / RUNNER_PATH).read_text(encoding="utf-8")
    readme = (root / "packaging/macos/README.md").read_text(encoding="utf-8")
    observed_failures = set(re.findall(r'fail "([a-z0-9-]+)"', runner))
    if observed_failures != EXPECTED_FAILURES:
        failures.append("release runner refusal taxonomy changed")
    required_terms = (
        "set -euo pipefail",
        "umask 077",
        '[[ $# -eq 2 ]]',
        '[[ "$POLICY_STATUS" = "release-approved" ]]',
        '[[ "$RELEASE_CLAIM" = "authorized-candidate" ]]',
        '[[ "$NOTARY_PROFILE" = "$FIXED_NOTARY_PROFILE" ]]',
        '[[ "$(/usr/bin/uname -s)" = "Darwin" ]]',
        '[[ "$(/usr/bin/uname -m)" = "arm64"',
        '[[ "$INVOKING_EFFECTIVE_UID" != "0" ]]',
        '[[ "$INVOKING_REAL_UID" = "$INVOKING_EFFECTIVE_UID" ]]',
        "/usr/bin/grep -Fx admin",
        "home-owner-mismatch",
        "git symbolic-ref -q HEAD",
        "git status --porcelain=v1 --untracked-files=all",
        'env -i HOME="$HOME"',
        'run_step "rust-build" "$FIXED_CARGO" build --workspace --all-targets --release --locked --target aarch64-apple-darwin',
        'run_step "typescript-build" "$FIXED_NPM" run build --workspace @agentmage/vscode-shell',
        "swift test --package-path platforms/macos",
        "-scheme \"$FIXED_XCODE_SCHEME\"",
        "CODE_SIGN_STYLE=Manual",
        "CODE_SIGN_IDENTITY=\"$APPLICATION_IDENTITY\"",
        'OTHER_CODE_SIGN_FLAGS="--timestamp --options runtime"',
        "codesign --verify --deep --strict --verbose=4",
        "codesign -d --verbose=4 --requirements :-",
        "codesign -d --entitlements :-",
        "com.apple.security.get-task-allow",
        "pkgbuild --root",
        "productsign --sign \"$INSTALLER_IDENTITY\"",
        "pkgutil --check-signature",
        "notarytool submit",
        '--keychain-profile "$FIXED_NOTARY_PROFILE"',
        "--wait --output-format json",
        "notarytool log",
        '"severity"[[:space:]]*:[[:space:]]*"(warning|error)"',
        "stapler staple",
        "stapler validate",
        "spctl --assess --type install",
        'run_step "gatekeeper-install" /usr/sbin/spctl --assess --type install --verbose=4 "$SIGNED_PACKAGE"',
        "installer -pkg \"$SIGNED_PACKAGE\" -target CurrentUserHomeDirectory",
        "--release-smoke-test --content-free-output",
        '"$HOME/Applications/AgentMage.app") /bin/rm -rf -- "$INSTALLED_APP"',
        "installer -pkg \"$PREVIOUS_PACKAGE\" -target CurrentUserHomeDirectory",
        "credential-shaped-evidence",
        '"release_claim":"signed-package-candidate"',
        '"record_type":"macos-standard-user-acceptance"',
        '"identity_class":"non-admin-standard-user"',
        '"phase_order":["build","sign","notarize","staple","gatekeeper-check","install","launch","use","remove"]',
    )
    for term in required_terms:
        if term not in runner:
            failures.append(f"release runner missing required term: {term}")
    ordered_terms = (
        'run_step "rust-build"',
        'run_step "xcode-archive"',
        'run_step "codesign-deep-verify"',
        'run_step "pkgbuild"',
        'run_step "productsign"',
        "notarytool submit",
        'run_step "staple"',
        'run_step "gatekeeper-install"',
        'run_step "candidate-install"',
        'run_step "candidate-launch"',
        'run_step "candidate-reinstall"',
        'run_step "rollback-install"',
        'run_step "rollback-launch"',
    )
    positions = [runner.find(term) for term in ordered_terms]
    if -1 in positions or positions != sorted(positions):
        failures.append("release lifecycle order changed")
    for prohibited in (
        "sudo ",
        " altool ",
        "--apple-id",
        "--password",
        "--api-key",
        "--api-issuer",
        "curl ",
        "wget ",
        "scp ",
        "gh release",
        "actions/upload-artifact",
        "spctl --master-disable",
        "security add-generic-password",
    ):
        if prohibited in runner:
            failures.append(f"release runner contains prohibited surface: {prohibited}")
    if runner.count("notarytool submit") != 1:
        failures.append("release runner notarization submission count changed")
    if runner.count("installer -pkg") != 3:
        failures.append("release runner install/rollback count changed")
    if runner.count("/bin/rm -rf -- \"$INSTALLED_APP\"") != 2:
        failures.append("release runner uninstall/rollback removal count changed")

    readme_terms = (
        "isolated Apple Silicon runner",
        "always rejected by live execution",
        "fixed notarization Keychain profile",
        "enabled for the bounded notarization phase",
        "disabled again before installed-product launch",
        "No step uses `sudo`",
        "has not run",
        "no release\nXcode app/project",
        "`BLOCKED-MACOS`",
    )
    for term in readme_terms:
        if term not in readme:
            failures.append(f"macOS release runbook missing statement: {term}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed macOS release-runner source changed: {relative}")
    return revision, tree


def build_report(
    root: Path = ROOT, *, source_revision: str = "HEAD"
) -> dict[str, Any]:
    failures = validate_sources(root)
    if failures:
        raise ValueError("; ".join(failures))
    revision, tree = resolve_revision(source_revision, root)
    return {
        "schema_version": 1,
        "record_type": "macos-release-runner-source-contract",
        "task_id": "8.1.1.7",
        "status": "prepared-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "refusal_count": len(EXPECTED_FAILURES),
            "policy_field_count": len(POLICY_KEYS),
            "component_count": len(COMPONENTS),
            "notarization_submissions": 1,
            "notary_profile_selector": "agentmage-release-notary-v1",
            "install_domain": "CurrentUserHomeDirectory",
            "installed_application": "$HOME/Applications/AgentMage.app",
            "root_execution_allowed": False,
            "administrator_group_allowed": False,
            "elevated_identity_allowed": False,
            "home_owner_match_required": True,
            "acceptance_phase_count": 9,
            "arbitrary_commands_allowed": False,
            "raw_credentials_accepted": False,
            "automatic_trigger_defined": False,
            "artifact_upload_defined": False,
            "prior_package_required": True,
            "prior_package_digest_required": True,
            "prior_valid_state_restored": True,
        },
        "execution": {
            "apple_silicon_runner_attested": False,
            "release_build_performed": False,
            "developer_id_signing_performed": False,
            "notarization_performed": False,
            "notary_log_reviewed": False,
            "stapling_performed": False,
            "gatekeeper_assessment_performed": False,
            "standard_user_install_performed": False,
            "packaged_launch_performed": False,
            "uninstall_performed": False,
            "rollback_performed": False,
            "residue_scan_performed": False,
        },
        "claims": {
            "task_complete": False,
            "signed_package_exists": False,
            "notarized_package_exists": False,
            "release_candidate_exists": False,
            "macos_support": False,
            "physical_m5_qualified": False,
        },
        "remaining_blockers": [
            "The fixed AgentMageRelease Xcode app project, scheme, and embedded signed targets do not exist.",
            "No release-approved production policy or release-derived identities exist.",
            "No Developer ID Application or Installer identity is available in this environment.",
            "No fixed notarization Keychain profile or Apple notarization credential is available.",
            "No previously shipped signed, notarized, stapled package is available for rollback.",
            "No disposable isolated physical Apple Silicon release runner has executed the ceremony.",
            "No native build, signature, entitlement, notarization, stapling, Gatekeeper, install, launch, uninstall, rollback, or residue evidence exists.",
            "No independent critical-boundary review or physical MacBook Pro M5 qualification has been retained.",
        ],
    }


def canonical_bytes(report: dict[str, Any]) -> bytes:
    return (json.dumps(report, indent=2, sort_keys=True) + "\n").encode()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            revision = arguments.source_revision
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained report has no source revision")
        expected = canonical_bytes(build_report(source_revision=revision))
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS release-runner source contract failed: {error}")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_bytes(expected)
    elif REPORT_PATH.read_bytes() != expected:
        print("macOS release-runner source report is missing or stale")
        return 1
    print("macOS isolated release-runner source contract passed without release promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
