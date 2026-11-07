# Approved project tests and exact reruns

The local coding owner can normalize a separately approved command's selected
test-output protocol and expose failure names. It uses the existing kernel validation
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
| `output_format` | Optional closed protocol: `agent_mage_json_v1` (default), `rust_libtest_text_v1` or `python_unittest_text_v1`; selection is fixed before execution |
| `approve_definition` | Explicit Boolean `true` confirms this exact definition, including repeatable setup; it does not approve execution |
| `prior_receipt_sha256` | Absent/null initially; exact last failed receipt for a later separately approved rerun |

The owner constructs the immutable template. The parser is the explicitly selected built-in protocol, version `1.0.0`, bound
to the normalizer source digest (and the text-adapter source for text protocols).
The definition's source and input-confirmation digests bind the canonical command,
kind, minimum count and parser. Those deterministic hashes are provenance bindings, not
authentication or execution authority. The control token authenticates definition
selection; the subsequent full preview approval authorizes one execution.
There are no caller-selected parser version/digest, artifact, baseline or
failure-attribution fields. `process_exit_v1` is not a test protocol and refuses. The template declares a focused test check and repeatable temporary setup.

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

## Result protocols and scope

The default protocol still requires the kernel's strict JSON envelope on stdout.
It never guesses a parser from output. Ordinary test prose under that default is
`malformed`, even with exit zero. For example:

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

For an ordinary project runner, select a text protocol in the same definition:

```json
{"output_format":"python_unittest_text_v1"}
```

This fragment is added to the complete definition above; it cannot execute alone.
Supply the exact sealed runner executable, argument vector and environment in
`command`. The service does not discover, rewrite or add shell commands. The
existing native launcher/executable trust and offline sandbox still apply.

- `rust_libtest_text_v1` reads default verbose libtest stdout, including multiple
  complete blocks such as a Cargo test invocation. Each block needs its declared
  running count, unique per-test events and a matching final summary. Failed names
  have a one-based block prefix (`libtest[1]::name`) to distinguish binary-local
  names. Ignored tests are skipped. Benchmarks, filtered tests, terse/JSON output,
  custom ignored reasons and arbitrary text outside test/failure blocks refuse.
  Before tests start, one complete Cargo compilation failure on stderr is also
  admitted: empty stdout, exit 101, a separate rustc error diagnostic and a final
  `error: could not compile` summary naming a supported target and positive error
  count. Optional emitted-warning counts must agree with their singular/plural
  form. The result is `compile_failed` with zero executed tests and no failed test
  names. Mixed test/compiler output, multiple compiler summaries, trailing job
  notices, incomplete diagnostics and other Cargo failures remain malformed.
- `python_unittest_text_v1` reads one standard `python -m unittest -v` stderr
  transcript. Unique per-test events, failure/error headings, executed count and
  terminal summary must agree. Skips are counted; errors are unsuccessful tests.
  Quiet output, subtests, expected failures, unexpected successes, custom multiline
  test descriptions and multiple runner summaries refuse. Stdout is preserved in
  the command response but supplies no Python result counts.

Test transcripts must be bounded ASCII with complete final lines; compiler
diagnostics may contain UTF-8 source text. ANSI escapes, CR, unsupported
formats and incomplete or contradictory summaries are malformed. A pass needs
exit zero and the minimum executed passing tests. A nonzero exit contradicting a
passing summary is malformed. Compiler prose outside the narrow Cargo form above
remains malformed. Test durations are not extracted from these
protocols; the receipt retains the command's measured elapsed time. Skipped-only
and zero/insufficient-test results never pass. All raw channels remain digest-bound.

The kernel checks the command receipt and captured output digests,
termination, output bounds, envelope consistency and minimum test count. Assertion
failure, compilation failure, malformed output, skipped-only output and too few
tests remain distinct. A zero command exit cannot replace a passing parsed result.
The result includes `validation.receipt` and `validation.validation_passed`.
The terminal exits unsuccessfully for a failed validation even when the command
itself exited successfully.

`verification_scope: approved-command-tests` describes text results; the default
JSON protocol preserves `approved-command-envelope`. Its scope
digest binds the held worktree root and creation receipt, not a source-content
snapshot. The approved runner remains trusted to describe its own tests;
normalization is not independent task grading. Project test code can print forged
runner output; these adapters do not make such code a grading oracle. No full-suite coverage, current
repository correctness, failure attribution or coding-model score is established.
`current_workspace_verified` and `test_suite_verified` remain false. Command output
and failed names are untrusted local data; the terminal escapes them. Output is
classified restricted, without a new secret-scanning claim.

## Bounded, separately approved reruns

The first admitted execution fixes one immutable definition for the created
worktree in this process, with at most three admitted executions. Each attempt requires a new preview and exact
consent. After an assertion or admitted compilation failure, prepare the unchanged definition with the
exact last receipt digest. The existing kernel rerun planner binds the prior
failure, unchanged command and new approval. A pass, unknown prior receipt,
changed definition, exhausted budget or unrepeatable result refuses selection.
Compilation failure binds the complete prior receipt and both output digests,
without inventing a failed-name set. It can select the same separately approved
rerun planner, including for the strict JSON envelope's `compile_failed` status.
Neither a compilation failure nor a rerun plan supplies execution authority.

Denial and expiry spend no execution attempt. Dispatch spends an attempt before
authority execution and clears the last result. Uncertain dispatch, corrupted
output or lost in-process completion leaves the series blocked, with no retry
authorization. Canonical authority receipts remain inspectable. A failed reply
must not be automatically resent.

The shared [command cancellation](coding-commands.md#sandbox-and-results) also
applies to validations. After dispatch admission, cancellation before native launch records a cancelled
attempt without calling the executor; cancellation during execution waits for
verified descendant cleanup. Dispatch still spends an attempt, no cancelled
receipt passes, and it cannot select failed-test repair or rerun eligibility.
Unknown cleanup leaves canonical uncertainty and no fabricated command result.
Cancellation does not release the API lease or restore a consumed approval.

The budget and normalized last receipt are process-only. A new successfully
created worktree starts a new series. Restart loses ownership, previews, results
and budgets; it cannot restore command authority from receipts. These limits
bound this manual validation series, not a durable coding run or the separate
ordinary-command API. Source changes between attempts are not classified as
flakiness. The [owned-edit binding](coding-owned-edits.md) preserves the immutable definition,
spent budget and prior result while invalidating pending test approvals. There is
no automatic model feedback/repair, connected coordinator retry, durable run
recovery, commit, push or independent grade. The separately consented
[repair proposal path](coding-model-repair.md) now discloses bounded excerpts of
the latest retryable failure to the configured model, once per failed receipt.
It prepares an inert diff; writes and test reruns retain their separate exact
approvals. Failure excerpts and the spent-request marker are process-only, and
successful results retain no excerpts.
