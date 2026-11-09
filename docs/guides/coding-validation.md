# Approved project tests and exact reruns

The local coding owner can normalize a separately approved command's selected
test-output protocol and expose failure names. It uses the existing kernel validation
templates, normalizer and rerun planner, and the same native executor and
canonical authority as [ordinary commands](coding-commands.md). T-303 and T-306
remain open. Positive native execution remains unmeasured in this lane under the
unchanged executable-trust refusal. Component outcomes use synthetic executors;
terminal approval tests use a synthetic HTTP peer.

## Definition and approval

The host library also offers `CodingEndpointFixedTestCheck` and
`CodingEndpointFixedTestVerifier` for an explicit, limited acceptance criterion.
Create the check from the same `CodingEndpointValidation` supplied to the combined
owner. Show the complete selected command (executable and arguments), test kind,
parser and minimum test count alongside `check.criterion()` before approval.
Render these values as escaped data; the criterion digest alone is not a
readable command preview. Use the criterion verbatim as the only entry in both
the task's acceptance criteria and the work packet's acceptance checks before
sealing the request. Use the combined tool catalog, controlled write mode, execution authority class and `ToolOutput` evidence requirement.
After startup has narrowed and resealed the request, construct the verifier with
`for_request(previews.startup().request(), check)`, then connect it using
`previews.completion_verifier(verifier)` before moving those previews into the
turn port. Construction reads no file and grants no authority.

The combined `serve-endpoint-coding-development` command can explicitly select
this checker with `"completion_check": "fixed_test_run_v1"` in its private launch
file. Generate the exact criterion as above and put it in both acceptance lists
before sealing the launch request. Startup preserves those lists and the objective;
it binds the checker to the final instruction-narrowed request with fresh run and
session identities. Unsupported definitions or broader criteria refuse before
encrypted state, native inspection or credentials are opened. Omit the field to
keep non-certifying development behavior. Null, unknown choices and the field on
the edit-only command refuse. No HTTP request or model output can opt in.

Embedding programs can use
`serve_coding_endpoint_edits_and_checked_tests` with a caller-selected
`CodingEndpointAcceptancePort`; it attaches the checker to the same original
preview owner before the turn port takes ownership. The existing generic
`serve_coding_endpoint_edits_and_tests` keeps its caller-supplied verifier.
All existing launch inspection consent, separate model disclosure consent and
exact creation/edit/test approvals remain required.

This checker supports the two built-in per-test text protocols. It refuses the
self-reported JSON protocol, non-test definitions, a zero minimum, other working
directories, additional criteria, changed requests and mismatched postconditions.
It requires the original linked history to end in a complete pass of the exact
command, kind, parser and minimum count. Every original attempt, including every
failed one, must correspond to an edit/test pair in the coordinator's records.
Exact command and normalized validation receipts, authority identities, output
schema and digest must match. Tool results and receipt identities retain their
original edit/test order. Aggregate evidence is correlated by unique identity
and complete reference equality, independent of its order: the coordinator
sorts evidence by ID, putting native command IDs before native edit IDs.
Missing, extra, duplicated or altered references refuse. Each edit must have a
successful changed result from the original native tool family; the final test
must be the last tool result. A contradictory no-op or unrun-check
claim fails. Failed candidates carry no passing evidence. The existing verifier
registry remains responsible for accepting the resulting candidate.

Use this checker only when that declared test-run criterion is the actual
acceptance check. It does not certify test coverage, test integrity or arbitrary
prose objectives. Project tests and their output remain untrusted input; printed
test results cannot independently establish functional correctness. The original
owner's source inspections still bracket dispatch and remain non-atomic. This
checker performs no file reads itself and receives no new execution authority.
The embedding host must supply the original coordinator observations, not
caller-invented transcripts. Synthetic checker tests establish this component's
record checks; positive original-owner dispatch and native/model execution
remain unobserved under the existing host-trust blocker. Neither parent row nor
any evaluation score is qualified by this library addition.

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
attempt, budget, sealed input mode and bounds, and expiry. Models and ordinary client tokens have no authority
over these routes. `approve_definition: true` alone cannot execute anything.

| Route | Request/result |
| --- | --- |
| `GET /v1/code/repository/validation` | No body; process-only preview, last receipt, bounded validation and command histories, rerun plans, linked-pass check, optional whole-worktree observation binding, attempt count and limits |
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

The validation result and inspection include `linked_pass_verified`. It is true
only when every admitted attempt completed with retained original command and
validation receipts, each earlier eligible failure has its exact immediate
rerun plan, all attempts have separate approvals and identities, and the final
result is a complete pass. Inspection keeps the original receipts in `history`
and the original edges in `rerun_plans`; earlier failures are never rewritten.
There are at most three attempts. A missing or uncertain successor leaves this
check false while retaining the completed prefix. Denied previews add no edge.

This check describes only the recorded command series. It does not verify
current source, independent task acceptance or suite coverage. The coding
completion verifier continues refusing failed tool history; acceptance of a
linked repair and a current source snapshot remains separate work.

The optional `workspace_binding` ties the final linked pass to bounded local
whole-worktree observations. Observation begins only inside the original kernel
command permit, immediately around the native executor; ordinary
commands do not scan. Every regular file is content-hashed, and every directory,
including empty directories, contributes identity and metadata. Hidden files,
tests, configuration and generated files are included. A local Git pointer is
hashed as a file; its target is never followed. Symbolic/hard links, special
objects, unsafe names, inaccessible files and exhausted limits make the binding
unavailable; no partial tree counts. Failure of this observation wrapper alone
does not change the command result. Native validation separately requires the
complete sealed capture described below; its capture failure refuses launch.

