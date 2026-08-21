# Version 2 Signed Release Contract

Version 2 is the exact production wire schema consumed by
`verify_platform_release`. The verifier accepts bounded UTF-8 JSON only when a
detached Ed25519 signature validates over:

```text
agentmage.platform-release-manifest.v2\0 || exact_manifest_bytes
```

The JSON object rejects unknown or missing fields and contains:

- `schema_version`: integer `2`;
- `record_type`: `platform-release-manifest`;
- `status`: `signed-release`;
- `adapter_api_version`: the current exact adapter API;
- `platform_family`: `fedora` or `ubuntu`;
- `architecture`: `x86_64` or `aarch64`;
- exact SHA-256 values for operating-system build, toolchain, supported Visual
  Studio Code build, and installed AgentMage package; and
- exactly ten ordered capability records, each with the canonical capability
  name and a nonzero expected mechanism SHA-256.

The expected public key is supplied independently from native adapter
observations. A signature, signer, schema, status, runtime, order, capability,
or mechanism mismatch refuses activation before workspace access.

This directory intentionally contains no manifest JSON, signature, private key,
or claimed release identity. Such files may be added only by the later reviewed
release pipeline and must represent the exact package and native evidence under
review.

## Frozen macOS-only fields

macOS platform activation is not yet implemented. Every future
`platform_family: "macos-apple-silicon"` manifest must carry the shared
signed-release fields plus every field in `FROZEN_MACOS_MANIFEST_FIELDS`
(defined in the kernel contracts crate and enumerated in
`docs/architecture/platform-adapter-contract.md`):

- `platform_build_sha256`
- `architecture` (frozen to `aarch64`)
- `toolchain_sha256`
- `team_id`
- `host_bundle_id`
- `helper_bundle_id`
- `app_group`
- `host_entitlements_sha256`
- `helper_entitlements_sha256`
- `designated_requirement_sha256`
- `helper_hashes`
- `package_sha256`
- `vscode_build_sha256`

No macOS signed release fixture is committed. The current verifier refuses
every manifest whose `platform_family` is `macos-apple-silicon` with
`ManifestUnsupported`, and macOS evidence cannot satisfy any Linux gate.
