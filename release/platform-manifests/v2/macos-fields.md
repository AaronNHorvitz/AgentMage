# Frozen macOS Release Manifest Fields

This document freezes the exact set of fields a future macOS release manifest
must carry as a separately versioned signed release record. Because the closed
v2 wire schema authenticated by
[`release/platform-manifests/v2/README.md`](README.md) fixes the Linux runtime
identity (single `os_build_sha256` and single `package_sha256`) and rejects
unknown or missing fields, the macOS record cannot be added as a v2 amendment.
It is therefore allocated to a distinct schema version, schema version 3, with
its own signature domain separator, so that no macOS manifest bytes are ever
interpreted as a v2 record and no signed v2 manifest bytes are ever reinterpreted
under the macOS shape. Freezing the fields before implementation prevents scope
drift, ambient inputs, or platform substitution. macOS remains unimplemented:
no macOS manifest JSON, signature, verifier acceptance, or activation path
exists in this repository, and the verifier continues to refuse
`macos-apple-silicon` as `ManifestUnsupported` until the full macOS increment
lands under schema version 3.

## Trust Envelope

The macOS manifest is one bounded UTF-8 JSON object with `deny_unknown_fields`,
signed by a detached Ed25519 signature that validates over the exact frozen
version-3 domain separator and manifest bytes:

```text
agentmage.platform-release-manifest.v3\0 || exact_manifest_bytes
```

The version-3 domain separator is byte-distinct from the version-2 domain
separator (`agentmage.platform-release-manifest.v2\0`), so any Ed25519
signature that verifies under one domain cannot verify under the other. The
verifier selects the domain separator solely from the outer schema version,
which is itself covered by the signature, so a signed v2 Linux manifest cannot
be reinterpreted as a v3 macOS manifest and a signed v3 macOS manifest cannot
be reinterpreted as a v2 Linux manifest.

The expected verifying key enters startup independently from any macOS adapter
observation. Signatures, signers, schema, status, runtime, capability order,
and macOS-specific fields cannot be selected, echoed, or substituted by an
adapter.

## Shared Runtime Identity (frozen from the Linux baseline)

- `schema_version`: integer `3`.
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
- `toolchain_sha256`: 64-character lowercase hexadecimal SHA-256 digest over
  the canonical Apple toolchain preimage defined in the "macOS Toolchain
  Identity" section below, pinned to one exact combination per release.
- `vscode_build_sha256`: 64-character lowercase hexadecimal SHA-256 digest
  over the canonical Visual Studio Code build preimage defined in the "macOS
  Visual Studio Code Build Identity" section below, pinned to one exact
  supported build per release.
- `distribution_artifact_sha256`, `installed_closure_sha256`, and
  `additional_signed_inventory_sha256`: the frozen release-distribution
  artifact digest, the separate runtime-verifiable four-member installed
  runtime component-closure digest, and the separate two-member additional
  signed inventory digest that authenticates the signed model installer and
  signed Visual Studio Code extension, defined in the "macOS Package
  Identity" section below.
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
  `agentmage.macos-build.v3\nproduct-build-version=<ProductBuildVersion>\n`
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

## macOS Toolchain Identity

The `toolchain_sha256` digest freezes exactly one Apple SDK plus Swift
toolchain quadruple per release. The preimage is the exact UTF-8 byte string

```text
agentmage.macos-toolchain.v3\nxcode-command-line-tools-build=<XcodeCLTBuild>\nmacos-sdk-version=<MacOSSDKVersion>\nswift-marketing-version=<SwiftMarketingVersion>\nswiftlang-build=<SwiftlangBuild>\n
```

with no BOM, no additional whitespace, no comment lines, no reordering of the
four key lines, and no trailing bytes beyond the final `\n`. The four tokens
are the authoritative values:

- `<XcodeCLTBuild>` is the exact ASCII value of the `ProductBuildVersion`
  key read from `/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/System/Library/CoreServices/SystemVersion.plist`
  on the pinned Xcode Command Line Tools installation used to produce the
  release, extracted with `plutil -extract ProductBuildVersion raw -o -`.
- `<MacOSSDKVersion>` is the exact ASCII value of the `Version` key read
  from `SDKSettings.plist` inside the pinned `MacOSX.sdk` bundle, extracted
  with `plutil -extract Version raw -o -`.
