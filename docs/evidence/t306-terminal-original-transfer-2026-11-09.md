# T-303/T-306: Terminal continuation to the original checkout

Date: 2026-11-09. Base:
`24633619dec4f011d81846eaac0eece42ddd7595`.
Same-row core increment; neither parent nor any sub-row closes.

`endpoint-original-apply` connects the existing terminal-record export, terminal
release and original-apply routes. It verifies the exact retained controlled
request and complete terminal event chain, then flushes the record to stdout
before offering release. The person separately confirms release, reads of both
held roots and the complete exact original diff. HTTP operations and terminal
confirmations retain one allowance and interruption latch. No model request,
new tool, command, grant policy, retry or automatic cancellation is added.
The command starts no model, Git or shell process.

Integration exposed an existing client mismatch: terminal selection and record
verification accepted only `ephemeral_read_only`, while native coding startup
uses `controlled_write`. Both now accept these existing process-held modes.
Durable mode and cursors still refuse. The new transfer command requires the
controlled mode. The original coordinator, disclosure confirmation, exact tool
approval, event chain and native ownership checks are unchanged. Offline replay
displays the controlled record without restoring execution or certifying its
authenticity, referenced payloads or task success.

## Verification

The new tests run the actual terminal executable through a controlling terminal
and the real strict/combined review HTTP services. Creation provenance is
synthetic. Both retain an actual prior native edit through the same owner, then
cancel the coding run before model dispatch. The command exports that cancelled
run's actual terminal record, performs fresh release/read/diff confirmations and
transfers only the earlier successful lineage. A fresh offline process replays
the controlled record. This claims no model edit, native worktree creation,
test execution or completion-checker dispatch inside the cancelled run.

Separate cases decline each of the three decisions, interrupt at apply, exhaust
the allowance at read consent and change original bytes before the approved
decision. They verify no later write and preservation of human data, the index
sentinel and worktree bytes. Invalid paths, allowances, foreign request bindings,
an unstarted event history and piped approvals refuse before release. Additional
terminal tests observe both separate controlled-mode tool/disclosure decisions
and continued durable-mode refusal.

A local proxy drops a successful release response or a successful apply
response from the real service. The client stops after the single affected
request, sends no later operation and emits no workflow-success summary.
Inspection distinguishes retained unchanged lineage after release from the
actually committed native write after apply. Unrelated human/index/worktree
bytes remain unchanged in both cases. A lost reply is not evidence of no effect.

The initial focused run retained four passes and one fixture failure. The proxy
forwarded its own Host authority to the API, which correctly refused with 403.
Only that fixture's forwarded Host was corrected to the actual API authority;
production Host validation and all existing tests remain unchanged. The failed
log and exact fixture source are retained. The repaired focused transfer suite
passes all five methods; the focused terminal selection/consent suite passes
all nine selected methods. Filtered focused runs are diagnostic, not the full
gate. In one focused observation, strict/combined native transfer sequences took
96,754/93,121 microseconds respectively; these are synthetic-creation component
timings, not model or coding scores.

The complete affected API gate is **PASS**: 238 passed across 11 unfiltered
targets, zero failed, ignored, measured or filtered tests. All seven new named
methods pass. Formatting and all-target Clippy pass with seven Rust source
hashes unchanged throughout. Earlier host/Linux failures remain outside this
API-only selection and are not cleared. Final prose and retained-review checks
are run separately after these source checks.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 0.30 |
| Clippy, all API targets | 0 | 1.75 |
| Tests, all API targets, no fail-fast | 0 | 44.51 |
| Complete affected gate | 0 | 47.08 |

Raw log SHA-256:
`f30eea0f6175e5fd0cd69cfe2a07ab484d8800ab47abdaacaa64b6d386139ba8`.
The private reproduction bundle binds the complete raw log, seven source hashes,
initial failed fixture and repaired focused runs. Each Cargo command takes its
own shared reservation with two build jobs and four test threads. No reservation
was bypassed. No supply-chain or long-gate regeneration is part of this batch.

## Review and remaining scope

The preceding original-review increment has an independent
[Verdict: PASS](../reviews/24633619dec4-local-coding-original-review.txt)
for the exact base commit, with no high or medium finding. The report is retained
unchanged. Its low client-coverage observation is addressed by this batch,
pending independent review. Extraction-error response handling, direct older
non-turn service coverage and native combined model-edit/test transfer remain
same-row backlog. No new sub-row is added for a finding.

Use an explicitly started original-review service: ordinary services discard
the owner at release. The terminal output is not an automatically saved record.
A faulted prefix requires separate inspection. Errors can lose process-only
ownership; a path or serialized record cannot recreate it. Declining after
release may leave the retained owner or an inert preview for manual inspection.
The earlier cancelled or failed outcome remains unchanged by a later transfer.
The original checkout is not tested by this command, and a native `committed`
write result is not a Git commit.

The executable-ownership environment blocker still prevents native creation and
command admission in this lane. The unchanged host/Linux failures are outside
this API-only gate. No native/model workflow, evaluation score, model admission,
durable crash recovery, approved commit/push, parent closure or release is
claimed. Frozen tasks, hidden tests, held-out data, scorer, thresholds, model
prompts, dependencies, SBOM, status model and scoreboard are unchanged. No GPU,
model download, trust exception or resource increase is used. Independent core
review of this increment is required.
