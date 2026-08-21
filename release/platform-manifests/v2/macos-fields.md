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
- `os_build_min_sha256`, `os_build_tested_sha256`, and `os_build_supported`:
  the frozen macOS build boundary as separately typed manifest fields, defined
  in the "macOS Build Boundary" section below. `os_build_min_sha256` and
  `os_build_tested_sha256` are independent 64-character lowercase hexadecimal
  SHA-256 digests over their canonical build preimages, and `os_build_supported`
  is the closed ordered set of every additionally accepted intermediate build
  digest.
- `toolchain_sha256`: SHA-256 of the normalized Apple SDK plus Swift toolchain
  identity (Xcode command-line tools build, macOS SDK version, and Swift
  toolchain build), pinned to one exact combination per release.
- `vscode_build_sha256`: SHA-256 of the supported Visual Studio Code build
  identity (marketing version plus commit) that the macOS shell targets.
- `distribution_artifact_sha256` and `installed_closure_sha256`: the frozen
  release-distribution artifact digest and the separate runtime-verifiable
  installed component-closure digest, defined in the "macOS Package Identity"
  section below.
- `capabilities`: exactly ten ordered records in the frozen closure order, each
  with the canonical capability name and a nonzero SHA-256 mechanism digest.

## macOS Build Boundary

The build boundary is expressed with three independently typed manifest
fields so that the verifier can recover each boundary value and compare it
against an independently observed native build without the adapter
reproducing the expected tuple:

- `os_build_min_sha256`: 64-character lowercase hexadecimal SHA-256 digest of
  the canonical minimum supported `ProductBuildVersion` preimage. The preimage
  is the exact UTF-8 byte string
  `agentmage.macos-build.v2\nproduct-build-version=<ProductBuildVersion>\n`
  with no additional whitespace, no BOM, and no trailing bytes. The
  `<ProductBuildVersion>` token is the exact ASCII value returned by
  `sw_vers -buildVersion` on the minimum supported build.
- `os_build_tested_sha256`: 64-character lowercase hexadecimal SHA-256 digest
  computed with the same canonical preimage rule over the exact
  `ProductBuildVersion` used by the release qualification run.
- `os_build_supported`: closed ordered JSON array of 64-character lowercase
  hexadecimal SHA-256 digests. Each entry is computed with the same canonical
  preimage rule over one additionally supported intermediate
  `ProductBuildVersion`. The array MUST contain at least
  `os_build_min_sha256` and `os_build_tested_sha256` as its first and last
  entries in that exact order and MUST NOT contain any duplicate entry.

The observed-build field an adapter reports is separately typed:

- The adapter observation records only `observed_os_build_sha256`, computed
  by the adapter over the current build using the same canonical preimage
  rule.
- The verifier accepts activation only when `observed_os_build_sha256` is
  byte-for-byte equal to one entry in the manifest's `os_build_supported`
  array. Any observation not present in that closed set refuses activation
  with `OsBuildMismatch` before workspace access.
- Mutating, adding, or removing any entry in `os_build_supported`, or
  mutating `os_build_min_sha256` or `os_build_tested_sha256`, changes the
  signed manifest bytes and refuses activation with
  `ManifestSignatureInvalid` before workspace access.

## macOS Package Identity

Package identity is split into a signed release-distribution artifact digest
and a signed runtime-verifiable installed component-closure digest so that
startup can validate the installed product after the distribution artifact
has been deleted or unmounted:

- `distribution_artifact_sha256`: 64-character lowercase hexadecimal SHA-256
  digest of the exact notarized, stapled distribution artifact bytes (`.pkg`
  or `.dmg`) as published. This field records the released artifact identity
  and is verified only at install time. It MUST NOT be evaluated at
  subsequent startup because the artifact may be removed after installation.
- `installed_closure_sha256`: 64-character lowercase hexadecimal SHA-256
  digest of the canonical installed component closure. The preimage is the
  exact UTF-8 byte string formed by concatenating, in this fixed order and
  with a single `\n` separator after each line, one line per closure member:
  `agentmage.macos-installed-closure.v2\n`
  followed by exactly four lines
  `component=<name> path=<installed-relative-path> sha256=<lowercase-hex>\n`
  where `<name>` is one of the four fixed component names
  `host`, `bridge`, `xpc_helper`, `inference` in that exact order,
  `<installed-relative-path>` is the exact installed relative path from the
  frozen `bundle_layout` map below, and `<lowercase-hex>` is the
  64-character lowercase hexadecimal SHA-256 of that installed signed
  executable file at rest on disk after installation. The closure has
  exactly four members; no member may be added, removed, renamed, or
  reordered without changing the signed manifest.