- `<SwiftMarketingVersion>` is the exact ASCII marketing version emitted by
  `swift --version` after the literal token `Swift version` and the single
  space, taken up to the next whitespace or `(` character, from the pinned
  Swift toolchain used to compile every signed component of the release. This
  is the Swift language marketing version only (for example `6.2.3`) and by
  itself does not uniquely identify a compiler build.
- `<SwiftlangBuild>` is the exact ASCII compiler build identifier of the same
  pinned Swift toolchain. It is extracted by locating the first
  parenthesized group emitted by `swift --version` that immediately follows
  `<SwiftMarketingVersion>` and that begins with the literal seven-byte
  prefix `swiftlang`, then taking every byte from the `s` of `swiftlang` up
  to the first whitespace or `)` character inside that group. For output
  `Apple Swift version 6.2.3 (swiftlang-6.2.3.3.20 clang-1700.0.13.5)`, the
  extracted value is exactly `swiftlang-6.2.3.3.20`. For output
  `Apple Swift version 6.2.3 (swiftlang-6.2.3.3.21 clang-1700.0.13.5)`, the
  extracted value is exactly `swiftlang-6.2.3.3.21`. Two `swift --version`
  outputs that share `<SwiftMarketingVersion>` but differ in
  `<SwiftlangBuild>` therefore produce different toolchain preimages and
  different `toolchain_sha256` values; two byte-identical outputs produce
  identical preimages and identical `toolchain_sha256` values.

Each token MUST match the regular expression `[0-9A-Za-z._-]+` and MUST NOT
contain whitespace, `=`, `\n`, `(`, `)`, or `\0`. If any authoritative source
is missing, empty, or non-ASCII, or if `swift --version` does not emit a
`swiftlang`-prefixed build identifier in the first parenthesized group
following `<SwiftMarketingVersion>`, the release build refuses to emit a
manifest.
Any deviation from the four fixed key lines, their order, their `key=value`
form, their trailing `\n`, or the absence of any other byte produces a
different digest and refuses activation with `ManifestSignatureInvalid`
before workspace access. Independent implementations given identical pinned
Xcode Command Line Tools, macOS SDK, and Swift toolchain inputs MUST
produce byte-identical preimages and therefore identical
`toolchain_sha256` values; mutating any one of the four tokens changes
only `toolchain_sha256`.

## macOS Visual Studio Code Build Identity

The `vscode_build_sha256` digest freezes exactly one supported Visual Studio
Code desktop build per release. The preimage is the exact UTF-8 byte string

```text
agentmage.macos-vscode.v3\nmarketing-version=<MarketingVersion>\ncommit=<Commit>\n
```

with no BOM, no additional whitespace, no comment lines, no reordering of
the two key lines, and no trailing bytes beyond the final `\n`. The two
tokens are the authoritative values:

- `<MarketingVersion>` is the exact ASCII value of the `version` key read
  from the Visual Studio Code application bundle's
  `Contents/Resources/app/product.json` file for the pinned macOS Apple
  Silicon Visual Studio Code build.
- `<Commit>` is the exact ASCII value of the `commit` key read from the
  same `product.json` file for the same pinned build.

`<MarketingVersion>` MUST match `[0-9]+\.[0-9]+\.[0-9]+`. `<Commit>` MUST
be a 40-character lowercase hexadecimal string. Neither token may contain
whitespace, `=`, `\n`, or `\0`. Any deviation from the two fixed key lines,
their order, their `key=value` form, their trailing `\n`, or the absence of
any other byte produces a different digest and refuses activation with
`ManifestSignatureInvalid` before workspace access. Independent
implementations given identical pinned Visual Studio Code inputs MUST
produce byte-identical preimages and therefore identical
`vscode_build_sha256` values; mutating either token changes only
`vscode_build_sha256`.

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
  digest of the canonical installed runtime component closure. The preimage
  is the exact UTF-8 byte string formed by concatenating, in this fixed
  order and with a single `\n` separator after each line, one line per
  closure member:
  `agentmage.macos-installed-closure.v3\n`
  followed by exactly four lines
  `component=<name> path=<installed-relative-path> sha256=<lowercase-hex>\n`
  where `<name>` is one of the four fixed component names
  `host`, `bridge`, `xpc_helper`, `inference` in that exact order,
  `<installed-relative-path>` is the exact installed relative path from the
  frozen `bundle_layout` map below, and `<lowercase-hex>` is the
  64-character lowercase hexadecimal SHA-256 of that installed signed
  executable file at rest on disk after installation. The closure has
  exactly four runtime members; no runtime member may be added, removed,
  renamed, or reordered without changing the signed manifest. Additional
  signed shipped components (the signed model installer, the signed
  Visual Studio Code extension bundle) are authenticated separately by
  `additional_signed_inventory_sha256` below and are excluded from the
  four-member runtime closure preimage.
