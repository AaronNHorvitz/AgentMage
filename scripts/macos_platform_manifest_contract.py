#!/usr/bin/env python3
"""Enforce the frozen macOS release manifest field set for schema version 3.

The macOS platform is unimplemented: no v3 manifest JSON, signature, verifier
acceptance, or adapter is committed. This contract guards the *fields* that a
future signed v3 manifest must carry so that later work cannot silently expand,
rename, reorder, or drop them. The authoritative freeze lives in
``release/platform-manifests/v2/macos-fields.md`` and is cross-referenced by
``release/platform-manifests/v3/README.md``. Any drift in the frozen tokens,
canonical preimage prefixes, closure counts, or the ``v3`` reservation fails
this contract before implementation can land.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
FREEZE_DOC: Final = ROOT / "release/platform-manifests/v2/macos-fields.md"
V3_DIR: Final = ROOT / "release/platform-manifests/v3"
V3_README: Final = V3_DIR / "README.md"

DOMAIN_SEPARATOR_V3: Final = r"agentmage.platform-release-manifest.v3\0"
DOMAIN_SEPARATOR_V2: Final = r"agentmage.platform-release-manifest.v2\0"

REQUIRED_SECTIONS: Final = (
    "## Trust Envelope",
    "## Shared Runtime Identity",
    "## macOS Build Boundary",
    "## macOS Toolchain Identity",
    "## macOS Visual Studio Code Build Identity",
    "## macOS Package Identity",
    "## macOS Signing and Sandbox Fields",
    "## Per-Component Entitlement Canonicalization",
    "## Per-Component Designated-Requirement Canonicalization",
    "## Field-level Freeze Rules",
    "## Independence From Linux Evidence",
    "## Implementation Scope",
)

REQUIRED_SHARED_FIELDS: Final = (
    "`schema_version`",
    "`record_type`",
    "`status`",
    "`adapter_api_version`",
    "`platform_family`",
    "`architecture`",
    "`os_build_min_sha256`",
    "`os_build_tested_sha256`",
    "`os_build_supported`",
    "`toolchain_sha256`",
    "`vscode_build_sha256`",
    "`distribution_artifact_sha256`",
    "`installed_closure_sha256`",
    "`additional_signed_inventory_sha256`",
    "`capabilities`",
)

REQUIRED_SIGNING_FIELDS: Final = (
    "`team_id`",
    "`bundle_ids.host`",
    "`bundle_ids.bridge`",
    "`bundle_ids.xpc_helper`",
    "`bundle_ids.inference`",
    "`bundle_ids.model_installer`",
    "`bundle_ids.vscode_extension`",
    "`app_group_id`",
    "`entitlements_map_sha256`",
    "`entitlements.host`",
    "`entitlements.bridge`",
    "`entitlements.xpc_helper`",
    "`entitlements.inference`",
    "`entitlements.model_installer`",
    "`entitlements.vscode_extension`",
    "`designated_requirements.host`",
    "`designated_requirements.bridge`",
    "`designated_requirements.xpc_helper`",
    "`designated_requirements.inference`",
    "`designated_requirements.model_installer`",
    "`designated_requirements.vscode_extension`",
    "`helper_hashes.host`",
    "`helper_hashes.bridge`",
    "`helper_hashes.xpc_helper`",
    "`helper_hashes.inference`",
    "`helper_hashes.model_installer`",
    "`helper_hashes.vscode_extension`",
    "`bundle_layout`",
    "`additional_signed_inventory`",
    "`installed_root_token`",
)

CANONICAL_PREIMAGE_PREFIXES: Final = (
    "agentmage.macos-build.v3",
    "agentmage.macos-toolchain.v3",
    "agentmage.macos-vscode.v3",
    "agentmage.macos-installed-closure.v3",
    "agentmage.macos-additional-signed-inventory.v3",
    "agentmage.macos-entitlements.v3",
    "agentmage.macos-entitlement.v3",
    "agentmage.macos-designated-requirement.v3",
)

FROZEN_CAPABILITY_COUNT: Final = "exactly ten"
FROZEN_RUNTIME_CLOSURE_COUNT: Final = "exactly four"
FROZEN_ADDITIONAL_INVENTORY_COUNT: Final = "exactly two"
FROZEN_ENTITLEMENT_COMPONENT_COUNT: Final = "exactly six"

FROZEN_FAILURE_CODES: Final = (
    "ManifestSignatureInvalid",
    "ManifestUnsupported",
    "ManifestMalformed",
    "OsBuildMismatch",
    "PackageMismatch",
)

FROZEN_PLATFORM_FAMILY: Final = "macos-apple-silicon"
FROZEN_ARCHITECTURE: Final = "aarch64"
FROZEN_INSTALLED_ROOT_TOKEN: Final = "applications-agentmage-bundle-v3"
FROZEN_TEAM_ID_LENGTH_TOKEN: Final = "exact 10-character Apple Developer Team Identifier"

FORBIDDEN_TOKENS: Final = (
    "Rosetta and x86_64 are not\n  accepted",
)

V3_README_REQUIRED_TOKENS: Final = (
    "Schema version 3",
    "macos-apple-silicon",
    "ManifestUnsupported",
    DOMAIN_SEPARATOR_V3,
    DOMAIN_SEPARATOR_V2,
    "../v2/macos-fields.md",
    "scripts/macos_platform_manifest_contract.py",
    "tests/test_macos_platform_manifest_contract.py",
)

V3_FORBIDDEN_SUFFIXES: Final = (".json", ".sig", ".pem", ".p8", ".key")


class MacosManifestContractError(ValueError):
    """Raised when the frozen macOS manifest field contract has drifted."""


def _missing(text: str, tokens: tuple[str, ...]) -> list[str]:
    return [token for token in tokens if token not in text]


def validate_freeze_document(text: str) -> list[str]:
    failures: list[str] = []
    for missing in _missing(text, REQUIRED_SECTIONS):
        failures.append(f"macOS freeze document lost section heading: {missing}")
    for missing in _missing(text, REQUIRED_SHARED_FIELDS):
        failures.append(f"macOS freeze document lost shared runtime field: {missing}")
    for missing in _missing(text, REQUIRED_SIGNING_FIELDS):
        failures.append(f"macOS freeze document lost signing/sandbox field: {missing}")
    for missing in _missing(text, CANONICAL_PREIMAGE_PREFIXES):
        failures.append(f"macOS freeze document lost canonical preimage prefix: {missing}")
    for missing in _missing(text, FROZEN_FAILURE_CODES):
        failures.append(f"macOS freeze document lost failure code: {missing}")
    if DOMAIN_SEPARATOR_V3 not in text:
        failures.append("macOS freeze document lost the v3 domain separator")
    if DOMAIN_SEPARATOR_V2 not in text:
        failures.append(
            "macOS freeze document lost the v2 domain separator distinction"
        )
    if FROZEN_PLATFORM_FAMILY not in text:
        failures.append("macOS freeze document lost the macos-apple-silicon binding")
    if FROZEN_ARCHITECTURE not in text:
        failures.append("macOS freeze document lost the aarch64 architecture binding")
    if FROZEN_INSTALLED_ROOT_TOKEN not in text:
        failures.append(
            "macOS freeze document lost the installed_root_token schema value"
        )
    if FROZEN_TEAM_ID_LENGTH_TOKEN not in text:
        failures.append("macOS freeze document lost the Team ID length constraint")
    if FROZEN_CAPABILITY_COUNT not in text:
        failures.append("macOS freeze document lost the ten-capability closure count")
    if FROZEN_RUNTIME_CLOSURE_COUNT not in text:
        failures.append(
            "macOS freeze document lost the four-member runtime closure count"
        )
    if FROZEN_ADDITIONAL_INVENTORY_COUNT not in text:
        failures.append(
            "macOS freeze document lost the two-member additional signed inventory count"
        )
    if FROZEN_ENTITLEMENT_COMPONENT_COUNT not in text:
        failures.append(
            "macOS freeze document lost the six-component entitlement-map count"
        )
    for forbidden in FORBIDDEN_TOKENS:
        if forbidden not in text:
            failures.append(
                "macOS freeze document lost the Rosetta/x86_64 rejection clause"
            )
    return failures


def validate_v3_reservation(readme_text: str, entries: tuple[Path, ...]) -> list[str]:
    failures: list[str] = []
    for missing in _missing(readme_text, V3_README_REQUIRED_TOKENS):
        failures.append(f"v3 README lost required token: {missing}")
    committed_names = {entry.name for entry in entries if entry.is_file()}
    if "README.md" not in committed_names:
        failures.append("v3 directory is missing its README.md reservation record")
    for entry in entries:
        suffix = entry.suffix.lower()
        if suffix in V3_FORBIDDEN_SUFFIXES:
            failures.append(
                f"v3 directory contains a forbidden {suffix} artifact: {entry.name}"
            )
    return failures


def load_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def enumerate_v3_entries() -> tuple[Path, ...]:
    if not V3_DIR.is_dir():
        return ()
    return tuple(sorted(V3_DIR.iterdir()))


def run_contract() -> list[str]:
    failures: list[str] = []
    if not FREEZE_DOC.is_file():
        failures.append(
            "macOS freeze document is missing at release/platform-manifests/v2/macos-fields.md"
        )
        return failures
    if not V3_DIR.is_dir():
        failures.append(
            "v3 reservation directory is missing at release/platform-manifests/v3"
        )
        return failures
    freeze_text = load_text(FREEZE_DOC)
    failures.extend(validate_freeze_document(freeze_text))
    if not V3_README.is_file():
        failures.append("v3 reservation README is missing at release/platform-manifests/v3/README.md")
    else:
        readme_text = load_text(V3_README)
        failures.extend(validate_v3_reservation(readme_text, enumerate_v3_entries()))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    try:
        failures = run_contract()
    except OSError as error:
        print(f"macOS manifest contract failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(
        "macOS release manifest v3 field freeze holds: "
        f"{len(REQUIRED_SHARED_FIELDS)} runtime fields, "
        f"{len(REQUIRED_SIGNING_FIELDS)} signing/sandbox fields, "
        f"{len(CANONICAL_PREIMAGE_PREFIXES)} canonical preimages."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
