# Decision 0022: Detached Package Signing Boundary

| Field | Value |
|---|---|
| Status | Accepted implementation boundary |
| Date | 2026-08-12 |
| Scope | Deterministic signable Linux bundles, external Ed25519 signing, and installed-payload verification against an external trust root |
| Advances | Package-level portions of `RV-01`, `RV-22`, and Sprint 25 release preparation |
| Blocks | Production signer approval, trust-root distribution, RPM/DEB repository signing, trusted VS Code host bootstrap, support, and release |
| Preserves | Decisions 0001 through 0021, no committed private key, no automatic update authority, and no pre-release support claim |

## Context

Decision 0020 introduced deterministic unsigned RPM, DEB, and VSIX candidates
and intentionally made signed-release verification unavailable. That prevented
an unsigned candidate from being relabeled as a release, but it left no bounded
path for a later isolated release ceremony to create or verify a detached
signature.

AgentMage needs package-level cryptographic mechanics before trusted bootstrap
and release lifecycle work can be exercised. Those mechanics must not invent a
production identity, retain a private key, or allow package contents to supply
their own trust root.

## Decision

1. A Linux release bundle contains a version 2 package manifest with status
   `signed-release`, a positive release sequence, one exact package identity,
   and the same closed host, VSIX, and license payload set.
2. The exact manifest bytes are signed with Ed25519 over
   `agentmage.package-manifest.v2\0 || manifest_bytes`.
3. The release signer reads exactly one raw 32-byte private seed from standard
   input, checks it against a separately supplied raw public key, zeroizes the
   input buffer, writes one new 64-byte detached signature, and refuses to
   overwrite an existing output.
4. The signer never accepts a private-key repository path or command-line
   value. No production key, test private key, credential, or signature output
   is retained in the repository.
5. Signed verification opens the package root, manifest, detached signature,
   public key, and payload fail closed; verifies the detached signature before
   parsing release claims; rejects symlinks, changed inputs, wrong sizes,
   multiple links, unknown fields, wrong class, nonpositive sequence, wrong
   signer, and payload mutation; then revalidates the exact payload.
6. The detached signature and trust root remain outside the RPM or DEB payload.
   A package cannot authenticate the public key used to verify itself.
7. The signable bundle is not a signed release until an approved external
   ceremony creates the detached signature and an independently distributed
   trust root selects the signer. RPM/DEB repository signatures and the trusted
   VS Code-to-host bootstrap remain separate gates.

## Verification

- Rust tests cover exact signed success, wrong trust root, manifest mutation,
  relabel refusal, candidate separation, payload mutation, symlinked payload
  parents, exact private-seed input, signer/public-key mismatch, and output
  overwrite refusal.
- Python tests cover the closed version 2 manifest, release identity, positive
  sequence, deterministic payload, and candidate/release separation.
- `scripts/package_release_lifecycle.py` builds the real release binary and
  VSIX twice, compares RPM/DEB/VSIX/manifest bytes, creates an ephemeral
  synthetic signing identity outside the repository, verifies extracted RPM
  and DEB payloads, exercises wrong-key/class/mutation refusals, and retains no
  key or release output.

## Consequences

- Signing and verification mechanics can be integrated into bootstrap and
  release tests without weakening the external trust boundary.
- A real release still requires protected signing credentials, key ceremony,
  independent trust-root delivery, OS-package signatures, clean lifecycle
  evidence, trusted extension bootstrap, and review.
- Current product, model, platform, package, and support status do not change.
