# Decision 0055: Versioned rustup Acquisition for the Clean Build

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-20 |
| Scope | How `rustup-init` is obtained and verified inside the reproducible Linux clean-build image |
| Authority | Decision 0054 standing owner delegation, and the owner's explicit direction to resolve this question using the agent's own recommendation |
| Preserves | Every clean-build container control, the fail-closed digest check, the pinned base images, and the offline verification boundary |

## Problem

`release/clean-build/Containerfile.linux` fetched `rustup-init` from the
**unversioned** URL `https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init`
while `architecture/clean-build-policy.json` pinned an exact SHA-256 of that
artifact. That URL always serves the current rustup release, so the pin and the
artifact drift apart whenever rustup publishes. They had drifted: the recorded
pin was `4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10`, and
on 2026-09-20 the URL served
`dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71`.

The image build therefore failed closed at `sha256sum --check --strict`, which is
the control behaving correctly. The defect is the addressing scheme, not the
check: a digest pin against a mutable URL cannot hold, and every future rustup
release would break the clean build again.

## Decision

1. The clean-build image obtains `rustup-init` from the **immutable versioned
   archive path**
   `https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/x86_64-unknown-linux-gnu/rustup-init`.
   The unversioned `/rustup/dist/` path is **never** used again for this or any
   other clean-build input.
2. The pinned acquisition is recorded in `architecture/clean-build-policy.json`
   under `download_integrity`, so the version and its digest live together in one
   policy-controlled place:
   - `rustup_init_version`: **`1.29.1`**
   - `rustup_init_linux_x64_sha256`:
     **`dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71`**
3. Both values are passed into the image build as build arguments by
   `scripts/clean_build_evidence.py`, so the policy remains the single source of
   truth and the Containerfile cannot silently disagree with it.
4. The in-image `sha256sum --check --strict` verification is retained exactly as
   written. Digest verification is not replaced by the versioned URL; the two are
   independent controls and both must pass.
5. Advancing `rustup_init_version` in future is ordinary delegated maintenance
   under Decision 0054, provided the new digest is verified against the published
   `rustup-init.sha256` for that exact version before the pin moves.

## Verification performed

On 2026-09-20 this agent resolved the current stable rustup release from
`https://static.rust-lang.org/rustup/release-stable.toml`, which reported
`version = '1.29.1'`. It then downloaded
`https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init`
(21,113,232 bytes) and the publisher's own
`rustup-init.sha256` for that exact path. The published digest read
`dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71  *./rustup-init`
and the locally computed SHA-256 of the downloaded artifact was identical. The
same digest matches what the unversioned URL served on the same day, which
confirms 1.29.1 is the release that had rotated into that path.

The downloaded artifact was hashed only. It was never made executable and never
run outside the clean-build container, and the local copy was deleted after
verification.

## Limitations

This decision fixes acquisition and verification. It makes no claim that the
clean build passes, that any platform is qualified, or that Story 9.1 is closed.
Those remain determined by the actual clean-build and package-lifecycle evidence.
