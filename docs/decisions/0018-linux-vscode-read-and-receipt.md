# Decision 0018: Linux VS Code Read and Receipt

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-11 |
| Scope | One bounded Linux repository read through the VS Code language-model chat surface |
| Resolves | `RM-018` |
| Preserves | Decisions 0001 through 0017, strict-local operation, exact-object authority, and independent release trust |

## Context

The repository has a durable kernel authority transaction, exact descriptor-held
Linux paths, a single-use effect permit, an offline Bubblewrap worker, encrypted
receipts, and authenticated local IPC. The Rust host and VS Code extension are
still inert scaffolds. No user-facing path composes those controls.

Phase 9 must prove one narrow product workflow without creating a general file
browser, an ambient shell, or a model-selected authority path. It must also
preserve the Phase 8 rule that an unsigned development executable cannot claim
to be a supported package. Independently signed package artifacts and automatic
host bootstrap remain Phase 11 work.

## Decision

### Product Surface

1. The VS Code extension registers one language-model chat provider with vendor
   `agentmage` and model ID `secure-local-read` during activation and disposes
   that registration during deactivation.
2. The provider accepts only the closed command `read <workspace-relative-path>`.
   Empty paths, absolute paths, traversal, wildcard syntax, encoded separators,
   remote workspaces, virtual workspaces, and multiple selected workspace roots
   fail before a host request.
3. The extension obtains the root only from the one active local VS Code
   workspace. Prompt text cannot select an absolute root or another workspace.
4. The extension renders bounded text, one local file citation, one receipt
   identity, and content-free denial codes. It never renders host error text,
   process details, environment values, native state paths, or worker standard
   error.

### Preview and Approval

1. The first host request is descriptive only. The host canonicalizes the
   relative path, selects the exact workspace, resolves and continuously holds
   one regular file, computes its preimage, and returns a non-authoritative
   preview.
2. The preview binds the workspace identity, canonical path components, exact
   held-object target, preimage, tool call, expected no-change effect, expiry,
   and policy digest into the existing `ApprovalRequest.confirmation_sha256`.
3. VS Code displays the canonical relative path, bounded byte count, sensitivity,
   and the fact that no state change is permitted in a modal confirmation. A
   model response, prompt phrase, prior approval, timeout, or default action
   cannot approve the operation.
4. The approval request returns only the exact preview identity and confirmation
   digest. A mismatch, expiry, cancellation, stale object, missing pending
   preview, or second use starts no worker.

### Authority and Execution

1. The host creates a session-read parent for the selected workspace and derives
   one exact single-use `WorkspaceRead` grant only after confirmation.
2. The operation grant, policy context, tool call, approval identity, argument
   digest, preimage, preview digest, and held target must agree exactly.
3. `DurableAuthorityRuntime::execute_effect` remains the sole launch boundary.
   The host may construct a `LinuxSandboxEffectDriver`, but it cannot construct,
   copy, serialize, or inspect an `EffectAuthorization`.
4. The worker receives one immutable projection of the held file, no adjacent
   workspace object, no network namespace, no ambient environment, no
   credentials, and fixed process, memory, runtime, and output limits.
5. The host returns UTF-8 text only within the product output bound. Binary,
   oversized, unsuccessful, cancelled, timed-out, or uncertain results return a
   bounded terminal disposition without partial content.

### Replay, Recovery, and Cancellation

1. Pending previews are memory-only, expire quickly, and are consumed before
   grant derivation. Host restart therefore invalidates every unapproved preview.
2. A completed authority transaction and receipt remain canonical in encrypted
   storage. Restart recovery never launches an interrupted or completed attempt.
3. Repeating an approval after completion returns a replay denial and the
   verifiable retained receipt identity when available; it never repeats the
   read to reconstruct content.
4. Cancellation or extension deactivation before durable consumption removes
   the pending preview and starts no worker. The Linux worker runtime bound is
   the fail-closed deadline after launch; Phase 9 does not claim cooperative
   mid-process cancellation.
5. Worker failure closes the consumed attempt with a terminal receipt. Host
   failure after any durable pre-launch checkpoint is reconciled by the existing
   restart protocol to a non-replayable terminal or explicit uncertain state.

### IPC and Packaging Boundary

1. Wire messages use one closed, versioned, length-bounded schema with unknown
   fields denied. Request and response identifiers are bounded and contain no
   native authority.
2. Production messages are admitted only on a Phase 8 authenticated local
   channel bound to the verified peer identity and one-use launch credentials.
   A parsed JSON message, socket connection, or VS Code provider request is not
   authentication.
3. The extension owns only authenticated IPC client behavior. It cannot launch
   a process, listen on a socket, inspect credentials, or read repository files
   directly. The host owns composition but cannot register UI or approve a
   request.
4. Phase 9 provides the provider, protocol, host workflow, and deterministic
   composition harness. Normal activation remains unavailable when the
   independently signed host package, trusted endpoint, or launch credentials
   are absent.
5. Phase 11 owns signed package production, installation, trusted endpoint
   discovery, direct credential delivery, and clean-install execution. Test
   signing keys, synthetic adapters, and harness channels cannot enter a
   production activation path or support claim.

## Verification

Phase 9 verification must include:

- extension activation, provider registration, deactivation, cancellation, and
  absent-host behavior;
- command, workspace, path, request-size, unknown-field, version, and identifier
  mutation;
- preview mismatch, expiry, cancellation, stale preimage, missing approval, and
  replay before any worker launch;
- one exact successful read, bounded UTF-8 output, local citation, and durable
  receipt identity;
- adjacent-file, symbolic-link, hard-link, mount, replacement, and worker
  process/network escape attempts;
- worker nonzero exit, output limit, runtime limit, worker crash, host crash at
  every durable checkpoint, restart, and receipt-chain verification; and
- source-boundary tests proving the VS Code shell has no filesystem, process,
  credential, listener, or internet authority.

The signed-package clean-install case remains explicitly unavailable until
Phase 11. Phase 9 may not convert a synthetic harness result into release
evidence.

## Consequences

- AgentMage gains one narrow, inspectable product path without introducing a
  general coding agent, model inference, repository indexing, write operation,
  terminal, Git operation, or network access.
- Approval is visible and exact, while all authority remains in the kernel and
  Linux platform boundary.
- Completed reads are auditable after restart, but file content is not retained
  merely to make replay convenient.
- The source can exercise the complete workflow before packaging, while the UI
  remains candidly unavailable outside a verified installation.
- Phase 9 cannot claim a supported product installation. That claim requires the
  Phase 11 signer, artifacts, package, installer, and clean-install evidence.

## Approval Record

The user accepted this decision and authorized the Phase 9 local commit and
entry into Phase 10 on 2026-08-11. That approval does not authorize a push or a
Phase 10 commit.
