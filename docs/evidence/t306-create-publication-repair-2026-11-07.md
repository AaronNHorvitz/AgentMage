# T-306: Refuse owned creation without terminal publication, 2026-11-07

Builder repair based on `b6d2829368e7e01716f4fc78c84e1154a62ed3f8`.
The [independent core review](../reviews/b6d2829368e7-local-coding-created-file-transfer.txt)
has **Verdict: FINDINGS**. Medium M1 identifies an owned creation path through
the hookless coordinator that could establish lineage without a tool-terminal
publication. T-306 and shared T-303 remain open pending independent re-review;
this record does not resolve that finding by self-approval.

Require terminal context at the native creation entry point whenever the live
worktree owner is attached. The refusal precedes destructuring the issued
operation, resolving targets, consuming its grant, or entering native effects.
The guarded closure encloses the entire existing creation implementation. The
unowned path retains its previous behavior. Admission alone still supplies no
grant or completion: the existing implementation must reconcile native results,
finish the specialized effect event and only then call driver completion.
Owned structured writes retain their existing equivalent refusal.

The new component tests invoke the actual dispatch guard without constructing a
fake native owner or bypassing the executable trust checks. Without context,
the dispatch closure is never entered; synthetic grant-consumption and
available-path sentinels remain unchanged. Other combinations enter exactly
once and propagate an uncertain result unchanged. The sentinels are not actual
native grants or transfer lineage. This component proof and source ordering do
not establish a successful owner-bound native coordinator run; the existing
host fixture still fails at sandbox executable trust before reaching that path.

Review low L1, creation wording in terminal confirmation, is recorded as backlog
in the existing row. The report is retained byte-for-byte. No new row or
sub-row, permission grant, model admission, hidden input, test threshold,
dependency or release authority is introduced. The existing source-selection,
lifecycle and snapshot-consistency lows remain backlog.

## Verification

The final shared source gate also covers the
[endpoint usage increment](t303-provider-usage-2026-11-07.md). One verified build
reservation ran all focused targets and the unchanged complete gate, using two
build jobs and four test threads. All nine Rust and two Python source hashes
remain unchanged afterward.

The actual guard components pass: **2 passed, 0 failed/ignored, 601 filtered**.
They also pass in the unfiltered host target. The complete run has **26 targets,
878 passed, 53 failed, 19 pre-existing ignored and 0 filtered**. Formatting and
Clippy with warnings denied pass; the full fast gate remains **FAIL**. Its nine
commands total 101.09 seconds excluding reservation wait (formatting 0.90,
Clippy 6.76, Rust tests 91.13). All 53 host failure identities and normalized
diagnostics equal the retained base. Eleven ignores match the shared targets;
eight inference ignores are proven in four source files byte-identical to base.
No ignore was added, and no trust check or test was weakened.

Source inspection and the audit confirm that the only call into the native
creation body is inside the guarded closure, which tests actual owner presence
and terminal-context presence. The body still finishes the specialized event
before completing lineage. This wiring check complements the executed component
refusal; it is not a native creation or terminal-publication observation.

The prior review is retained unchanged, low L1 is backlog, and both ledger
statuses remain open. Final prose and policy checks are retained separately;
they cannot make the full source gate green. The earlier usage-only run is
historical and does not verify this repair. No real-model, native Git workflow
or coding score is claimed. Exact-commit independent core re-review remains
required.
