# Capability Grant and Policy Reference

## Status and Scope

This document describes the shared/Linux implementation of AgentMage Story 5.1 at the source
revision recorded by its generated evidence report. It is a design and review artifact, not a
release claim, durable-storage claim, sandbox claim, or macOS implementation claim.

`CapabilityGrant` is the sole authority-bearing public contract. `ApprovalRequest`, task records,
plans, prompts, tool definitions, policy decisions, consumption records, and lifecycle records are
descriptive or evidentiary objects. They cannot be converted into a grant or satisfy execution
policy.

The current issuer and transitions are in-memory. The encrypted operational store, authenticated
caller boundary, platform path confinement, and isolated tool worker remain later work.

## Grant Schema

The top-level `CapabilityGrant` wire object has exactly 29 required keys. Keys whose values are
nullable are still required in JSON. Unknown keys fail closed.

| Ordinal | Field | Type or closed values | Authority binding |
|---:|---|---|---|
| 1 | `schema_version` | integer, current value `1` | Selects the closed wire contract. |
| 2 | `grant_id` | `GrantId` | Identifies one issuer-retained grant. |
| 3 | `revision` | positive integer | Identifies one immutable lifecycle revision. |
| 4 | `grant_class` | `session_read`, `operation` | Separates parent scope from one operation attempt. |
| 5 | `actor_id` | `ActorId` | Binds the local actor. |
| 6 | `session_id` | `SessionId` | Binds the owning session. |
| 7 | `task_id` | `TaskId` | Binds the user-directed task. |
| 8 | `action_id` | required nullable `ActionId` | Binds an operation action; absent only for a session parent. |
| 9 | `action_kind` | required nullable `ActionKind` | Binds the descriptive action class. |
| 10 | `operation` | `GrantOperation` | Selects one closed operation; no wildcard/custom value exists. |
| 11 | `tool_id` | required nullable `ToolId` | Binds an exact registered tool for operation grants. |
| 12 | `tool_version` | required nullable string | Binds the immutable tool-contract version. |
| 13 | `targets` | ordered `GrantTarget[]` | Binds included componentized workspace-relative targets. |
| 14 | `excluded_targets` | ordered `GrantTarget[]` | Retains inherited excluded subtrees. |
| 15 | `sensitivity` | `DataSensitivity` | Binds the reviewed data class. |
| 16 | `argument_sha256` | lowercase SHA-256 | Binds canonical arguments or the session-scope description. |
| 17 | `preimages` | ordered `GrantPreimage[]` | Binds exact observed target state. |
| 18 | `expected_side_effects` | ordered `GrantSideEffect[]` | Binds operation-typed expected effects. |
| 19 | `rollback_description` | bounded string | Retains the reviewed rollback or recovery statement. |
| 20 | `issued_at_epoch_ms` | unsigned integer | Defines the beginning of the validity interval. |
| 21 | `expires_at_epoch_ms` | unsigned integer | Defines the exclusive end of the validity interval. |
| 22 | `nonce` | `GrantNonce` | Prevents reuse at issuance. |
| 23 | `use_limit` | positive integer | Limits parent derivations or the single operation use. |
| 24 | `use_count` | unsigned integer | Records derivation/consumption count. |
| 25 | `parent_grant_id` | required nullable `GrantId` | Links an operation to its session parent. |
| 26 | `parent_grant_sha256` | required nullable SHA-256 | Binds the exact parent revision used for derivation. |
| 27 | `preview_sha256` | lowercase SHA-256 | Binds the exact user-visible confirmation snapshot. |
| 28 | `policy_sha256` | lowercase SHA-256 | Binds the immutable policy document identity. |
| 29 | `status` | `issued`, `consumed`, `revoked`, `expired`, `invalidated`, `uncertain` | Records the closed lifecycle state. |

### Nested Authority Shapes

| Type | Required fields | Closed rules |
|---|---|---|
| `GrantTarget` | `workspace_id`, `path_components` | Components only; no absolute path, separator, NUL, `.`, `..`, `*`, or `**`. |
| `GrantPreimage` | `target_index`, `content_sha256`, `observed_revision` | Index must name a target; digest is lowercase SHA-256; nullable revision key is required. |
| `GrantSideEffect` | `operation`, `target_indexes`, `details_sha256` | Operation equals the grant operation; indexes name targets; details use a canonical digest. |

### Class Invariants

| Invariant | Session-read parent | Operation child |
|---|---|---|
| Operation | `workspace_read` only | One exact closed operation |
| Action and tool | Required nullable fields are `null` | Exact action, kind, tool, and version |
| Scope | Non-empty included roots and nested exclusions | Intersection of parent inclusions and exclusions |
| Lifetime | Positive and no longer than 24 hours | Positive, begins before parent expiry, never outlives parent |
| Use | 1 to 4,096 derivations | Exactly one use, initially zero |
| Parent | Required nullable fields are `null` | Exact parent identity and pre-derivation revision hash |
| Side effects | Read-only scope description | Non-empty, operation-matching exact effects |

## Policy Document

`PolicyDocument` is canonical JSON over ordered struct fields and `BTreeSet` values. The kernel
computes `policy_sha256`; callers do not supply a trusted policy digest. Every `ScopeRules<T>` has
an exact `allowed` set and exact `denied` set. Membership in both resolves to deny. Absence from
both also resolves to deny.

