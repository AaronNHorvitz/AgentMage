# Frozen macOS Release Manifest Fields

This document freezes the exact set of fields a future macOS release manifest
must carry under version 2 of the signed release contract. Freezing the fields
before implementation prevents scope drift, ambient inputs, or platform
substitution. macOS remains unimplemented: no macOS manifest JSON, signature,
verifier acceptance, or activation path exists in this repository, and the
verifier continues to refuse `macos-apple-silicon` as `ManifestUnsupported`
until the full macOS increment lands.

## Trust Envelope

The macOS manifest is one bounded UTF-8 JSON object with `deny_unknown_fields`,
signed by a detached Ed25519 signature that validates over the exact frozen
domain separator and manifest bytes:

```text
agentmage.platform-release-manifest.v2\0 || exact_manifest_bytes
```

The expected verifying key enters startup independently from any macOS adapter
observation. Signatures, signers, schema, status, runtime, capability order,
and macOS-specific fields cannot be selected, echoed, or substituted by an
adapter.

## Shared Runtime Identity (frozen from the Linux baseline)

- `schema_version`: integer `2`.
- `record_type`: `platform-release-manifest`.
- `status`: `signed-release`.
- `adapter_api_version`: the current adapter API integer.
- `platform_family`: `macos-apple-silicon`.
- `architecture`: `aarch64` (Apple Silicon only; Rosetta and x86_64 are not
  accepted).
- `os_build_sha256`: SHA-256 of the normalized macOS build identity covering
  both the minimum supported and the tested `ProductBuildVersion` values, so a
  build outside the frozen supported range refuses activation before workspace
  access.
- `toolchain_sha256`: SHA-256 of the normalized Apple SDK plus Swift toolchain
  identity (Xcode command-line tools build, macOS SDK version, and Swift
  toolchain build), pinned to one exact combination per release.
- `vscode_build_sha256`: SHA-256 of the supported Visual Studio Code build
  identity (marketing version plus commit) that the macOS shell targets.
- `package_sha256`: SHA-256 of the exact installed AgentMage package artifact
  (notarized, stapled `.pkg` or `.dmg`) at rest on disk.
- `capabilities`: exactly ten ordered records in the frozen closure order, each
  with the canonical capability name and a nonzero SHA-256 mechanism digest.

## macOS Signing and Sandbox Fields (added by this freeze)

Every field below is required, is a fixed schema string when it names an
identifier, and is a lowercase hex SHA-256 digest when it names a hash. None
carries free-form text, environment values, credentials, notarization tickets,
provisioning-profile bodies, entitlements plists, or code-signing private keys.

- `team_id`: exact 10-character Apple Developer Team Identifier used by every
  signed component in the release. One release binds to one Team ID; a
  substituted Team ID refuses activation.
- `bundle_ids.host`: reverse-DNS bundle identifier of the sandboxed kernel
  host application.
- `bundle_ids.bridge`: reverse-DNS bundle identifier of the Visual Studio Code
  bridge helper installed alongside the extension.
- `bundle_ids.xpc_helper`: reverse-DNS bundle identifier of the XPC helper
  service used for privileged mediation.
- `bundle_ids.inference`: reverse-DNS bundle identifier of the local inference
  runtime component.
- `app_group_id`: exact `group.*` App Group identifier that owns the mode
  `0600` Unix socket container shared across the host, bridge, XPC helper, and
  inference components.
- `entitlements_sha256`: SHA-256 over the canonicalized, signed entitlement
  set applied to every component. Covers Hardened Runtime, minimal App Sandbox,
  the frozen App Group membership, and the exact list of allowed and denied
  entitlements. A single added, removed, or altered entitlement changes this
  digest.
- `designated_requirements.host`: SHA-256 over the canonical designated
  code-signing requirement (`csreq`) expression that peers use to validate the
  host's audit token.
- `designated_requirements.bridge`: SHA-256 over the canonical designated
  requirement for the Visual Studio Code bridge helper.
- `designated_requirements.xpc_helper`: SHA-256 over the canonical designated
  requirement for the XPC helper service.
- `designated_requirements.inference`: SHA-256 over the canonical designated
  requirement for the inference runtime component.
- `helper_hashes.host`: SHA-256 of the signed host executable file at rest.
- `helper_hashes.bridge`: SHA-256 of the signed Visual Studio Code bridge
  helper executable at rest.
- `helper_hashes.xpc_helper`: SHA-256 of the signed XPC helper executable at
  rest.
- `helper_hashes.inference`: SHA-256 of the signed inference-runtime binary at
  rest.

## Field-level Freeze Rules

- The union above is closed. Adding, removing, renaming, or reordering a field
  is a manifest-schema change that requires a new signed schema version, not a
  v2 amendment.
- Every SHA-256 field is a 64-character lowercase hex string over 32 nonzero
  bytes. An all-zero digest is rejected as `ManifestUnsupported`.
- Bundle identifiers, the App Group identifier, and the Team ID are exact
  schema strings compared byte-for-byte. Casing, whitespace, and Unicode
  normalization drift refuse activation.
- Designated requirements are frozen as digests of canonicalized `csreq`
  expressions; the expressions themselves are not stored in the manifest and
  are never emitted by an adapter observation.
- Entitlement lists are frozen only through `entitlements_sha256`. Raw
  entitlement bodies, profile identifiers, and provisioning secrets never
  enter the manifest or an observation.
- The manifest carries no ambient values: no hostnames, users, absolute
  paths, notarization tickets, Apple ID identifiers, keychain items, machine
  UDIDs, or credential material.

## Independence From Linux Evidence

A macOS-signed manifest and any macOS adapter observation apply only to a
`macos-apple-silicon` runtime. Fedora or Ubuntu evidence cannot satisfy any
macOS field above, and macOS evidence cannot satisfy any Linux capability.
The verifier enforces this by rejecting `platform_family` substitution and by
requiring exact runtime, capability-order, mechanism-digest, Team ID, bundle,
App Group, entitlement, designated-requirement, and helper-hash matches
before constructing a `VerifiedPlatformAdapter`.

## Implementation Scope

This document is a field freeze only. It does not add a macOS manifest file,
signature, verifier acceptance, adapter, or capability probe. Those arrive
with the macOS platform increment in the owning sprint and must exactly match
the frozen fields above.
