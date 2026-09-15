# macOS Platform Release Manifest Field Freeze

## Status and Scope

This document freezes the macOS release-manifest field surface required by
sub-task 7.1.1.2 and Sprint 8. It records the exact structural fields that any
future Apple Silicon release manifest must present, together with a public
synthetic fixture used only for contract validation.

The field freeze is not a signed release manifest, not a Developer ID
attestation, and not evidence that any macOS package, helper binary, or
entitlement is currently produced. The synthetic fixture uses distinctive
non-secret placeholder digests and an obvious placeholder Team ID; production
values may only enter through the later reviewed release pipeline.

Linux evidence cannot satisfy this field freeze, and this frozen surface cannot
satisfy any Linux, Windows, or GA gate. macOS execution, signing, notarization,
Gatekeeper acceptance, sandbox verification, and clean-install evidence remain
owned by Sprint 8 and the retained MacBook Pro M5 tasks.

## Frozen Fixture Location

`release/platform-manifests/v2/field-freeze/macos-aarch64.json` records the sole
current fixture. The `field-freeze/` subdirectory is deliberately distinct from
any future signed-release directory so a static tree scanner can never confuse a
frozen field specification with a released artifact.

## Required Top-Level Fields

Every future macOS release manifest and the current field freeze must present
exactly these top-level keys with the declared types and no additional keys:

| Field | Type | Notes |
| --- | --- | --- |
| `schema_version` | integer `2` | Matches the version-2 wire schema family. |
| `record_type` | string `platform-release-manifest-field-freeze` | Distinct from `platform-release-manifest`; a signed release must switch the value and add the signed-release fields. |
| `manifest_id` | string | Stable content-free identity for the field freeze. |
| `status` | string `frozen-field-specification` | Never `signed-release` in this file. |
| `adapter_api_version` | integer | Current platform adapter API version. |
| `platform_family` | string `macos-apple-silicon` | The only frozen family. |
| `architecture` | string `aarch64` | The only frozen architecture. |
| `platform_build` | object `{id, sha256}` | Exact operating-system build identity. |
| `product_toolchain` | object `{id, sha256}` | Pinned Rust/aarch64 Apple Darwin toolchain. |
| `vscode` | object `{version, commit, sha256}` | Supported Visual Studio Code build. |
| `team_id` | string, ten uppercase alphanumerics | Apple Developer Team identifier. |
| `app_group` | string | Fully qualified `<TEAMID>.<reverse-dns>` App Group. |
| `bundle_ids` | object with the four helper keys below | One bundle identifier per signed executable. |
| `designated_requirements` | object with the same four keys | One Apple designated requirement string per helper. |
| `entitlements` | object with the same four keys | Sorted, deduplicated entitlement identifiers per helper. |
| `helper_hashes` | object with the same four keys | Exact SHA-256 of each helper Mach-O. |
| `package` | object `{format, sha256, identity_class}` | `pkg` installer digest and identity class. |
| `credential_values_present` | boolean `false` | Manifest never carries credentials. |
| `private_environment_values_present` | boolean `false` | Manifest never carries environment secrets. |
| `linux_evidence_substituted` | boolean `false` | Linux evidence cannot cross into this manifest. |
| `release_claim` | string `none` | The field freeze never claims a release. |

## Helper Set

The frozen manifest identifies exactly four signed macOS components. Every
helper key is present in `bundle_ids`, `designated_requirements`,
`entitlements`, and `helper_hashes`:

- `kernel_host` — the arm64 AgentMage kernel host under Hardened Runtime and
  minimal App Sandbox entitlements.
- `vscode_bridge` — the signed native Visual Studio Code IPC bridge and
  mode-restricted App Group socket.
- `xpc_tool_helper` — the signed stateless XPC tool helper with one consumed
  grant, one bookmark, isolated scratch, and no network entitlement.
- `metal_inference` — the isolated Metal inference service with no workspace,
  tool, grant, or credential authority.

## Field Rules

- Every SHA-256 field is a lowercase 64-character hexadecimal digest.
- `team_id` is exactly ten uppercase ASCII alphanumeric characters.
- `app_group` must begin with `<team_id>.` and continue with a reverse-DNS
  identifier.
- Every bundle identifier is a reverse-DNS string using only lowercase ASCII
  letters, digits, hyphens, and dots.
- Every designated requirement string references the matching bundle identifier
  and Team ID.
- Every entitlement list is non-empty, sorted, and deduplicated.
- The `package.format` value is `pkg`; the `package.identity_class` value is
  `synthetic-field-freeze-fixture` for the current fixture and must change to a
  released-package identity class in any future signed manifest.

## Prohibitions

The manifest must never contain a hostname, username, home directory,
credential, token, private key, password, API key, or any file path that
identifies a private workstation. It must not carry `release_claim` values other
than `none` while the field freeze is in effect. Signed-release fields
(`signature`, `signer_identity`, `signed_at`, notarization ticket bytes, and
similar values) must never appear in this file; they belong to the later
signed-release surface.
