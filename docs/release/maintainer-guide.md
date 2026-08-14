# AgentMage v0.1 Maintainer Release Guide

## Status and Roles

This is a pre-release procedure. It does not authorize publication. The builder, signer, platform
operator, evidence reviewer, security reviewer, accessibility reviewer, and release approver must be
recorded. A person or component must not independently create and approve its own evidence where an
independent review is required.

## Release Preconditions

Do not create a production release candidate until:

- every v0.1 owning sprint has completed its first required execution;
- every open v0.1 task has current evidence or remains an explicit release blocker;
- the reviewed revision has a clean index, clean worktree, no untracked or disallowed ignored data,
  and no unresolved repository state;
- dependency locks, source inventory, schemas, requirement registry, architecture decisions,
  security references, and supply-chain records are current;
- production profile, runtime, package, configuration, support, signer, and trust-root identities are
  immutable and reviewed; and
- every required platform and independent reviewer is available.

## Integrated Local Gates

Run from the exact reviewed clean revision:

```bash
npm ci --ignore-scripts
npm run docs:check
npm run product:check
npm run requirements:check
npm run supply-chain:check
npm run artifact-scan:check
npm run phase11:check
npm run release-signing:test
```

Also execute every applicable `RV-01` through `RV-22` protocol through its owning script or retained
manual procedure. The final result registry must list the exact command, source revision,
environment, start and stop time, exit status, output hash, skipped and ignored cases, limitation,
and evidence path. A passing aggregate command cannot hide an ignored native test.

Manual fuzzing remains deferred to Sprint 166 by Decision 0025. That deferral blocks any release
criterion that requires current `RV-15`; it is not a waiver.

## Clean Build and Package Identity

Follow [`product-ci-and-clean-build.md`](../product-ci-and-clean-build.md). Build from the complete
recursive Git tree in the declared immutable environment. Acquire dependencies only in the
separated acquisition phase. Verification runs offline with a read-only root filesystem, fixed
unprivileged identity, dropped capabilities, no new privileges, and bounded temporary storage.

Build RPM, DEB, VSIX, and later platform packages twice. Compare bytes, manifests, payload sets,
modes, versions, configuration identity, component identity, and source-to-package provenance. Any
difference blocks signing.

## Signing Boundary

Production private keys must not exist in this repository, build output, logs, environment, command
arguments, diagnostics, or test fixtures. The signing operator receives the exact reviewed manifest
digest through a separately controlled process and returns a detached signature and signer identity.

Verify with a trust root obtained independently from the package. Refuse wrong signer, missing
signature, altered manifest, altered payload, downgrade, revoked component, unsupported platform,
expired support state, and candidate/release relabeling. Synthetic lifecycle keys prove mechanics
only and can never sign a release.

## Platform Matrix

The v0.1 declaration requires the identical supported workflow on the recorded MacBook Pro M5,
Fedora, and Ubuntu environments. Windows work is additive first-GA planning but does not substitute
for a declared v0.1 platform requirement. For each supported platform retain:

1. Exact hardware, firmware, operating system, security-control, toolchain, VS Code, and package
   identity without private host identifiers.
2. Three independent clean standard-user install, first-run, model import, activation, offline,
   workflow, evidence export, recovery, and uninstall runs.
3. Malformed, replayed, wrong-session, wrong-signer, wrong-peer, stale-profile, cancellation,
   resource, and control-disablement matrices.
4. Keyboard, focus, screen-reader, live-region, zoom, reflow, contrast, cancellation, and error
   accessibility evidence.
5. Process, socket, namespace, route, filesystem, secret-store, and packet observations sufficient
   to establish the declared local boundary.

Linux results cannot substitute for macOS. A GitHub macOS runner proves only the bounded command it
executes and does not establish MacBook Pro M5, signing, notarization, install lifecycle, hardware
fit, or accessibility evidence.

## Model and Workflow Gate

Use one explicitly selected exact admitted profile; assume no family. Retain complete profile,
artifact, license, provenance, codec, tokenizer, template, context, decoding, runtime, hardware,
quality, repeatability, policy, support, activation, and limitation identities.

Exercise model discovery, immediate revalidation, inference, tools, repository map, evidence,
citations, diagnostics, cancellation, crash recovery, context pressure, quarantine, removal, and
explicit model change through native Chat. No fallback, hidden retry, cloud route, stale launch, or
completion claim without verifier evidence is permitted.

## Incident and Patch Readiness

Before approval, execute the four `RV-21` scenarios and complete `RV-22` manual patch matrix with
independent named participants. Validate suspected egress, compromised package/dependency,
prompt-injection disclosure, and model/runtime revocation. Exercise valid, wrong-signer, downgrade,
interrupted, corrupt, manifest-mismatched, migration-failed, rollback, revoked-component, and
end-of-support patch states.

Support evidence must be bounded and redacted. Search retained artifacts for synthetic canaries and
prohibited host identity. Emergency disablement and patching are explicit local actions; no remote
kill switch, telemetry path, silent update check, or package-supplied trust root is permitted.

## Release Manifest and Review

The final manifest binds the source revision and tree, complete artifact set, hashes, signer,
configuration, component and profile identities, platform results, requirement and acceptance
results, clean-build record, SBOM, provenance, security review, support period, revocation state,
known limitations, and release notes.

Two independent reviewers reconstruct the result from raw evidence. The release is blocked if any
required item is failed, skipped, stale, unavailable, flaky, quarantined, suppressed, unreviewed, or
bound to another revision. Never edit an evidence result to make it current; rerun the owning gate.

## Publication and Rollback

Publication is a separate explicit authority after approval. Push only the reviewed commit and exact
signed artifacts. Do not rewrite history, force-push, prune, publish from a dirty repository, or
include unrelated files. Verify published bytes from the distribution location against the release
manifest before announcement.

Retain the prior supported release and tested rollback path. A failed publication, installation, or
migration suspends the new release locally and preserves the prior package and data. Publish clear
manual recovery instructions and support state. Uninstall must remove AgentMage-owned package state
without deleting user repositories or unrelated data.