- `bundle_layout`: frozen JSON object with the four keys `host`, `bridge`,
  `xpc_helper`, and `inference`, each mapping to the exact installed
  relative path of the signed executable used as the closure preimage.

Startup MUST recompute `installed_closure_sha256` from the installed
components after the distribution artifact is removed or unmounted, and MUST
refuse activation with `PackageMismatch` if any installed signed component
has been mutated, added, removed, or swapped.

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
- `entitlements_map_sha256`: 64-character lowercase hexadecimal SHA-256
  digest over the canonical component-keyed entitlement map defined below.
  The preimage is the exact UTF-8 byte string
  `agentmage.macos-entitlements.v2\n`
  followed by exactly four lines, in this fixed component order,
  `component=<name> sha256=<lowercase-hex>\n`
  where `<name>` is one of `host`, `bridge`, `xpc_helper`, `inference` in
  that exact order and `<lowercase-hex>` is the value of the corresponding
  per-component field below. Adding, removing, renaming, reordering, or
  swapping any component's entitlement digest changes this aggregate digest
  and refuses activation with `ManifestSignatureInvalid` before workspace
  access.
- `entitlements.host`: 64-character lowercase hexadecimal SHA-256 digest over
  the canonicalized, signed entitlement set applied to the host component
  (Hardened Runtime, minimal App Sandbox, the frozen App Group membership,
  and the exact allowed and denied entitlements for the host).
- `entitlements.bridge`: 64-character lowercase hexadecimal SHA-256 digest
  over the canonicalized, signed entitlement set applied to the bridge
  component.
- `entitlements.xpc_helper`: 64-character lowercase hexadecimal SHA-256
  digest over the canonicalized, signed entitlement set applied to the XPC
  helper component.
- `entitlements.inference`: 64-character lowercase hexadecimal SHA-256 digest
  over the canonicalized, signed entitlement set applied to the inference
  runtime component.
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
- Every SHA-256 field is a 64-character lowercase hexadecimal encoding of a
  32-byte value. The complete 32-byte value is required not to equal zero;
  an all-zero digest is rejected as `ManifestUnsupported`. Individual `00`
  octets inside an otherwise nonzero digest are valid. Uppercase, mixed-case,
  short, long, or non-hexadecimal encodings are rejected as
  `ManifestMalformed`.
- Bundle identifiers, the App Group identifier, and the Team ID are exact
  schema strings compared byte-for-byte. Casing, whitespace, and Unicode
  normalization drift refuse activation.
- Designated requirements are frozen as digests of canonicalized `csreq`
  expressions; the expressions themselves are not stored in the manifest and
  are never emitted by an adapter observation.
- Entitlement lists are frozen only through the per-component
  `entitlements.<component>` digests and the aggregate
  `entitlements_map_sha256`. Raw entitlement bodies, profile identifiers,
  and provisioning secrets never enter the manifest or an observation.
- The manifest carries no ambient values: no hostnames, users, absolute
  paths, notarization tickets, Apple ID identifiers, keychain items, machine
  UDIDs, or credential material.

## Independence From Linux Evidence

A macOS-signed manifest and any macOS adapter observation apply only to a
`macos-apple-silicon` runtime. Fedora or Ubuntu evidence cannot satisfy any
macOS field above, and macOS evidence cannot satisfy any Linux capability.
The verifier enforces this by rejecting `platform_family` substitution and by
requiring exact runtime, capability-order, mechanism-digest, Team ID, bundle,
App Group, per-component entitlement, aggregate entitlement-map,
designated-requirement, installed-closure, and helper-hash matches before
constructing a `VerifiedPlatformAdapter`.

## Implementation Scope

This document is a field freeze only. It does not add a macOS manifest file,
signature, verifier acceptance, adapter, or capability probe. Those arrive
with the macOS platform increment in the owning sprint and must exactly match
the frozen fields above.
