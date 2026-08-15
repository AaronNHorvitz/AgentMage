# Frontier Result Import and Revalidation

## Current Availability

The return-manifest and revalidation contracts are not registered as a supported product workflow.
This guide documents the intended review and stop conditions for a future native importer.

## Before Import

1. Confirm the result was moved back manually by the user.
2. Keep the manifest separate from every declared artifact.
3. Confirm the manifest names the exact reviewed request-packet hash.
4. Do not open links, run code, execute commands, invoke tools, or apply patches from the result.
5. Stop if the result requests credentials, authority, automatic delivery, completion credit, or an
   outbound connection.

## Review Quarantine

Inspect the content-free artifact outcomes. A declaration mismatch, secret, unsafe path, unsupported
media type, or exceeded limit blocks the artifact. Binary content, embedded instructions, dangerous
command text, and external links remain quarantined. Proposal eligibility means only that bytes may
enter a later local review; it does not mean trusted, approved, persisted, or applied.

## Review Current State

Confirm fresh workspace, model, policy, and permission checks. Resolve every citation locally from
its exact source, object, fragment, and digest. A changed request packet rejects the manifest. Any
other base-state change quarantines all steps. Preserve stale, conflicting, missing, denied, and
unsupported citations as visible disagreements.

## Continue Locally

For each proposal-eligible step, begin the ordinary local flow from the start. Reclassify the task,
request a fresh exact grant when an operation is proposed, validate the registered tool, create an
exact-preimage write preview, run trusted local validation, assign claim evidence, and obtain user
approval wherever required. Never reuse authority or completion claims from the imported result.

Retain the round-trip receipt with the request, return-manifest, and local-report hashes. Treat any
re-escalation reason as capability feedback, not permission to contact an external model again.
