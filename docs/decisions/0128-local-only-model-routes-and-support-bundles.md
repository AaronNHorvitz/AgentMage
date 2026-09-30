# Decision 0128: Review Fixes for Batch 14, Local-Only Model Routes and Support Bundles Through the Development CLI

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088, 0116, 0124, 0126 and 0127; current owner restart |
| Scope | Findings F1 to F3 and notes N1 to N3 of the independent review of `e5f34910`; AMR-05.9.2 and AMR-05.9.3; new row AMR-05.9.7 |

## Findings

An independent read-only review of `7c593b3b..e5f34910` passed with three low
findings and seven notes.

- F1: two completeness rules of the job control history had no test that
  failed when either rule alone was removed. One rule is that a run resumed
  after a host restart declares no history. The other is that a decision whose
  resulting ledger head was not read leaves the history incomplete.
- F2: no test anywhere asserted the preauthorized reason code or the entry a
  narrowing records.
- F3: the order in which the CLI writes an export and the outcome was untested.
  The test named for it only checked rendering.
- N1: a match arm after the narrowing evaluation could not be reached.
- N2: Decision 0127 calls the effect digest canonical. It is the SHA-256 of
  compact `serde_json` output with sorted keys, which is canonical only because
  the workspace builds `serde_json` without `preserve_order`.
- N3: the local testing guide quoted the range notice as `reason: range`.

## Decision

F1. A live runtime test decides a real control request through a durable
ledger and records it. It then tests each rule on its own. The same entries
are not declared once the run is marked as resumed after a restart. A later
decision recorded without a readable head leaves the history undeclared.

F2. The native preauthorized command test asserts the run's one entry. The entry
is a command run under the fresh grant, with the reason
`coding.preauthorized.succeeded`, the preauthorization and receipt as evidence,
and the issuance time. The native narrowed patch test asserts two entries. The
first refuses the original patch under the person's decision, with the reason
`runtime.coding.narrowed-to-selection`. The second is the derived write under its
own grant, with the reason `coding.approved.succeeded`. Both tests are native
boundary tests like their siblings, so the builder's sandbox records them as
unverified.

F3. The development CLI renders each run's result first. It then writes the
result through one function over a writer pair: verified artifact rows, a
requested export, the export's notice on standard error, the outcome, then
progress, declarations and job state on standard error. A test captures both
streams. It asserts that the export precedes the outcome, that the outcome is
the last line on standard output, and that an unavailable export leaves only a
notice. The rendering test is renamed for what it checks.

N1. Each branch of the narrowing evaluation records its own refusal, so no arm
is left that cannot be reached. N2. This decision corrects the wording above.
The effect and job control digests are the SHA-256 of compact, key-sorted
`serde_json` output, and nothing changes in code. N3. The guide says the range
notice carries the reason `range`.

### AMR-05.9.2: local-only model routes by default

Owner. The development host's runtime factory routes each run's model requests.
It does this before it constructs the model, through the kernel router of
Decision 0124 (`route_gateway`), in local-only mode. No other mode exists in
the development host. The decision covers every model request of that
composition.

Routes. The host offers exactly one route: the selected source's exact profile.
That is the scripted fixture or a pinned development candidate. The route
identity is the profile identity. The candidate digest is the canonical digest
of the exact profile. The class is `strict_local`, because the scripted
proposals stay in process and the candidates are served over an authenticated
Unix socket on the same machine. The capability digest covers the profile's
capability records, and the qualification digest covers the admission (the
capability states and the purpose admitted). The route is not a
qualification. Its limits are the profile's context and one concurrent request.
The development host observes no model health or resource fit before a run. It
records the route as healthy, fitting and admitted at composition. A model that
then fails to start or is refused by the inference preflight fails the run as
before.

Request. The request identity is the run identity. The data classes are the
person's conversation, workspace excerpts and tool outputs. The most permissive
class allowed is `strict_local` and no wider disclosure is accepted. The request
carries no fallback policy and no hybrid grant, because the development host has
no grant owner. The policy digest covers this fixed local-only policy, and the
time is the host clock. If the router selects nothing, composition fails before
any model or tool is built. It prints `coding.development.route.<reason>` on
standard error.

Receipt. The host declares the router's receipt without changing its fields or
its digest. The kernel receipt carries its reason codes as static text and
cannot be parsed, so the host has its own closed type with the same fields and
the same encoding. A test checks that converting a kernel receipt keeps its
digest byte for byte.

History. The same owner keeps a third action history for the run. It has one
model route entry per composition. The entry is authorized by the person's
decision: the digest of the exact run request, which the person started with
this profile. The effect digest covers the run, the selected route and
candidate, the class, the mode and the data classes. Its evidence is the receipt
digest and the policy digest, and its reason is the receipt's reason.

Transport. Run declarations schema 3 adds `route_receipt` and `route_history`,
and the IPC wire version becomes 10. As for the other parts, a run resumed from
an event cursor declares neither. That includes an in-host resumption, which was
composed again.

CLI. The driver keeps a receipt only if all of the following hold:

- it names the run and its digest recomputes;
- its mode is local-only;
- it selected one route of class `strict_local`;
- it names no hybrid grant or provider;
- its data classes are the declared ones.