- `bundle_layout`: frozen JSON object with the four keys `host`, `bridge`,
  `xpc_helper`, and `inference`, each mapping to the exact installed
  relative path of the signed executable used as the closure preimage.
- `additional_signed_inventory_sha256`: 64-character lowercase hexadecimal
  SHA-256 digest of the canonical inventory of every other Team-ID-signed
  shipped component installed under the frozen bundle root. The preimage
  is the exact UTF-8 byte string formed by concatenating, in this fixed
  order and with a single `\n` separator after each line:
  `agentmage.macos-additional-signed-inventory.v3\n`
  followed by exactly two lines
  `component=<name> path=<installed-relative-path> sha256=<lowercase-hex>\n`
  where `<name>` is one of the two fixed additional component names
  `model_installer`, `vscode_extension` in that exact order,
  `<installed-relative-path>` is the exact installed relative path from the
  frozen `additional_signed_inventory` map below, and `<lowercase-hex>` is
  the 64-character lowercase hexadecimal SHA-256 of that installed signed
  component at rest. The `model_installer` entry names the signed installer
  or importer required by the `ModelInstallation` capability under
  [`RUNTIME-BOUNDARIES.md`](../../../RUNTIME-BOUNDARIES.md);
  the `vscode_extension` entry names the signed Visual Studio Code
  extension bundle shipped alongside the bridge. The inventory has exactly
  two members; no additional signed component may be added, removed,
  renamed, reordered, mutated, or substituted without changing the signed
  manifest bytes.
- `additional_signed_inventory`: frozen JSON object with the two keys
  `model_installer` and `vscode_extension`, each mapping to the exact
  installed relative path of the corresponding signed component used as
  the `additional_signed_inventory_sha256` preimage.
- `installed_root_token`: closed schema token from the fixed enumeration
  `{ "applications-agentmage-bundle-v3" }` that names the installed bundle
  root prefix under which every signed component in the release resides.
  The verifier translates the token to its fixed absolute-path binding
  internally; the manifest carries no absolute path. The four
  `bundle_layout` paths and the two `additional_signed_inventory` paths
  are interpreted relative to the token-bound root, and the union of the
  six paths is byte-disjoint at the schema level.

Startup MUST recompute `installed_closure_sha256` from the installed runtime
components after the distribution artifact is removed or unmounted, and MUST
refuse activation with `PackageMismatch` if any of the four listed installed
signed runtime components has been mutated, removed, or swapped, or if any of
the four listed runtime paths is not present, is not a regular file, or is
not signed by the frozen Team ID. Startup MUST separately recompute
`additional_signed_inventory_sha256` from the installed signed model
installer and signed Visual Studio Code extension and MUST refuse activation
with `PackageMismatch` if either listed additional signed component has been
mutated, removed, or swapped, or if its listed path is not present, is not a
regular file, or is not signed by the frozen Team ID.

Because `installed_closure_sha256` is defined over exactly the four fixed
`bundle_layout` entries and `additional_signed_inventory_sha256` is defined
over exactly the two fixed `additional_signed_inventory` entries, neither
preimage changes when a seventh executable or signed Mach-O appears at an
unlisted path. To detect such an addition, startup MUST additionally perform
an installed-inventory completeness check before workspace access:

- Recursively enumerate every regular file under the absolute path
  bound to `installed_root_token`, following no symbolic links.
- For each enumerated file, refuse activation with `PackageMismatch`
  if the file is a Mach-O binary (magic `0xFEEDFACE`, `0xFEEDFACF`,
  `0xCEFAEDFE`, `0xCFFAEDFE`, `0xCAFEBABE`, or `0xCAFEBABF`), or is
  a `.dylib`, `.so`, `.bundle`, `.framework` binary, or is otherwise
  signed under the frozen Team ID, unless its installed relative path
  is byte-for-byte equal to one of the four values in `bundle_layout`
  or to one of the two values in `additional_signed_inventory`.
