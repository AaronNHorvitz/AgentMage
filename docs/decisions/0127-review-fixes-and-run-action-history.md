# Decision 0127: Review Fixes for Batch 13 and Run Action History Through the Host and CLI

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088, 0116, 0117, 0120, 0122, 0124 and 0126; current owner restart |
| Scope | Findings F1 to F5 and notes N2 and N4 of the independent review of `7c593b3b`; AMR-05.9.1 |

## Findings

An independent read-only review of `8469649d..7c593b3b` passed with five low
findings and seven notes.

- F1: replay refused a repeated evidence digest, but no test failed when that
  rule alone was weakened.
- F2: the adjacent-edits case of the language server codec asserted only that an
  observation was returned, so a rejection passed it.
- F3: a recipe manifest with a repeated member was admitted with the last value,
  because the bytes were parsed into a generic value first.
- F4: the batch 13 record stated that the Sprint 78 test filter holds sixteen
  tests; it holds eight.
- F5: an empty published diagnostics list was sealed complete with no items,
  although Decision 0126 says an empty answer is never a complete proof of
  absence.
- N2: the duplicate-path case of the documentation pack manifest also broke the
  byte total, so it did not isolate the path rule.
- N4: a selector test name still said "fifty-one rows".

## Decision

F1. The replay table gains a case whose only change is a repeated evidence
digest.

F2. The adjacent-edits case asserts a complete observation with both edits and
no terminal code.

F3. A recipe manifest is parsed directly into its closed structure, which refuses
a repeated member, and must still serialize back to exactly the bytes' value. The
closed-parsing test gains a repeated member, a repeated member beside a tag
without fields, and a repeated tag.

F4. The batch 13 record specification says eight, and the record is regenerated
in the batch 14 evidence pass. The disposition stays: the Sprint 78 report is
historical because its producer fixes a count of five.

F5. An empty published diagnostics list is sealed unavailable with the empty
result code, like every other empty answer. A server may publish an empty list
before or instead of analysing, so it is not treated as "no problems". The
contract's carve-out for diagnostics still permits a complete empty observation.
The codec no longer produces one.

N2. The manifest table gains a repeated path with a consistent total. N4. The
test is renamed so it no longer carries a count.

### AMR-05.9.1: an action history for each authorized effect of a coding run

Owners. Each effect owner keeps its own action history (Decision 0124) for one
run, in memory, and declares it with the run's other declarations (Decision
0116):

- the coding tool boundary keeps one entry for each call whose grant it consumed
  (tool calls, file writes and commands) and one for each call a person refused
  or narrowed;
- the host's live runtime service keeps one entry for each job control request
  the durable job ledger decided, applied or refused. A retry that the ledger
  answers with its first decision adds nothing.

The two chains are not merged: each replays alone to its owner's head.

Tool boundary entries:

- Kind: patch, selected, inverse and created writes are file writes, commands
  and validations are command runs, and reads, Git inspections and change
  history reads are tool calls.
- Identity: the runtime operation identity.
- Authorization: for a consumed grant, the grant's identity and the canonical
  digest of the exact kernel grant as issued, for every write kind as well as generic
  calls. The runtime's evaluation carries one decision digest that is either
  the person's approval response or the session preauthorization, and it
  does not say which. The boundary knows which path issued the grant, so the
  reason code says it (`coding.approved.<ended>` or
  `coding.preauthorized.<ended>`), and the decision digest is kept as evidence.
  The runtime contract does not change. For a refusal, the authorization is the
  person's decision: the digest of the person's response.
- Effect digest: the canonical digest of the session, task, run, operation and
  the canonical digest of the exact call.
- Outcome: succeeded, failed (also timed out), cancelled, denied and uncertain
  follow the result. A call that failed after its grant was consumed is
  uncertain, with the reason `unknown`. A refusal is denied with its refusal
  code, and a narrowing is denied with `runtime.coding.narrowed-to-selection`.
  The derived write has its own entry when it runs.
- Evidence: the sorted, unique digests of the decision, the displayed preview,
  the runtime authority binding and, when the call returned, its receipt. A
  refusal keeps the preview digest.
- Time: the trusted time at which the grant was issued or the refusal decided,
  taken from the coordinator's clock. The boundary keeps no clock of its own.
  Entries are kept for thirty days.

Job control entries: the kind is job control and the identity is the client's
request identity. The authorization is the person's decision: the digest of the
request under the client scope the host derived from the authenticated peer. The
effect digest covers the job, the action and the observed revision. The outcome
is succeeded when applied and denied when refused, and the reason is
`job.control.<action>.applied` or the refusal. The evidence is the ledger head
after the decision. The time is the host clock at the decision.

Completeness. An entry that cannot be kept makes that history incomplete, and
an incomplete history is never declared. This covers a full chain, a malformed
entry, a time that went backwards, a draft that could not be built and a
decision whose resulting ledger head could not be read. A run resumed from an
event cursor after a host restart declares neither history: its owners hold
only what happened since (as Decision 0117 does for recoverability). A run
resumed in the same host (Decision 0122) was composed again, so its tool
boundary history is unavailable. The service decided every one of its control
requests, so it carries the job control history into the resumed session. A
run without a job declares no job control history. An expired place keeps no
time (Decision 0125), so a history replayed after its last entries expired
knows only the last kept entry's time. Run histories are not replayed by their
owners and nothing expires within a run, so this matters only to AMR-05.9.6.

Transport. Run declarations schema 2 adds `effect_history` and
`job_control_history`. Each is optional and holds the retained positions and
the head. The action history types gain closed deserialization. The variant
without authorization becomes a variant with no fields, so a member beside its
tag is refused. Its encoding is unchanged, so no entry digest changes. The IPC
wire version becomes 9.

CLI. The driver keeps declarations only for the exact run at schema 2. It drops
as unavailable a history that does not replay to its head, or that holds a kind
its owner does not keep. After each run, both histories are shown on standard
error: in text, one line per entry; in JSON, inside the run declarations row.
`--action-history-export CHAIN:FROM:TO`, where the chain is `effects` or
`job-control`, builds the redacted, rescanned export of that range from the
replayed history. It prints the export on standard output before the outcome
line. In JSON this is a row with the exact document text and its digest; in text
it is a header line followed by the document line. A range the history does not
hold, or an unavailable history, prints a content-free notice on standard error
instead. Nothing is written to a file, and the host keeps no export.

Limits. The histories live for one run in one host process. Persistence across
host restarts, with viewing and export of an ended run, is a new open row,
AMR-05.9.6. Model routes, network requests, memory and extension changes enter
histories with AMR-05.9.2 and AMR-05.9.4. A native actual-process proof
remains AMR-05.10. The row closes only as a component, host-unit, wire and CLI
row, with independent review open. The tool boundary's recording glue runs only
in native boundary tests. Those tests cannot run in the builder's sandbox, like
their siblings, so they are recorded as unverified there.

## Consequences

- `kernel/engine/src/action_history.rs`: closed deserialization;
  `Unauthorized {}`; a closed JSON round-trip test.
- `shells/host/src/coding_action_history.rs` (new): recorder, entry mapping,
  chain verification, export selection, view and export rendering, with tests.
- `linux_coding_runtime.rs` records consumed grants and refusals.
  `coding_live_runtime.rs` records decided control requests and declares both
  histories. `runtime_transport.rs` gains schema 2, `runtime_ipc.rs` wire 9,
  and `cli_runtime.rs` verifies each history. `coding_development_client.rs`
  shows and exports the histories, and `cli.rs` gains the option.
- TASKS.md: AMR-05.9.1 is recorded with its evidence, and AMR-05.9.6 is added as
  open.
