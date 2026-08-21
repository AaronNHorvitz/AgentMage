#!/usr/bin/env python3
"""Enforce the frozen macOS release manifest field set for schema version 3.

The macOS platform is unimplemented: no v3 manifest JSON, signature, verifier
acceptance, or adapter is committed. This contract guards the *fields* that a
future signed v3 manifest must carry so that later work cannot silently expand,
rename, reorder, or drop them. The authoritative freeze lives in
``release/platform-manifests/v2/macos-fields.md`` and is cross-referenced by
``release/platform-manifests/v3/README.md``. Any drift in the frozen tokens,
canonical preimage byte strings, closure counts, frozen array orderings,
authoritative source rules, or the ``v3`` reservation fails this contract
before implementation can land.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Final, Iterable


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

# --- Closed manifest field set ------------------------------------------------
# The complete, closed set of manifest field identifiers a signed v3 record
# must carry. Any declaration bullet inside the field-declaration sections that
# names an identifier outside this set is an unauthorized field addition. Any
# member of this set that is not declared by a bullet is a lost field.
FROZEN_MANIFEST_FIELDS: Final = frozenset(
    {
        # Shared runtime identity
        "schema_version",
        "record_type",
        "status",
        "adapter_api_version",
        "platform_family",
        "architecture",
        "os_build_min_sha256",
        "os_build_tested_sha256",
        "os_build_supported",
        "toolchain_sha256",
        "vscode_build_sha256",
        "distribution_artifact_sha256",
        "installed_closure_sha256",
        "additional_signed_inventory_sha256",
        "capabilities",
        # Package identity
        "bundle_layout",
        "additional_signed_inventory",
        "installed_root_token",
        # Signing / sandbox
        "team_id",
        "bundle_ids.host",
        "bundle_ids.bridge",
        "bundle_ids.xpc_helper",
        "bundle_ids.inference",
        "bundle_ids.model_installer",
        "bundle_ids.vscode_extension",
        "app_group_id",
        "entitlements_map_sha256",
        "entitlements.host",
        "entitlements.bridge",
        "entitlements.xpc_helper",
        "entitlements.inference",
        "entitlements.model_installer",
        "entitlements.vscode_extension",
        "designated_requirements.host",
        "designated_requirements.bridge",
        "designated_requirements.xpc_helper",
        "designated_requirements.inference",
        "designated_requirements.model_installer",
        "designated_requirements.vscode_extension",
        "helper_hashes.host",
        "helper_hashes.bridge",
        "helper_hashes.xpc_helper",
        "helper_hashes.inference",
        "helper_hashes.model_installer",
        "helper_hashes.vscode_extension",
    }
)

# Identifiers that appear as declaration bullets somewhere in the freeze
# document but name separately typed adapter observations rather than manifest
# fields. They are excluded from the closed manifest-field set instead of being
# scoped to a section list so that a future author cannot smuggle a new
# manifest field into a non-manifest section and evade the closed-set check.
ADAPTER_OBSERVATION_ALLOWLIST: Final = frozenset(
    {
        "observed_os_build_sha256",
    }
)

# Per-field canonical preimage definitions. Each definition is validated
# inside the specific bullet that declares its owning manifest field so that a
# grammar shared with a sibling field (for example the installed-closure and
# additional-signed-inventory member lines both spelling
# ``component=<name> path=<installed-relative-path> sha256=<lowercase-hex>``)
# cannot silently drift on one side while the sibling occurrence keeps a
# global substring check satisfied.
CANONICAL_PREIMAGE_DEFINITIONS: Final = (
    {
        "field": "installed_closure_sha256",
        "section": "## macOS Package Identity",
        "domain_prefix": r"agentmage.macos-installed-closure.v3\n",
        "member_grammar": (
            r"component=<name> path=<installed-relative-path> "
            r"sha256=<lowercase-hex>\n"
        ),
        "count_phrase": "exactly four lines",
        "ordering": (
            "`host`, `bridge`, `xpc_helper`, `inference` in that exact order"
        ),
    },
    {
        "field": "additional_signed_inventory_sha256",
        "section": "## macOS Package Identity",
        "domain_prefix": r"agentmage.macos-additional-signed-inventory.v3\n",
        "member_grammar": (
            r"component=<name> path=<installed-relative-path> "
            r"sha256=<lowercase-hex>\n"
        ),
        "count_phrase": "exactly two lines",
        "ordering": (
            "`model_installer`, `vscode_extension` in that exact order"
        ),
    },
    {
        "field": "entitlements_map_sha256",
        "section": "## macOS Signing and Sandbox Fields",
        "domain_prefix": r"agentmage.macos-entitlements.v3\n",
        "member_grammar": r"component=<name> sha256=<lowercase-hex>\n",
        "count_phrase": "exactly six lines",
        "ordering": (
            "`host`, `bridge`, `xpc_helper`, `inference`, "
            "`model_installer`, `vscode_extension` in that exact order"
        ),
    },
)

# --- Exact canonical preimage byte strings ------------------------------------
# Each entry is the exact UTF-8 byte string that a canonical preimage MUST be
# built from, spelled with literal ``\n`` sequences the way the freeze document
# spells them. Renaming any key, reordering the keys, or dropping/adding a key
# perturbs the exact string and refuses this contract.
CANONICAL_PREIMAGE_STRINGS: Final = (
    r"agentmage.macos-build.v3\nproduct-build-version=<ProductBuildVersion>\n",
    (
        r"agentmage.macos-toolchain.v3\n"
        r"xcode-command-line-tools-build=<XcodeCLTBuild>\n"
        r"macos-sdk-version=<MacOSSDKVersion>\n"
        r"swift-marketing-version=<SwiftMarketingVersion>\n"
        r"swiftlang-build=<SwiftlangBuild>\n"
    ),
    r"agentmage.macos-vscode.v3\nmarketing-version=<MarketingVersion>\ncommit=<Commit>\n",
    (
        r"agentmage.macos-entitlement.v3\ncomponent=<name>\n"
        r"blob-sha256=<entitlement_blob_sha256_component>\n"
        r"der-sha256=<entitlement_der_sha256_component>\n"
    ),
    (
        r"agentmage.macos-designated-requirement.v3\ncomponent=<name>\n"
        r"csreq-sha256=<designated_requirement_bin_sha256_component>\n"
    ),
    r"agentmage.macos-installed-closure.v3\n",
    r"agentmage.macos-additional-signed-inventory.v3\n",
    r"agentmage.macos-entitlements.v3\n",
    r"component=<name> path=<installed-relative-path> sha256=<lowercase-hex>\n",
    r"component=<name> sha256=<lowercase-hex>\n",
)

# --- Frozen array / component orderings --------------------------------------
# These orderings are covered by the signed manifest. Reordering any one of
# them changes signature-covered bytes and refuses this contract.
FROZEN_ARRAY_ORDERINGS: Final = (
    # Four-member runtime closure order (installed_closure_sha256 preimage).
    "`host`, `bridge`, `xpc_helper`, `inference` in that exact order",
    # Two-member additional signed inventory order.
    "`model_installer`, `vscode_extension` in that exact order",
    # Six-component entitlement-map order (spans two markdown lines).
    (
        "`host`, `bridge`, `xpc_helper`, `inference`,\n"
        "  `model_installer`, `vscode_extension` in that exact order"
    ),
    # Bundle-layout keys (Package Identity).
    "four keys `host`, `bridge`,\n  `xpc_helper`, and `inference`",
    # Additional signed inventory keys.
    "two keys\n  `model_installer` and `vscode_extension`",
)

# --- Authoritative source rules ----------------------------------------------
# Each entry is a substring that pins where a canonical value MUST be sourced
# from. Weakening any one of these (for example dropping ``product.json`` or
# ``sw_vers -buildVersion``) refuses this contract.
AUTHORITATIVE_SOURCE_RULES: Final = (
    # macOS build boundary source.
    "`sw_vers -buildVersion`",
    # Xcode Command Line Tools SDK ProductBuildVersion source.
    "plutil -extract ProductBuildVersion raw -o -",
    "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/System/Library/CoreServices/SystemVersion.plist",
    # macOS SDK Version source.
    "plutil -extract Version raw -o -",
    "SDKSettings.plist",
    # Swift toolchain sources.
    "`swift --version`",
    "swiftlang",
    # Visual Studio Code build sources.
    "Contents/Resources/app/product.json",
    "the `version` key",
    "the `commit` key",
    # Entitlement extraction sources.
    "CSMAGIC_EMBEDDED_ENTITLEMENTS",
    "0xFADE7171",
    "CSMAGIC_EMBEDDED_ENTITLEMENTS_DER",
    "0xFADE7172",
    "--entitlements :-",
    # Designated-requirement extraction sources.
    "kSecCodeMagicRequirement",
    "0xFADE0C00",
    "codesign --display --requirements -",
    "csreq -b -r",
)


class MacosManifestContractError(ValueError):
    """Raised when the frozen macOS manifest field contract has drifted."""


_FIELD_IDENT_RE = re.compile(r"`([a-z_][a-z0-9_.]*)`")


def _missing(text: str, tokens: Iterable[str]) -> list[str]:
    return [token for token in tokens if token not in text]


def _extract_section(text: str, heading: str) -> str:
    """Return the body of the given ``##`` section (up to the next ``##``)."""
    lines = text.splitlines()
    start = None
    for index, line in enumerate(lines):
        if line.startswith(heading):
            start = index + 1
            break
    if start is None:
        return ""
    end = len(lines)
    for index in range(start, len(lines)):
        if lines[index].startswith("## "):
            end = index
            break
    return "\n".join(lines[start:end])