- The check MUST run after the four runtime components and the two
  additional signed components have been verified and MUST run before
  any workspace, configuration, key, or state authority is granted.

The installed closure digest authenticates the identity of the four listed
runtime components; the additional signed inventory digest authenticates the
identity of the signed model installer and signed Visual Studio Code
extension; the installed-inventory completeness check authenticates the
absence of every unlisted executable or signed component. All three checks
are required, and any failure refuses activation with `PackageMismatch`
before workspace access. A release fixture containing the expected signed
model installer at its declared path passes closure verification; removing,
mutating, substituting, or adding an undeclared installer, or adding any
other undeclared Team-ID-signed Mach-O anywhere under the frozen bundle
root, refuses activation with `PackageMismatch` before workspace access.

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
- `bundle_ids.model_installer`: reverse-DNS bundle identifier of the signed
  model installer or importer component required by the `ModelInstallation`
  capability.
- `bundle_ids.vscode_extension`: reverse-DNS bundle identifier of the signed
  Visual Studio Code extension bundle shipped alongside the bridge.
- `app_group_id`: exact `group.*` App Group identifier that owns the mode
  `0600` Unix socket container shared across the host, bridge, XPC helper, and
  inference components.
- `entitlements_map_sha256`: 64-character lowercase hexadecimal SHA-256
  digest over the canonical component-keyed entitlement map defined below.
  The preimage is the exact UTF-8 byte string
  `agentmage.macos-entitlements.v3\n`
  followed by exactly six lines, in this fixed component order,
  `component=<name> sha256=<lowercase-hex>\n`
  where `<name>` is one of `host`, `bridge`, `xpc_helper`, `inference`,
  `model_installer`, `vscode_extension` in that exact order and
  `<lowercase-hex>` is the value of the corresponding per-component field
  below. Adding, removing, renaming, reordering, or swapping any
  component's entitlement digest changes this aggregate digest and refuses
  activation with `ManifestSignatureInvalid` before workspace access.
- `entitlements.host`: 64-character lowercase hexadecimal SHA-256 digest
  over the canonical per-component entitlement preimage defined in the
  "Per-Component Entitlement Canonicalization" section below, computed for
  the host component (Hardened Runtime, minimal App Sandbox, the frozen App
  Group membership, and the exact allowed and denied entitlements for the
  host).
- `entitlements.bridge`: 64-character lowercase hexadecimal SHA-256 digest
  over the canonical per-component entitlement preimage for the bridge
  component.
- `entitlements.xpc_helper`: 64-character lowercase hexadecimal SHA-256
  digest over the canonical per-component entitlement preimage for the XPC
  helper component.
- `entitlements.inference`: 64-character lowercase hexadecimal SHA-256
  digest over the canonical per-component entitlement preimage for the
  inference runtime component.
- `entitlements.model_installer`: 64-character lowercase hexadecimal
  SHA-256 digest over the canonical per-component entitlement preimage for
  the signed model installer component.
- `entitlements.vscode_extension`: 64-character lowercase hexadecimal
  SHA-256 digest over the canonical per-component entitlement preimage for
  the signed Visual Studio Code extension component.
- `designated_requirements.host`: 64-character lowercase hexadecimal SHA-256
  digest over the canonical per-component designated-requirement preimage
  defined in the "Per-Component Designated-Requirement Canonicalization"
  section below, computed for the host component.
- `designated_requirements.bridge`: 64-character lowercase hexadecimal
  SHA-256 digest over the canonical per-component designated-requirement
  preimage for the Visual Studio Code bridge helper.
- `designated_requirements.xpc_helper`: 64-character lowercase hexadecimal
  SHA-256 digest over the canonical per-component designated-requirement
  preimage for the XPC helper service.
- `designated_requirements.inference`: 64-character lowercase hexadecimal
  SHA-256 digest over the canonical per-component designated-requirement
  preimage for the inference runtime component.
- `designated_requirements.model_installer`: 64-character lowercase
  hexadecimal SHA-256 digest over the canonical per-component
  designated-requirement preimage for the signed model installer component.
- `designated_requirements.vscode_extension`: 64-character lowercase
  hexadecimal SHA-256 digest over the canonical per-component
  designated-requirement preimage for the signed Visual Studio Code
  extension component.
- `helper_hashes.host`: SHA-256 of the signed host executable file at rest.
- `helper_hashes.bridge`: SHA-256 of the signed Visual Studio Code bridge
  helper executable at rest.