Each capture requires two identical complete scans with a shared cooperative
two-second deadline. Ceilings are 4,096 entries, 256 directories including the
root, directory depth 16, 16 MiB per file and 64 MiB of file bytes per scan.
Each directory admits at most 256 KiB of UTF-8 names. These local hashes and
metadata stay process-only; the API returns only digests and counts. File and
directory identities, modes, sizes, modification times and change times are
bound. Restoring old bytes is detectable when the filesystem advances the
recorded change times. Coarse timestamps can miss a write-and-restore within
one tick; the observation remains sampled and non-atomic. The capture immediately
before execution must match the capture after it. The
binding also identifies the exact creation receipt and final validation receipt.
Inspection rechecks the current held root and the complete tree.

`workspace_binding.matches_current_observation` is an observation result, not an
atomic filesystem snapshot: concurrent writers can race any traversal, and this
is not a security boundary against a privileged filesystem adversary. False or
null must never establish current-source acceptance. `atomic_snapshot` and
`independent_task_grade` are explicitly false. The existing
`current_workspace_verified` and `test_suite_verified` remain false even with a
matching observation, and the completion/development verifiers stay unchanged.
The binding also includes the final command receipt digest and its optional
`sealed_workspace_input`. `sealed_input_matches_observation` checks whether
that record's root, capture observation and counts match the bound observation.
This flag does not recheck current files; `matches_current_observation` supplies
that separate sampled comparison. Either flag alone, or both together, cannot
establish task acceptance. Reviewed acceptance integration remains open.
Observations add bounded local work outside the runner's elapsed-time receipt;
blocked filesystem calls can delay cancellation/deadlines. No raw file content
is newly disclosed to a model or client, no command authority is created, and no
receipt is rewritten. Denial/expiry/cancellation before launch produce no binding;
an uncertain successor clears any binding while retaining the completed history.

The budget, bounded history, rerun plans and normalized last receipt are process-only. A new successfully
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

## Sealed native validation inputs

Native validation now selects a complete sealed copy instead of mounting the
live worktree. Its exact approval includes `workspace_input` with mode
`sealed-read-only-tree-v1` and the capture bounds. The original kernel permit,
command and trusted native supervisor remain required. Direct and retained
endpoint validation select the same mode; ordinary commands keep their existing
mount behavior.

Every regular file is copied through its held descriptor into a sealed memory
file. A second complete observation must match the first; nothing is silently
excluded. The closed command owner now admits exactly the sealed capture count,
up to 128 files (including zero for an empty tree). Ordinary commands keep their
existing limit of at most one source descriptor. The supervisor is responsible
for retaining admitted descriptors until cleanup. Its sandbox arguments construct a private directory tree, mounts copied file contents
read-only, restores ordinary permission bits and remounts all directories
read-only. The live worktree root is never passed to the validation child.
Edits to the host files after capture cannot change the sealed bytes.

The additional file-descriptor bound is 128 regular files. Other limits remain
256 directories including root, 4,096 entries, depth 16, 16 MiB per file and
64 MiB total file bytes, with a cooperative capture deadline of two seconds or
the remaining command deadline, whichever comes first. Links, unsupported names,
special files or special permission bits, races, unavailable cancellation and
exhausted limits refuse without launch or a mutable fallback. Blocking filesystem
calls can delay cancellation or deadlines. A consumed validation attempt remains
spent when capture fails.

The sandbox copy preserves file bytes, directory names and ordinary modes;
inodes, timestamps and ownership differ. Sealed contents are immutable, while
capture is not an atomic snapshot of the source filesystem. The original command
receipt now includes optional `sealed_workspace_input`: version one, held-root
and capture-observation digests, projection digest, file and directory counts
(including root), and total file bytes. The projection digest binds the mode,
root permissions, ordered directory paths and permissions, and ordered file
paths, permissions and exact copied preimages. It describes the descriptors
handed to the original supervisor, not bytes reread after execution. It covers
source inputs only, not the entire toolchain or runtime filesystem. It does not
prove the child entered its command body; the original termination and cleanup
evidence remain necessary.

The kernel seals this record into the original command receipt, which the
validation receipt already references. `command_history` exposes those original
receipts alongside `history`, in the same bounded order, retaining failed inputs
unchanged. The immediate validation result also exposes the sealed record.
Missing legacy records remain absent and cannot establish a sealed-input match.
The optional receipt extension preserves absent-field canonical bytes; older
external readers retaining the old closed schema require an update. The
published command-receipt schema and current host/native-tool digests include
this optional closed record and enforce its version, fields and bounds. Digests are integrity
bindings, not authentication for caller-supplied receipts or task grades.

The existing workspace observation remains a sampled comparison, both completion verifiers remain unchanged, and
`current_workspace_verified` and `test_suite_verified` remain false. Native/model
workflow evidence and reviewed acceptance integration are still open.

The [sealed-validation review](../reviews/dc5cb4532d97-local-coding-sealed-validation.txt)
found that earlier supervisor admission rejected every capture of two or more
files before launch. The [same-row admission repair](../evidence/t303-sealed-admission-repair-2026-11-09.md)
corrects that count check. Native sealed launch, read-only behavior and descriptor
cleanup still require a supported host run; component tests are not that evidence.