def _bullets(section_body: str) -> list[str]:
    """Split a section body into logical top-level bullet strings."""
    bullets: list[str] = []
    current: list[str] | None = None
    for line in section_body.splitlines():
        if line.startswith("- "):
            if current is not None:
                bullets.append(" ".join(current))
            current = [line[2:]]
        elif current is not None and line.startswith("  "):
            current.append(line.strip())
        else:
            if current is not None:
                bullets.append(" ".join(current))
                current = None
    if current is not None:
        bullets.append(" ".join(current))
    return bullets


def _all_section_headings(text: str) -> list[str]:
    """Return every ``## `` heading line in document order."""
    return [line for line in text.splitlines() if line.startswith("## ")]


def extract_declared_fields(text: str) -> set[str]:
    """Return every manifest-field identifier declared by a top-level bullet
    anywhere in the freeze document, sourced only from the bullet head
    (before the first ``:``).

    Every ``##`` section is inspected so that a manifest-field declaration
    cannot be smuggled outside a previously allowlisted section. Identifiers
    that name separately typed adapter observations (per
    ``ADAPTER_OBSERVATION_ALLOWLIST``) are excluded because they are not
    manifest fields. A bullet without a ``:`` is prose that references
    identifiers rather than declaring them; it is skipped so that removing a
    real declaration bullet while leaving an incidental backticked mention
    still fails the contract.
    """
    declared: set[str] = set()
    for heading in _all_section_headings(text):
        for bullet in _bullets(_extract_section(text, heading)):
            if ":" not in bullet:
                continue
            head = bullet.split(":", 1)[0]
            for ident in _FIELD_IDENT_RE.findall(head):
                if ident in ADAPTER_OBSERVATION_ALLOWLIST:
                    continue
                declared.add(ident)
    return declared


