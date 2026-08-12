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