- `helper_hashes.xpc_helper`: SHA-256 of the signed XPC helper executable at
  rest.
- `helper_hashes.inference`: SHA-256 of the signed inference-runtime binary at
  rest.
- `helper_hashes.model_installer`: SHA-256 of the signed model installer
  executable at rest.
- `helper_hashes.vscode_extension`: SHA-256 of the signed Visual Studio Code
  extension bundle payload at rest.

## Per-Component Entitlement Canonicalization

Each `entitlements.<component>` digest is computed over an exact
UTF-8 preimage derived deterministically from the signed embedded
entitlements slot of the corresponding installed component. The
authoritative extraction and canonicalization procedure is:

1. Read the installed signed executable file identified by
   `bundle_layout.<component>` (for `host`, `bridge`, `xpc_helper`,
   `inference`) or by `additional_signed_inventory.<component>` (for
   `model_installer`, `vscode_extension`) at rest after installation.
2. Extract the raw bytes of the `CSMAGIC_EMBEDDED_ENTITLEMENTS` slot
   (`0xFADE7171`) from the embedded code signature superblob. Take the
   payload bytes only, meaning the slot bytes with the 8-byte
   `SuperBlob`/`GenericBlob` header (magic and length) stripped. This
   payload is the exact XML plist that `codesign --display
   --entitlements :-` emits for the component.
3. Compute
   `entitlement_blob_sha256_<component>` as the 64-character lowercase
   hexadecimal SHA-256 of those exact payload bytes with no
   normalization, no re-encoding, no whitespace trimming, and no
   plist re-serialization.
4. Compute
   `entitlement_der_sha256_<component>` as the 64-character lowercase
   hexadecimal SHA-256 of the exact payload bytes of the
   `CSMAGIC_EMBEDDED_ENTITLEMENTS_DER` slot (`0xFADE7172`) from the
   same embedded code signature superblob, with the 8-byte
   `SuperBlob`/`GenericBlob` header stripped. Both entitlement slots
   MUST be present, MUST be nonempty, and MUST cover the identical
   entitlement identity; if either slot is missing, empty, or
   semantically divergent from the other, the release build refuses
   to emit a manifest.

The per-component preimage is the exact UTF-8 byte string

```text
agentmage.macos-entitlement.v3\ncomponent=<name>\nblob-sha256=<entitlement_blob_sha256_component>\nder-sha256=<entitlement_der_sha256_component>\n
```

with `<name>` one of `host`, `bridge`, `xpc_helper`, `inference`,
`model_installer`, or `vscode_extension`; no BOM, no additional
whitespace, no comment lines, no reordering of the three key lines,
and no trailing bytes beyond the final `\n`. `entitlements.<component>`
is the 64-character lowercase hexadecimal SHA-256 of that preimage.

Representation-only variation that does not change either signed
slot's payload bytes (for example, canonically identical output from
two different signing runs) MUST yield identical per-component
digests. Any semantic mutation of any entitlement key or value in
either slot changes the corresponding slot's payload bytes and
therefore both the per-component digest and `entitlements_map_sha256`.
Independent implementations given identical installed signed
components MUST produce byte-identical per-component and aggregate
entitlement digests.

## Per-Component Designated-Requirement Canonicalization

Each `designated_requirements.<component>` digest is computed over
an exact UTF-8 preimage derived deterministically from the signed
`CSMAGIC_REQUIREMENTS` internal requirements set of the corresponding
installed component. The authoritative extraction and canonicalization
procedure is:

1. Read the installed signed executable file identified by
   `bundle_layout.<component>` (for `host`, `bridge`, `xpc_helper`,
   `inference`) or by `additional_signed_inventory.<component>` (for
   `model_installer`, `vscode_extension`) at rest after installation.
2. Extract the exact payload bytes of the designated-requirement
   slot (`kSecCodeMagicRequirement` = `0xFADE0C00`) that
   `codesign --display --requirements -` selects as the designated
   requirement, meaning the individual `Requirement` blob bytes with
   the 8-byte magic-and-length header retained. This is the
   authoritative canonical binary form emitted by Apple's code-signing
   tooling and matches the bytes produced by
   `csreq -b -r'<designated expression>'` for the same expression.
3. Compute
   `designated_requirement_bin_sha256_<component>` as the 64-character
   lowercase hexadecimal SHA-256 of those exact payload bytes with no
   normalization, no re-encoding, and no expression reformatting.