def _find_field_bullet(section_body: str, field: str) -> str | None:
    """Return the top-level bullet in ``section_body`` whose head declares
    ``field``, or ``None`` if no such bullet exists."""
    prefix = f"`{field}`:"
    for bullet in _bullets(section_body):
        if bullet.startswith(prefix):
            return bullet
    return None


def validate_canonical_preimage_definition(
    text: str, spec: "dict[str, str]"
) -> list[str]:
    """Validate the complete canonical preimage definition for one field
    inside the specific bullet that declares it. Binds the domain prefix,
    member-line grammar, count phrase, and component ordering to the owning
    manifest field instead of accepting them as free-floating global
    substrings that a sibling section could satisfy on its behalf."""
    failures: list[str] = []
    field = spec["field"]
    section_body = _extract_section(text, spec["section"])
    bullet = _find_field_bullet(section_body, field)
    if bullet is None:
        failures.append(
            "macOS freeze document is missing the canonical preimage "
            f"declaration bullet for {field} inside {spec['section']}"
        )
        return failures
    for label, token in (
        ("domain prefix", spec["domain_prefix"]),
        ("member-line grammar", spec["member_grammar"]),
        ("count phrase", spec["count_phrase"]),
        ("component ordering", spec["ordering"]),
    ):
        if token not in bullet:
            failures.append(
                f"macOS freeze document {field} canonical preimage lost its "
                f"{label}: {token!r}"
            )
    return failures


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

    # Closed manifest field set: every declared identifier must be a frozen
    # field, and every frozen field must be declared by a bullet.
    declared = extract_declared_fields(text)
    for extra in sorted(declared - FROZEN_MANIFEST_FIELDS):
        failures.append(
            f"macOS freeze document declares unauthorized manifest field: {extra}"
        )
    for missing in sorted(FROZEN_MANIFEST_FIELDS - declared):
        failures.append(
            f"macOS freeze document is missing declaration of frozen field: {missing}"
        )

    # Exact canonical preimage byte strings.
    for preimage in CANONICAL_PREIMAGE_STRINGS:
        if preimage not in text:
            failures.append(
                "macOS freeze document lost the exact canonical preimage string: "
                f"{preimage}"
            )

    # Per-field canonical preimage definitions, scoped to their owning
    # section bullet so a shared grammar cannot silently drift on one side.
    for spec in CANONICAL_PREIMAGE_DEFINITIONS:
        failures.extend(validate_canonical_preimage_definition(text, spec))

    # Frozen array / component orderings.
    for ordering in FROZEN_ARRAY_ORDERINGS:
        if ordering not in text:
            failures.append(
                "macOS freeze document lost a frozen component ordering: "
                f"{ordering!r}"
            )

    # Authoritative source rules for canonical values.
    for rule in AUTHORITATIVE_SOURCE_RULES:
        if rule not in text:
            failures.append(
                f"macOS freeze document weakened an authoritative source rule: {rule}"
            )

    return failures


