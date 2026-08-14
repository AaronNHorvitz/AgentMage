# AgentMage v0.1 Local Incident-Response Runbook

## Status

This is a versioned pre-release runbook for Sprint 25. It defines local-first incident preparation
but does not record an actual incident, execute `RV-21`, authorize external communication, or claim
that a supported release exists.

## Authority Boundary

An incident does not create unlimited collection, filesystem, credential, network, publication, or
remote-control authority. AgentMage may be suspended locally, inspected through already authorized
content-free diagnostics, and repaired through an explicitly selected signed local package.

There is no telemetry channel, remote kill switch, silent update, background patch download,
automatic external notification, or package-supplied trust root. Any communication occurs outside
the AgentMage runtime through a separately authorized human process.

## Roles

| Role | Responsibility | Prohibited shortcut |
|---|---|---|
| Reporter or device owner | Report the bounded symptom and preserve control of private data. | Sending raw prompts, repositories, credentials, keys, or broad archives. |
| Incident lead | Own severity, scope, suspension, decisions, communications, and closure. | Self-approving evidence produced by a component under investigation. |
| Technical investigator | Reproduce with synthetic or minimized facts and identify exact affected identities. | Expanding local authority because a diagnosis is difficult. |
| Evidence custodian | Preserve approved content-free evidence, access, retention, and hold state. | Treating an incident as permanent retention authority. |
| Remediation owner | Produce the fix, revocation, disable policy, or recovery procedure. | Signing or approving the remediation alone. |
| Independent verifier | Rebuild, verify, test, and reconstruct the outcome. | Accepting summaries without raw source-bound records. |
| Communications owner | Prepare bounded notices through approved human channels. | Allowing AgentMage to send a notice automatically. |

One person may hold multiple roles in a small project, but the record must identify conflicts and
must retain an independent verifier for release-impacting conclusions.

## Common State Machine

Every scenario follows these ordered transitions:

1. **Detected:** record UTC time, reporter role, exact release/component/profile identity if known,
   content-free symptom, and confidence. Unknown identity remains Unknown/Blocked.
2. **Locally suspended:** prevent new model loading, capability registration, and work acceptance
   through local disablement, process stop, or uninstall. Preserve user repositories and unrelated
   data.
3. **Contained:** remove acquisition or network authority, quarantine exact suspect artifacts,
   isolate affected local state, and prevent publication. Do not destroy evidence.
4. **Evidence bounded:** preview the minimum approved records, assign sensitivity and retention,
   scan synthetic canaries, encrypt when durable, and record access. No automatic export occurs.
5. **Severity assigned:** state impact, affected identities, uncertainty, exploitability,
   confidentiality/integrity/availability effect, and response target.
6. **Owner assigned:** name the incident lead, remediation owner, verifier, communications owner,
   and next decision point.
7. **Communication prepared:** identify who needs notice and the minimum content. Human approval and
   external tools remain outside AgentMage.
8. **Remediated:** apply an exact signed local disable policy, replacement package, model/runtime
   revocation, configuration recovery, or state restore through an approved local transaction.
9. **Verified:** rerun affected tests, complete boundary tests, offline proof, recovery, and
   supported-platform checks. Failure preserves suspension.
10. **Recovered:** restore only exact verified components and state; confirm prior or new supported
    configuration, no replay, and no undeclared residue.
11. **Lessons recorded:** capture timeline, decisions, failures, open risks, corrective actions,
    owners, due criteria, retention, and closure decision.

Ambiguous, late, duplicate, conflicting, and false-positive signals remain visible events. They do
not erase earlier records or silently reopen authority.

## Scenario A: Suspected Egress

**Detection:** an unexpected connection, listener, DNS event, route, packet, or external URI appears
in a strict-local phase.

**Immediate suspension and containment:** stop AgentMage model and tool processes; preserve the
bounded process/socket observation; remove acquisition authority; block publication; keep the
device owner's network response process separate.

**Minimum evidence:** release and configuration digests, process executable and start identities,
namespace/cgroup identity hashes, socket endpoints by class, strict-local ledger range, packet count
and destination class, relevant content-free receipts, and clock facts. Do not retain packet payload,
prompt, file content, browser data, unrelated process detail, or credential material.

**Verification and recovery:** determine whether attribution is complete, reproduce in an isolated
fixture, test all declared topology and hostile-network gates, inspect for undeclared source and
runtime paths, issue a signed local disable or replacement if needed, then rerun a complete live
zero-egress workflow before recovery.

## Scenario B: Compromised Dependency or Package

**Detection:** signer, source, lock, SBOM, provenance, artifact, repository, package, or distribution
identity is missing, altered, revoked, unsupported, or reported compromised.

**Immediate suspension and containment:** block build and publication, disable the exact affected
release/component, quarantine artifacts without executing them, preserve prior known-good packages,
and reject package-provided trust roots.

**Minimum evidence:** source commit/tree/archive digests, dependency lock and SBOM identities,
package manifest and payload hashes, signer and trust-root identifiers, provenance result,
revocation/support state, and verification reason codes. Never retain a private signing key.

