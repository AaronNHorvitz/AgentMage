# Controlled Writes

## Status

Controlled writes are implemented as local contracts but are not product registered or released.
The write profile remains read-only and blocked by `G-V0.2`, `G-V0.3`, and the product
configuration loader. This guide describes the required review and recovery flow; it is not an
instruction to bypass the disabled profile.

## Exact Change Review

One write approval covers one exact shadow change set. The preview identifies every affected file,
operation, complete escaped diff, rationale, expected behavior, verification plan, known risk,
rollback approach, and unverified assumption. It also binds the current workspace, exact source
preimages, expected postimages, policy, expiry, and separately permitted verification labels.

Approval becomes invalid when any target identity, source byte, operation, path, metadata field,
preview, policy, workspace, grant, expected side effect, or expiry changes. A new proposal and
approval are required. Wildcards, recursive changes, implicit parent creation, overwrite by
default, and approval transfer are not part of the contract.

## Controlled Operations

| Operation | Required behavior |
|---|---|
| Create | Destination must be absent under an approved root; parent creation is not implicit |
| Exact patch | Structured patch must apply to the exact current preimage |
| Copy | Source and destination hashes are reviewed; existing destination is refused |
| Move | Source identity and destination collision are checked; restoration evidence is retained |
| Trash-first delete | Separate high-risk approval identifies one exact target; recursive delete is absent |
| Markdown update | Structural edit preserves unrelated bytes and warns or refuses on unsupported syntax |
| Knowledge create | One closed workflow renders one exact file after identity, namespace, and link checks |

## Transaction Flow

1. Open and hash every exact source through the current approved workspace handle.
2. Build the proposed postimages in memory outside user-owned files.
3. Validate syntax, paths, encoding, line endings, operation uniqueness, generated-file policy,
   metadata, limits, and expected postimages.
4. Display the complete change set and recovery narrative.
5. Bind an explicit decision to that exact preview.
6. Derive and consume one short-lived, non-transferable, single-use operation grant.
7. Re-read every held source immediately before application.
8. Apply atomically where the platform supports the exact operation.
9. Freshly verify every expected postimage and operation receipt.
10. Publish derived indexes only after canonical verification.
11. Publish terminal receipts and the next safe checkpoint.
12. Restore known partial changes or stop as uncertain; never guess or replay.

Formatters, tests, builds, migrations, and other commands are not implied by write approval. Each
requires a later separate constrained-command grant. The v0.3 write boundary includes no generic
shell, Git commit or push, network publication, connector, schedule, or unattended write.

## Protected Content

Instruction files, handoffs, source records, secrets, Git metadata, canonical application stores,
unrelated dirty work, excluded paths, links, sockets, devices, out-of-root paths, colliding names,
and concurrently changed files must stop or require a more specific later workflow. The agent may
not erase provenance, reorganize a vault in bulk, promote memory from a prompt, replace later user
work, or silently merge a conflict.

## Recovery

Cancellation before grant consumption leaves no write authority. After consumption, restart must
freshly observe canonical state and may not repeat the operation. A moved root, permission or mount
drift, concurrent change, unavailable secret store, uncertain result, failed restoration, or broken
receipt chain stays visible and blocks completion. Derived indexes rebuild from canonical bytes;
they never replace source truth.

Temporary staging has an owner, digest, expiration boundary, and state. Unknown or quarantined
objects are preserved for review. Attributable orphan cleanup requires a separate proposal and
grant, produces its own hash-chained receipt, and cannot be recorded twice for one staging object.

## Current Limits

The current repository has platform-neutral and Fedora-local contract evidence, but no released
write profile, complete native cross-platform matrix, exhaustive native crash/race campaign,
whole-root residue scan, signed v0.3 package, independent release decision, or completed manual
fuzz campaign. The authoritative status is the blocked
[`v0.3-write-pack-manifest.json`](../../release/v0.3-write-pack-manifest.json).
