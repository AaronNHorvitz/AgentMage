# Approved test envelopes and exact reruns

The local coding owner can normalize a separately approved command's test-result
envelope and expose failure names. It uses the existing kernel validation
templates, normalizer and rerun planner, and the same native executor and
canonical authority as [ordinary commands](coding-commands.md). T-303 and T-306
remain open. Positive native execution remains unmeasured in this lane under the
unchanged executable-trust refusal. Component outcomes use synthetic executors;
terminal approval tests use a synthetic HTTP peer.

## Definition and approval

Only a worktree successfully created and held by this running service can supply
execution scope. Start with an exact sealed `CommandSpec`, as documented for
ordinary commands. The definition request is a closed JSON object:

| Field | Meaning |
| --- | --- |
| `command` | Complete sealed kernel command specification; the existing offline, read-only worktree and resource limits apply |
| `kind` | `unit`, `integration` or `end_to_end`; other check kinds refuse |
| `minimum_test_count` | Positive minimum executed passing tests required for a pass |
| `approve_definition` | Explicit Boolean `true` confirms this exact definition, including repeatable setup; it does not approve execution |
| `prior_receipt_sha256` | Absent/null initially; exact last failed receipt for a later separately approved rerun |

The owner constructs the immutable template. The parser is always the existing
`AgentMageJsonV1`, version `1.0.0`, bound to the actual normalizer source digest.
The definition's source and input-confirmation digests bind the canonical command,
kind and minimum count. Those deterministic hashes are provenance bindings, not
authentication or execution authority. The control token authenticates definition
selection; the subsequent full preview approval authorizes one execution.
There are no caller-selected parser, artifact, baseline or failure-attribution
fields. The template declares a focused test check and repeatable temporary setup.

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> validation-prepare '<definition-json>'
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> validation
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> validation-run <absolute-management-directory>/worktree
```

Preparation verifies ownership, the exact definition and prior result, and the
existing native executable/launcher manifest. It launches nothing and replaces
the pending command slot only on success. Commands and validations share that
slot; each inspect route exposes its own preview kind. Their decision routes
cannot approve each other's previews. `validation-deny` consumes the validation
preview without executing it.

Execution shows the complete escaped preview and requires exact
`test <preview-sha256>` consent on the controlling terminal. The client verifies
the displayed root before and after consent and sends once. The outer digest
binds the command, template, minimum count, root, creation receipt, prior receipt,
attempt, budget and expiry. Models and ordinary client tokens have no authority
over these routes. `approve_definition: true` alone cannot execute anything.

| Route | Request/result |
| --- | --- |
| `GET /v1/code/repository/validation` | No body; process-only pending preview, last normalized receipt, attempt count and limits |
| `POST /v1/code/repository/validation/prepare` | Exact definition object above; returns an inert preview |
| `POST /v1/code/repository/validation/decide` | Exact `preview_sha256` and Boolean `approve`; returns both command and validation outcomes |

All three require the control token. Unknown, duplicate, positional and wrongly
typed request fields refuse. API cancellation remains unavailable after admission;
the existing native timeout and descendant cleanup apply.

## Result envelope and scope

This entry point supports commands that already emit the kernel's result envelope
on stdout. It does not convert ordinary test-runner prose into counts. A plain
test-runner transcript becomes `malformed`, even with exit zero. For example:

```json
{
  "schema_version": 1,
  "status": "passed",
  "passed": 2,
  "failed": 0,
  "skipped": 0,
  "duration_ms": 1,
  "failed_names": [],
  "artifact_ids": [],
  "retry_count": 0,
  "initial_failure_sha256": null
}
```

The unchanged kernel checks the command receipt and captured output digests,
termination, output bounds, envelope consistency and minimum test count. Assertion
failure, compilation failure, malformed output, skipped-only output and too few
tests remain distinct. A zero command exit cannot replace a passing envelope.
The result includes `validation.receipt` and `validation.validation_passed`.
The terminal exits unsuccessfully for a failed validation even when the command
itself exited successfully.

`verification_scope: approved-command-envelope` describes this result. Its scope
digest binds the held worktree root and creation receipt, not a source-content
snapshot. The envelope producer remains trusted to describe its own tests;
normalization is not independent task grading. No full-suite coverage, current
repository correctness, failure attribution or coding-model score is established.
`current_workspace_verified` and `test_suite_verified` remain false. Command output
and failed names are untrusted local data; the terminal escapes them. Output is
classified restricted, without a new secret-scanning claim.

## Bounded, separately approved reruns

The first admitted execution fixes one immutable definition for the created
worktree in this process, with at most three admitted executions. Each attempt requires a new preview and exact
consent. After an assertion failure, prepare the unchanged definition with the
exact last receipt digest. The existing kernel rerun planner binds the prior
failure, unchanged command and new approval. A pass, unknown prior receipt,
changed definition, exhausted budget or unrepeatable result refuses selection.
Compilation failure has no failed-name set under this parser and therefore cannot
select the existing failed-test rerun planner in this composition.

Denial and expiry spend no execution attempt. Dispatch spends an attempt before
authority execution and clears the last result. Uncertain dispatch, corrupted
output or lost in-process completion leaves the series blocked, with no retry
authorization. Canonical authority receipts remain inspectable. A failed reply
must not be automatically resent.

The budget and normalized last receipt are process-only. A new successfully
created worktree starts a new series. Restart loses ownership, previews, results
and budgets; it cannot restore command authority from receipts. These limits
bound this manual validation series, not a durable coding run or the separate
ordinary-command API. Source changes between attempts are not classified as
flakiness. There is no automatic model feedback/repair, connected coordinator
retry, durable run recovery, owned-edit binding, commit, push or independent grade.
