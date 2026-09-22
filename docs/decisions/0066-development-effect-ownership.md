# Decision 0066: Development Effect Ownership

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-22 |
| Authority | Decision 0054, Decision 0063 and Tasks 48.2.4.1-48.2.4.2 |
| Scope | Effect ownership for the disposable Linux coding-development activation |
| Preserves | Thin-client authority limits, opaque effect authorization, exact process identity, private development state, production activation, model admission and evidence distinctions |

## Context

Decision 0063 requires the terminal to start an exact sibling host for the
actual-process development campaign. The first connected implementation put the
native process-launch call in the shell and implemented an opaque-permit consumer
there. Candidate rejection retention also performed directory cleanup directly
in the shell. The repository's structural effect gate correctly rejected those
placements: a presentation shell cannot own native launch, filesystem mutation or
opaque effect authorization merely because the activation is development-only.

Removing the structural checks, registering the shell as a permit owner, or
changing the campaign to an in-process fixture would weaken accepted boundaries
or lose the required executable workflow. The ownership correction must preserve
the already demonstrated transport and coordinator behavior.

## Decision

1. The Linux platform adapter owns the development sibling-host process. Its
   public operation is closed over the exact `agentmage-host` sibling, the fixed
   `--coding-development-host` operation, the enumerated scenario and model
   labels, absolute activation roots, inherited direct envelope pipe and exact
   child lifecycle. It cannot select an arbitrary command or open-ended argument
   list.
2. The terminal continues to validate the complete Decision 0063 activation
   before calling that adapter. It owns presentation, event verification,
   approval input and cancellation requests, but no process-launch API.
3. The Linux platform adapter owns creation and verification of the exact
   mode-`0700` development subdirectories and the paired mode-`0600` rejected
   candidate records. Writes are create-new, substitutions are refused, and a
   failed metadata write removes only the raw file created by the same call.
4. An already verified in-memory observation that needs a durable authority
   receipt uses a kernel-owned inert observation driver. The shell supplies only
   redacted material; it never imports, constructs or consumes
   `EffectAuthorization`.
5. None of these development adapters is a production activation path, model
   admission, general command launcher, general filesystem grant or product
   support claim. Existing exact grants, durable transactions, confinement,
   verifier ownership and independent-review gates remain unchanged.

## Consequences

The actual CLI and host remain separate authenticated processes, while native
effects reside in their accepted platform or kernel owners. The structural
effect checker can continue rejecting shell process launch, shell filesystem
mutation and unregistered permit consumers without an exemption for the coding
harness.
