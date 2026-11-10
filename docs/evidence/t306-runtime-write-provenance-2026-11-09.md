# T-306: Original approval provenance for native runtime writes

Scope: T-303/T-306, core, non-closing increment atop immutable commit
`9d6d453aa97195ec9a3d5d6b49cbb9d7079afa73`, under independent review when
this work begins. Both parents remain open; no row or sub-row is added or closed.

During preparation, the [base independent core review](../reviews/9d6d453aa971-local-coding-write-provenance.txt)
returned **Verdict: PASS** for the exact base commit with no high or medium
finding. The report is retained byte-for-byte. Its low L2 poisoned-owner cleanup
finding is addressed by explicitly retiring the old proof during recovery, with
an assertion in the existing poisoned-owner regression. The new two-file runtime
projection covers L3's first gap. Low L1 precheck ordering, L3's manual postcommit
refusal coverage and older lows remain same-row backlog. This review closes no row.

The kernel's existing opaque write authorization now borrows the original
approval after preapply validation, one-use consumption and the mandatory
checkpoint. The runtime writer captures that approval and its shadow change set
from the authorization before native dispatch. The same live owner can retain
the private write proof after the existing terminal publication callback and
successful completion checks. No caller receipt-adoption API is introduced.

Completion correlates the original approval, issued grant digest and revision,
actual consumed grant, transaction identity and complete checked application
chain. Runtime transaction identifiers need not follow the prepared-edit naming
convention. Original source/root checks still apply before review projection.
Previous proof is retired before dispatch. Unpublished writes, altered results
and incomplete first-preimage/current-postimage coverage retain no new proof.
Sequential retries on one file and edits on different files cannot invent an
aggregate approval. Complete cumulative history and new-file review remain open.

Four new Linux regressions use real native writes, grants and receipt chains
with synthetic creation provenance and a test-controlled terminal-publication
choice. They exercise complete two-file receipt projection, fresh source drift,
unpublished writes, modified consumption records and sequential transactions.
A new kernel regression uses a synthetic memory driver to prove that the
authorization borrows the original approval after the consumed checkpoint.
These are component observations, not actual terminal publication, native Git,
model execution, independent review or task grades.

The preliminary focused run passes all four Linux methods (420 filtered) and
all eleven kernel write-transaction methods (1,565 filtered), with no failure or
ignore. The commands take 1,144.83 and 20.51 seconds including reservations;
the first waits approximately nineteen minutes for the shared build slot. Test
bodies take 1.11 and 0.04 seconds. The two-file native write/projection component
measures 0.540118 seconds, including fixture/store setup and source-drift checks.
This is not a model, command-test, task or coding-score measurement. The explicit
poison cleanup and its assertion are added after this preliminary run and verified
by the complete final gate below.

The complete affected fast gate remains **FAIL**: **2,925 passed, 72 failed,
72 ignored**, zero measured or filtered, across **46 unfiltered targets**.
The preceding 26 targets match the baseline's exact failure/ignore identities
and counts, with precisely four new Linux passes. The additional twenty kernel
targets pass 1,635 tests and retain eight existing ignores. All five new methods
pass. No failure or ignore is added; no original assertion is removed. The
existing poisoned-owner regression also passes with its new proof-retirement
assertion.

Formatting and Clippy with warnings denied pass. The complete gate takes
457.35 seconds including reservations: formatting 2.45 seconds, Clippy 27.40
seconds and compilation/tests 425.67 seconds. Its raw log SHA-256 is
`9ae43ea2214990acdfef0d62a8f53a068cbc714ce98d4cd301ebb0de70d6f57b`.
All six Rust hashes remain fixed throughout final verification; no source edit
follows the complete gate. Exact per-target comparisons and raw command/source
records are retained privately for independent reproduction. Comparison success
does not pass the full gate or accept either parent.

Final light checks cover changed prose, exact retained review bytes, unchanged
ledger rows, original assertions and added-line hygiene. The full gate's source
freeze remains current. Independent core review of this increment is required.

Reviewer output and artifact provenance, candidate execution, approved Git,
native/model measurement and durable recovery remain open. No route, terminal
command, approval bypass, model request or trust exception is added. No frozen,
hidden or held-out task, scorer, threshold, prompt, dependency, supply-chain,
status or scoreboard change is involved. No model download, GPU process, long
acceptance gate, public release or owner-ready claim is involved.