**Verification and recovery:** independently reacquire source and trust material, rebuild twice in
the clean environment, compare bytes, exercise wrong-signer/corrupt/revoked cases, rotate or revoke
identities as required, and recover only to a newly approved exact release or verified prior release.

## Scenario C: Prompt-Injection Disclosure

**Detection:** untrusted repository, attachment, model, tool, or handoff content attempts to conceal
sources, broaden authority, extract secrets, mislabel evidence, or trigger an external action.

**Immediate suspension and containment:** cancel the task, revoke pending grants, retain only typed
denial and source identities, quarantine suspect parsed material if policy requires, and do not
follow links or instructions in the content.

**Minimum evidence:** content hash, workspace-relative source/range, parser identity, injection
classification, proposed versus admitted action identities, denial receipt, disclosure result, and
redaction findings. Raw content is retained only when separately approved and essential; secrets are
never retained.

**Verification and recovery:** replay a synthetic equivalent through authority-escalation,
disclosure, secret-canary, citation, and tool matrices; verify no side effect or hidden omission;
invalidate affected maps/evidence; and resume only with regenerated current evidence.

## Scenario D: Model or Runtime Revocation

**Detection:** an exact artifact, model profile, codec, tokenizer, template, runtime, image, driver,
or adapter becomes revoked, unsupported, quarantined, incompatible, or materially degraded.

**Immediate suspension and containment:** unload or prevent load, remove it from ordinary selection,
preserve the active predecessor, deny automatic substitution, and block affected release claims.

**Minimum evidence:** complete profile tuple, artifact and runtime digests, lifecycle and support
state, quality/repeatability tuple, activation manifest, health reason, and local unload receipt. Do
not retain model weights in incident exports.

**Verification and recovery:** verify signed revocation and support state, test unload/quarantine and
restart, evaluate a replacement independently through the same neutral admission boundary, require
explicit user selection, and rerun quality, safety, offline, and lifecycle evidence.

## Scenario E: Key Store or Cryptographic Failure

**Detection:** secret service is locked or substituted; key retrieval, signature, encryption,
decryption, checkpoint, backup, trust root, or algorithm identity fails.

**Immediate suspension and containment:** enter local safe mode, stop model/capability registration,
do not retry with a weaker provider, preserve encrypted artifacts unchanged, and prevent new writes
that depend on uncertain key state.

**Minimum evidence:** provider/client executable digest, algorithm and key identifiers without key
bytes, operation type, bounded error class, storage/checkpoint identity, and failure receipt. Never
collect secret values, private keys, recovery phrases, environment values, or command history.

**Verification and recovery:** validate provider identity, access policy, backup identity, and
cryptographic inventory; test wrong-key and corruption behavior against disposable fixtures; restore
only through the explicit verified process; rotate affected keys and trust roots outside product
logs; verify canary absence before reopening.

## Scenario F: Corrupted Operational State

**Detection:** schema, migration, checkpoint chain, receipt chain, database page, retention event,
backup, configuration, or restart reconciliation is incomplete, inconsistent, or corrupt.

**Immediate suspension and containment:** stop the single writer, preserve the exact encrypted
preimage and backup, do not overwrite or auto-repair, invalidate pending authority, and classify
uncertain effects as Unknown/Blocked.

**Minimum evidence:** schema and migration version, database and checkpoint identity hashes,
last verified sequence, corruption class, backup identity, configuration digest, and recovery
result. Do not export rows containing prompts, files, credentials, or private metadata.

**Verification and recovery:** reproduce with a synthetic corrupted copy, verify prior backup and
key, restore to a fresh candidate, reconcile every grant/effect/receipt transition, prove no replay,
then atomically select the complete valid state. Failure leaves the original untouched and blocked.

## Support Evidence Allowlist

Support may request only content-free items needed for the named scenario:

- exact product, component, configuration, policy, profile, runtime, package, and platform digests;
- typed state, reason, remediation, denial, cancellation, and receipt codes;
- bounded process/socket/network classes without payload;
- source revision, command inventory, exit code, output hash, and test limitation;
- sanitized event sequence, UTC time, role, retention, and access decision; and
- user-reviewed diagnostic export manifest and redaction result.

Support must not request raw prompts, model responses, workspace or repository files, credentials,
tokens, cookies, private keys, recovery secrets, full environment dumps, shell history, browser or
email profiles, unrelated paths, packet payloads, memory dumps, databases, home-directory archives,
or unreviewed diagnostic bundles.

## Retention and Closure

Every retained record has an owner, purpose, sensitivity, access set, creation time, expiration,
deletion or hold disposition, and evidence hash. An incident hold is separate, scoped, approved,
time-bounded authority; it cannot be inferred from severity.

Closure requires a reconstructable timeline, verified recovery or explicit continued suspension,
bounded communication decision, canary scan, unresolved-risk list, corrective actions and owners,
retention decision, and independent review. A false positive is closed as a false positive without
deleting the reasoning that established it.
