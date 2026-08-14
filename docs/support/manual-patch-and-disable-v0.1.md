# AgentMage v0.1 Manual Patch and Emergency Disable Procedure

## Status

This is a pre-release local exercise procedure. Existing fixtures use synthetic keys, synthetic
packages, and synthetic advisories. They do not install a product patch, activate a production
disable policy, provide a production signer, or establish platform support.

## Fixed Safety Properties

- User initiation and local file selection are mandatory.
- Acquisition occurs outside the normal AgentMage runtime.
- The trust root is local and obtained independently from the package.
- Detached Ed25519 signatures, signer threshold, exact artifact hashes, release sequence, current
  release precondition, platform, support, revocation, migration, and rollback are verified.
- Verification occurs before extraction or activation.
- Activation is atomic, durable, and explicitly approved.
- The prior valid package or policy remains until post-activation verification succeeds.
- Interruption selects the complete prior state or complete new state, never a partial mix.
- Network access, automatic checks, background download, remote trigger, remote control, telemetry,
  and workstation-data transmission are false.

## Retained Synthetic Fixtures

The manual patch corpus is indexed by
[`cases.json`](../../fixtures/support/manual-patch/cases.json). It covers valid, wrong-signer,
downgrade, corrupt, platform-mismatched, interrupted, revoked, unsupported-version,
migration-failure, and rollback states. Metadata binds package, configuration, component inventory,
SBOM, cryptographic BOM, model BOM, provenance, authority delta, capability delta, advisory, and
rollback artifacts.

The emergency-disable fixture is
[`emergency-disable-policy.valid.json`](../../schemas/support/examples/emergency-disable-policy.valid.json).
It contains signed synthetic block entries for exact model artifact, runtime, component, capability,
and release version subjects. Evaluation occurs before ordinary authority at startup, model load,
runtime load, component load, capability registration, and work acceptance.

## Local Verification

Run from the exact reviewed repository revision:

```bash
python3 scripts/manual_patch_metadata.py
python3 scripts/manual_patch_verifier.py
python3 scripts/emergency_disable_policy.py
node --test tests/test_manual_patch_schema.mjs tests/test_emergency_disable_schema.mjs
python3 -m unittest tests.test_manual_patch_metadata tests.test_manual_patch_verifier tests.test_emergency_disable_policy
```

Expected fixture-level results are:

- all ten patch cases produce their exact declared outcome;
- only the valid case may proceed past verification;
- interruption and failed postcheck preserve or restore the exact prior fixture;
- all five exact disable subject classes block before authority evaluation;
- malformed, expired, downgraded, unknown, hash-mismatched, or untrusted disable material is rejected;
- no fixture retains production key material, private user data, network authority, remote control,
  or product activation; and
- reports retain no release or actual-incident claim.

## Exercise Sequence

1. Record source revision, clean-tree status, operator, verifier, fixture identities, and tool
   versions.
2. Verify formal schemas and source hashes before reading a case as trusted metadata.
3. Execute the valid patch case through selection, metadata verification, package verification,
   compatibility verification, staging, explicit approval, activation simulation, postcheck, and
   completion.
4. Execute each adversarial patch case independently. Confirm one exact refusal and no changed prior
   state.
5. Interrupt after every state transition. Confirm restart selects the prior complete state or the
   complete staged state according to the recorded transaction.
6. Simulate failed post-activation verification and verify complete rollback.
7. Evaluate every disable subject class, unknown subject, wrong hash, altered policy, expiration,
   deletion, downgrade, and attempted remote unblock.
8. Search outputs and reports for synthetic canaries, raw key material, private data fields, network
   authority, remote-control claims, product-activation claims, and release claims.
9. Have an independent verifier recompute outcomes and source hashes.
10. Retain only content-free results, limitations, role decisions, and follow-up owners.

## Production Readiness Requirements

Fixture success is insufficient for `RV-22`. A production exercise additionally requires:

- a reviewed production signer ceremony and independently distributed trust root;
- exact signed release and disable bundles for each supported platform;
- platform-native standard-user staging, activation, postcheck, rollback, restart, and uninstall;
- startup and registration integration that consumes signed support and revocation state;
- current package, schema migration, encrypted data, and configuration compatibility evidence;
- explicit support start/end and revoked/unsupported behavior;
- no secret, prompt, repository, or host-identity leakage in evidence;
- complete clean-build and source-to-package provenance; and
- independent security and release approval.

Until those requirements pass, the fixtures remain synthetic contract evidence and no patch or
disable policy may be described as production-ready.