def _walk(entry: Path) -> list[Path]:
    """Expand a single entry into the set of files to inspect. Directories are
    walked recursively; non-directory entries are returned as-is so callers can
    still probe synthetic paths."""
    if entry.is_dir():
        return sorted(path for path in entry.rglob("*") if not path.is_dir())
    return [entry]


def _display(path: Path) -> str:
    try:
        return path.relative_to(V3_DIR).as_posix()
    except ValueError:
        return path.as_posix()


def validate_v3_reservation(readme_text: str, entries: tuple[Path, ...]) -> list[str]:
    failures: list[str] = []
    for missing in _missing(readme_text, V3_README_REQUIRED_TOKENS):
        failures.append(f"v3 README lost required token: {missing}")
    files: list[Path] = []
    for entry in entries:
        files.extend(_walk(entry))
    committed_names = {entry.name for entry in files if entry.is_file()}
    if "README.md" not in committed_names:
        failures.append("v3 directory is missing its README.md reservation record")
    for entry in files:
        suffix = entry.suffix.lower()
        if suffix in V3_FORBIDDEN_SUFFIXES:
            failures.append(
                f"v3 directory contains a forbidden {suffix} artifact: {_display(entry)}"
            )
    return failures


def load_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def enumerate_v3_entries() -> tuple[Path, ...]:
    """Return every path (files and directories) under the v3 reservation
    directory, walked recursively so nested artifacts cannot hide beneath a
    committed subdirectory."""
    if not V3_DIR.is_dir():
        return ()
    return tuple(sorted(V3_DIR.rglob("*")))


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
        f"{len(FROZEN_MANIFEST_FIELDS)} closed manifest fields, "
        f"{len(CANONICAL_PREIMAGE_STRINGS)} canonical preimage strings, "
        f"{len(CANONICAL_PREIMAGE_DEFINITIONS)} section-scoped preimage "
        "definitions, "
        f"{len(FROZEN_ARRAY_ORDERINGS)} frozen orderings, "
        f"{len(AUTHORITATIVE_SOURCE_RULES)} authoritative source rules."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