The driver keeps a route history only if it replays to its head and holds model
route entries only. Each entry must be authorized by this run's request digest
and carry the kept receipt's digest as evidence. Anything else is dropped as
unavailable. After each run the receipt and the route history are shown with
the other declarations, and `--action-history-export routes:FROM:TO` exports a
range of the route history.

Limits. Hybrid mode needs a grant owner, and the development host has none. That
covers remote routes under their own grants, expiry and budget counters taken
from the grant's owner, and granting and revoking through the CLI. This work
moves to the new open row AMR-05.9.7, and AMR-05.9.2 is reworded to local-only
routing by default with its receipt. Every remote route stays unavailable,
because none is offered. A host test offers a remote route with a valid grant
and shows that local-only mode refuses it. It also shows that a remote route
alone leaves nothing selected. The composition glue runs only where native
composition runs, like the other factory paths. A native actual-process proof
remains AMR-05.10.

### AMR-05.9.3: support bundles through the development CLI

Owner. The development CLI owns the support bundle for its own invocation.
It builds the bundle from what it observed and previews and publishes it through
the existing one-use diagnostic export workflow (`DiagnosticExportWorkflow`).
The host is not asked to write anything.

Request. `--support-bundle DIRECTORY` names an existing absolute directory. The
export workflow requires it to be private, owned by the person, not linked and
not synchronized. After the invocation ends, the CLI previews a new file in that
directory named by the preview identity. The invocation may have ended with an
outcome, a runtime failure or a startup failure. When cancellation was
requested, the CLI prints a content-free skip line instead and asks nothing.

Contents. The bundle is the doctor report, component versions and evidence
digests of Decision 0124, and holds no content, path, prompt or host identity.
The CLI observes these doctor components; every other component is reported
missing (unavailable):

| Component | Observation |
| --- | --- |
| Package | degraded, `diagnostic.package.development-build`: a development CLI, not a released package |
| Platform | healthy, `diagnostic.platform.linux-development` |
| Model | degraded, `diagnostic.model.scripted-fixture` or `diagnostic.model.development-candidate`, identified by the profile digest |
| Runtime | healthy, `diagnostic.runtime.host-served` after every run was served; blocked with the CLI's content-free failure code otherwise |
| OfflineBoundary | healthy, `diagnostic.offline.local-only-route`, identified by a verified local-only receipt; unavailable, `diagnostic.offline.route-unobserved`, without one |
| ReceiptChain | healthy, `diagnostic.receipt-chain.replayed` when every chain of the last run was declared and replayed, identified by a digest of their heads; degraded, `diagnostic.receipt-chain.incomplete`, when some were not; unavailable before any run |
| Recovery | healthy, `diagnostic.recovery.job-ended` for an ended job, identified by its head; degraded, `diagnostic.recovery.job-open` otherwise; unavailable, `diagnostic.recovery.job-unobserved`, without a job state |

The versions name the CLI, the run declarations schema and the IPC wire
version. The evidence names these digests from every run: the run request, the
recoverability report, the receipt, the three history heads and the job ledger
head.

Approval. The CLI shows the preview on standard error. The preview has the
destination digest, payload digest, byte count, field families, redactions,
sensitivity, retention, expiry and confirmation digest. It then asks the person
to type `yes`. Only that exact answer publishes the preview once and prints the
receipt. Any other answer, end of input or expiry cancels the preview and
writes nothing. `--approve-this-run` does not cover the bundle. A refused
destination prints the workflow's content-free code. Nothing is uploaded: the
CLI has no network client.

Output. The preview, prompt, receipt, refusal and skip lines go to standard
error. Standard output keeps its contract that each run's outcome is its last
line. Text lines and JSON rows are both supported. The bundle never changes the
invocation's exit code.

Limits. The bundle describes one CLI invocation. It is not a host doctor: the
host's native prerequisites are reported missing rather than guessed. The
support bundle of a running host through production transport is part of the
AMR-05.9.4 transport decision. A native actual-process proof remains AMR-05.10.

## Consequences

- `coding_live_runtime.rs`: the F1 test, route declarations taken from the
  factory, and schema 3 declarations. `linux_coding_runtime.rs`: the F2
  assertions and N1.
- `coding_route.rs` (new): the development route, request, policy, host receipt
  type, route history entry, verification and rendering, with tests.
  `coding_development_runtime.rs` routes at composition, and
  `native_chat_runtime.rs` gains `take_route_declaration`.
- `coding_support_bundle.rs` (new): the observations, the bundle, preview,
  approval and refusal through the export workflow, and rendering, with tests.
  `cli.rs` gains `--support-bundle`, and `coding_development_client.rs` runs it
  and writes each run's result through `write_run_result` (F3).
- `runtime_transport.rs` moves to schema 3, `runtime_ipc.rs` to wire 10, and
  `cli_runtime.rs` verifies the receipt and route history.
  `coding_action_history.rs` gains the route chain.
- `scripts/coding_harness.py` passes `--support-bundle` through.
- TASKS.md: AMR-05.9.2 and AMR-05.9.3 are recorded with their evidence, and
  AMR-05.9.7 is added as open.