The per-component preimage is the exact UTF-8 byte string

```text
agentmage.macos-designated-requirement.v3\ncomponent=<name>\ncsreq-sha256=<designated_requirement_bin_sha256_component>\n
```

with `<name>` one of `host`, `bridge`, `xpc_helper`, `inference`,
`model_installer`, or `vscode_extension`; no BOM, no additional
whitespace, no comment lines, no reordering of the two key lines,
and no trailing bytes beyond the final `\n`.
`designated_requirements.<component>` is the 64-character lowercase
hexadecimal SHA-256 of that preimage. The raw expression text is
never stored in the manifest and is never emitted by any adapter
observation.

Textual representation-only variation of the source expression that
produces byte-identical compiled `Requirement` blob bytes MUST yield
identical per-component digests. Any semantic mutation of the
designated requirement (identifier, anchor, certificate leaf, Team
ID, or subject constraints) changes the compiled blob bytes and
therefore the per-component digest. Independent implementations
given identical installed signed components MUST produce
byte-identical per-component designated-requirement digests.

## Field-level Freeze Rules

- The union above is closed. Adding, removing, or renaming a field, or
  changing the ordering of any frozen array (including `os_build_supported`
  and `capabilities`), of the four fixed runtime component entries inside
  `installed_closure_sha256` and `bundle_layout`, of the two fixed
  additional signed component entries inside
  `additional_signed_inventory_sha256` and `additional_signed_inventory`,
  or of the six fixed component entries inside `entitlements_map_sha256`,
  `entitlements.*`, `designated_requirements.*`, `helper_hashes.*`, and
  `bundle_ids.*`, is a manifest-schema change that requires a new signed
  schema version and a new domain separator, not a v3 amendment. JSON
  object member order inside the manifest is NOT normative: the signed
  exact manifest bytes authenticate whatever object member order the
  signer emitted, and the verifier's `deny_unknown_fields` membership
  check accepts any member order that a conforming JSON parser emits.
  Object member reordering therefore does not require a new schema version
  and does not by itself refuse activation; only field membership and the
  array orderings above are frozen at the schema level.
- Every SHA-256 field is a 64-character lowercase hexadecimal encoding of a
  32-byte value. The complete 32-byte value is required not to equal zero;
  an all-zero digest is rejected as `ManifestUnsupported`. Individual `00`
  octets inside an otherwise nonzero digest are valid. Uppercase, mixed-case,
  short, long, or non-hexadecimal encodings are rejected as
  `ManifestMalformed`.
- Bundle identifiers, the App Group identifier, and the Team ID are exact
  schema strings compared byte-for-byte. Casing, whitespace, and Unicode
  normalization drift refuse activation.
- Designated requirements are frozen through the per-component canonical
  preimage digests defined in the "Per-Component Designated-Requirement
  Canonicalization" section, which are computed over the exact compiled
  `Requirement` blob bytes from the installed component's code signature.
  The raw `csreq` expressions themselves are not stored in the manifest and
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
Because the Linux manifest is a signed schema version 2 record under the v2
domain separator and the macOS manifest is a signed schema version 3 record
under the v3 domain separator, no Linux v2 manifest bytes can be
reinterpreted as a macOS v3 record and no macOS v3 manifest bytes can be
reinterpreted as a Linux v2 record: their Ed25519 signature envelopes are
byte-distinct, and the outer schema-version field (itself covered by the
signature) determines which variant's field set and domain separator apply.
The verifier additionally enforces this at the field level by rejecting
`platform_family` substitution and by requiring exact runtime,
capability-order, mechanism-digest, Team ID, bundle, App Group,
per-component entitlement, aggregate entitlement-map, designated-requirement,
installed-closure, additional-signed-inventory, and helper-hash matches
before constructing a `VerifiedPlatformAdapter`. Cross-variant fields (for
example the Linux `os_build_sha256` or `package_sha256` appearing inside a
v3 record, or any macOS-only field appearing inside a v2 record) are
rejected by `deny_unknown_fields`, and missing required fields inside
either variant are rejected as `ManifestMalformed`.

## Implementation Scope

This document is a field freeze only. It does not add a macOS manifest file,
signature, verifier acceptance, adapter, or capability probe. Those arrive
with the macOS platform increment in the owning sprint and must exactly match
the frozen fields above.
