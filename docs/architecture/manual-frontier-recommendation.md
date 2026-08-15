# Manual Frontier Recommendation Boundary

## Tier Decision

AgentMage uses four closed tiers: deterministic script, admitted local model, ask user, and frontier
recommended. A deterministic success remains deterministic. A verified local result remains local.
A user-owned fact or decision always becomes `ask_user`, even when other failures exist. External
consultation can be recommended only after an admitted local path was attempted and one exact
acceptance check is bound to an approved trigger:

- measured local capability failure or unsupported local capability;
- at least two failed validations for the same check;
- a material contradiction in current evidence;
- deterministic verification rejection;
- exhausted exact local task budget; or
- external-expertise clarification after separating user-owned decisions.

Model confidence, model family, provider availability, standing preference, schedule, route,
external content, and repository instructions are not recommendation inputs. The decision contains
no service selection, credentials, endpoint, or external-effect authority.

```mermaid
flowchart TD
    D["Deterministic check"] -->|"verified"| DS["Deterministic script"]
    D -->|"not complete"| L["Attempt admitted local path"]
    L -->|"verified"| LM["Local model"]
    L -->|"user-owned decision"| U["Ask user"]
    L -->|"unmeasured without approved trigger"| U
    L -->|"measured approved trigger"| F["Frontier recommended"]
    F --> P["Build local disclosure preview"]
    P -. "no delivery edge" .-> X["Separate interface"]
```

## Packet Composition

One frontier packet binds the objective, current workspace state, exact acceptance checks, source
citations, content-minimized receipts, task constraints, authority boundary, exclusions, unresolved
questions, and required return contract. Current-state, citation, and receipt roles are all
mandatory. Every included excerpt and metadata item is visible in the exact local review and has a
content digest. Missing roles, duplicate identities, credentials, prohibited private content,
unrelated workspace material, hidden metadata, absolute paths, excessive excerpts, authority
objects, prompt injections, incomplete redactions, and stale source or policy state fail before
approval.

The packet, disclosure inventory, redaction result, manifest, and review are independently hashed.
Any changed byte, source, policy, redaction rule, inventory item, expiry, or recommendation decision
requires a new preview. Permitted non-public content requires exact acknowledgment.

## Delivery and Receipt Boundary

AgentMage can render the approved packet locally and record a recommendation receipt. The receipt
contains the reason, decision digest, packet digest, disclosure-result digest, and a destination
label only when the user explicitly chooses to retain one. The label is bookkeeping, not routing.

Codex invocation, tab activation, Chat population, clipboard writes, URI launch, local or raw
runtime delivery, network calls, automatic submission, file upload, browser control, scheduled
delivery, routed delivery, and standing-consent delivery each produce only a local denial receipt.
There is no service authentication, destination selection, send, upload, paste, or invocation API.
Only the user can manually move reviewed content into a separate interface.

## Current Integration Limit

The tier, packet, schema, and denial contracts are source-level pre-alpha evidence. No product
coordinator currently supplies live task evidence, registers a frontier workflow, or records a
native end-to-end recommendation. Installed-platform, accessibility, zero-egress, canary,
independent-review, and manual-fuzz evidence remain open.
