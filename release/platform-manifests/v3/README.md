# Version 3 Reserved macOS Signed Release Contract

Schema version 3 is the reserved signed release contract for the future
`macos-apple-silicon` platform. No macOS manifest JSON, signature, verifier
acceptance, adapter, or fixture is committed under this directory: the
verifier continues to refuse `macos-apple-silicon` as `ManifestUnsupported`
until the full macOS increment lands in its owning sprint.

The exact field set, canonical preimages, domain separator, and closure
counts that a future v3 manifest must carry are frozen in
[`../v2/macos-fields.md`](../v2/macos-fields.md). That freeze covers the
platform build boundary, architecture, toolchain, Apple Team ID, bundle
identifiers, App Group identifier, per-component and aggregate
entitlement digests, per-component designated-requirement digests,
per-component helper hashes, split distribution-artifact and installed
closure package digests, additional signed inventory, and the pinned
Visual Studio Code build identity. Adding, removing, renaming, or
reordering any frozen field or any frozen array member is a schema
change that requires a new signed schema version and a new domain
separator, not a v3 amendment.

The version-3 signed envelope binds Ed25519 signatures to the exact
byte string

```text
agentmage.platform-release-manifest.v3\0 || exact_manifest_bytes
```

The version-3 domain separator is byte-distinct from the version-2
domain separator (`agentmage.platform-release-manifest.v2\0`), so no
signed v2 Linux manifest bytes can be reinterpreted as a v3 macOS
record and no signed v3 macOS manifest bytes can be reinterpreted as a
v2 Linux record. Linux evidence cannot satisfy any macOS field and
macOS evidence cannot satisfy any Linux capability.

The freeze is enforced by
[`scripts/macos_platform_manifest_contract.py`](../../../scripts/macos_platform_manifest_contract.py)
and its companion test
[`tests/test_macos_platform_manifest_contract.py`](../../../tests/test_macos_platform_manifest_contract.py).
Any drift in the frozen field set, canonical preimages, closure counts,
or the reservation described here fails the contract before the manifest
implementation can land.