The document contains actor, task, action, exact tool/version, operation, and target rules;
explicit argument/preimage digest denials; and exact network, credential, and publication rules.
Wildcards, empty/NUL scope values, traversal/separator target components, malformed digests, an
unsupported schema, or revision zero fail policy construction.

## Policy Decision Table

Evaluation returns at the first failing scope in this fixed order. `PolicyEngine::evaluate` never
executes or mutates. `GrantIssuer::consume_for_execution` uses the decision in one exclusive
in-memory transaction and terminalizes a stale otherwise-current operation grant.

| Order | Scope | Stable code | Exact allow condition | On failure during consumption |
|---:|---|---|---|---|
| 1 | `Grant` | `policy.deny.grant` | Current issuer record; schema/class/status/use/policy/lifetime all exact. | Issued stale grant becomes `expired` at/after expiry, otherwise `invalidated`; terminal replay stays terminal. |
| 2 | `Actor` | `policy.deny.actor` | Context actor equals grant actor and exact actor rule allows it. | `invalidated` |
| 3 | `Session` | `policy.deny.session` | Context session equals grant session. | `invalidated` |
| 4 | `Task` | `policy.deny.task` | Context task equals grant task and exact task rule allows it. | `invalidated` |
| 5 | `Action` | `policy.deny.action` | Action identity/kind equal grant and exact action rule allows it. | `invalidated` |
| 6 | `Tool` | `policy.deny.tool` | Tool identity/version equal grant and exact binding rule allows them. | `invalidated` |
| 7 | `Operation` | `policy.deny.operation` | Exact closed operation is allowed and not denied. | `invalidated` |
| 8 | `Path` | `policy.deny.path` | Ordered targets equal grant and every exact target rule allows them. | `invalidated` |
| 9 | `Argument` | `policy.deny.argument` | Canonical argument digest equals grant and is not explicitly denied. | `invalidated` |
| 10 | `Preimage` | `policy.deny.preimage` | Ordered preimages equal grant and no current digest is explicitly denied. | `invalidated` |
| 11 | `SideEffect` | `policy.deny.side_effect` | Ordered expected effects equal the confirmed grant. | `invalidated` |
| 12 | `Preview` | `policy.deny.preview` | Current confirmation digest equals the grant preview digest. | `invalidated` |
| 13 | `Network` | `policy.deny.network` | Exact scope is present and allowed only for `network_access`; otherwise it is absent. | `invalidated` |
| 14 | `Credential` | `policy.deny.credential` | Exact scope is present and allowed only for `credential_access`; otherwise it is absent. | `invalidated` |
| 15 | `Publication` | `policy.deny.publication` | Exact scope is present and allowed only for publication-class operations; otherwise it is absent. | `invalidated` |

An allow returns `policy.allow.exact_grant`. Atomic consumption then verifies the retained revision
hash again, increments revision and use count, sets `consumed`, hashes the terminal revision, and
only then replaces current state. Two consumers cannot both hold the required exclusive borrow.

## Strict-Local Operation Matrix

The named constructor `PolicyEngine::strict_local_read_only` generates the following immutable
operation ceiling. The twelve hazardous classes are present in the explicit deny set. Other
non-read operations are denied by absence.

| Operation | Strict-local disposition | Rule source |
|---|---|---|
| `workspace_read` | Allow only with every exact identity/scope check | Explicit allow |
| `workspace_write` | Deny | Explicit deny |
| `workspace_delete` | Deny | Explicit deny |
| `command_execute` | Deny | Explicit deny |
| `network_access` | Deny | Explicit deny and empty network allow set |
| `git_commit` | Deny | Explicit deny |
| `git_push` | Deny | Explicit deny and empty publication allow set |
| `publish` | Deny | Explicit deny and empty publication allow set |
| `send` | Deny | Explicit deny and empty publication allow set |
| `upload` | Deny | Explicit deny and empty publication allow set |
| `deploy` | Deny | Explicit deny and empty publication allow set |
| `database_read` | Deny | Absent from allow set |
| `database_write` | Deny | Explicit deny |
| `credential_access` | Deny | Explicit deny and empty credential allow set |
| `model_inference` | Deny | Absent from allow set |

The general `PolicyEngine::new` constructor supports isolated tests and future separately gated
profiles. Its existence does not enable a profile. Startup configuration-to-policy binding and
capability registration remain open work.

## Review Checklist

1. Confirm all 29 top-level fields remain required and nested authority shapes reject extras.
2. Confirm no wildcard/custom operation exists and strict-local explicit denials remain complete.
3. Confirm policy digest computation uses deterministic canonical bytes and deny precedence.
4. Confirm evaluation order and redacted codes match the decision table.
5. Confirm consumption performs every fallible check/hash before current-state replacement.
6. Confirm stale, expired, consumed, and uncertain grants cannot return to `issued`.
7. Confirm approval, decision, consumption, and lifecycle records remain non-authoritative.
8. Do not infer durable storage, authenticated IPC, sandboxing, path safety, tool execution, or
   macOS behavior from this shared/Linux reference.

## Limitations

- Grant and policy state is currently in-memory and is not crash durable.
- The real dispatcher does not yet invoke a sandbox worker with the consumed grant.
- Current target components are candidates; Sprint 6 owns canonical descriptor-relative paths.
- Current policy callers are in-process; authenticated local IPC and process ownership are later.
- The strict-local profile is not yet bound to startup configuration by the application host.
- No macOS implementation, execution, signing, sandbox, or packaging evidence is claimed.
