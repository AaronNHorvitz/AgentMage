# T-303/T-306: frozen line-range thirty-six-attempt checkpoint

Frozen source: `53c09dac184fab2a75ee57f0f540a3ae4ad2d9bb`.
Reporting parent: `78ea34c2e623abd915b81a284bfeb87ba887f420`.

This increment records dev-005 and dev-006 on all six existing profiles:
**12 new attempts; 36/180 cumulative, 144 remaining**. Cumulative strict
ungraded proposals: gemma4-31b 3/6; qwen35-27b 5/6; gpt-oss 3/6; muse 2/6; gemma4-e4b 2/6; gemma4-26b-a4b 2/6. These are not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

| Profile | New strict ungraded proposals | Cumulative strict ungraded proposals | New rendered input / requested output tokens | New backend input / output tokens | New request and strict-check seconds | New reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- | --- |
| gemma4-31b | 2/2 | 3/6 | 8061–9093 / 23643–24675 | 17154 / 1318 (2/2 terminal counters) | 112.848 | 4.192 |
| qwen35-27b | 1/2 | 5/6 | 6905–7875 / 24861–25831 | 14780 / 773 (2/2 terminal counters) | 51.447 | 1.625 |
| gpt-oss | 1/2 | 3/6 | 6179–7231 / 25505–26557 | 13410 / 39115 (2/2 terminal counters) | 247.945 | 4.646 |
| muse | 0/2 | 2/6 | 6090–7245 / 25491–26646 | 7245 / 7933 (1/2 terminal counters) | 496.643 | 10.147 |
| gemma4-e4b | 1/2 | 2/6 | 8057–9089 / 23647–24679 | 17146 / 4001 (2/2 terminal counters) | 58.780 | 1.208 |
| gemma4-26b-a4b | 1/2 | 2/6 | 8061–9093 / 23643–24675 | 17154 / 1671 (2/2 terminal counters) | 89.934 | 2.361 |

gemma4-31b: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 3}`. Cumulative backend counter coverage 6/6; terminations `{"stopped-below-token-limits": 6}`. Does not fit: 0.

qwen35-27b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 5}`. Cumulative backend counter coverage 6/6; terminations `{"stopped-below-token-limits": 6}`. Does not fit: 0.

gpt-oss: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"generation-refused:error:chat.response.incomplete": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 3}`. Cumulative backend counter coverage 6/6; terminations `{"output-cap": 1, "stopped-below-token-limits": 5}`. Does not fit: 0.

muse: new causes `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 1}`; cumulative causes `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 2}`. Cumulative backend counter coverage 5/6; terminations `{"stopped-below-token-limits": 5, "transport-timeout-no-terminal-counters": 1}`. Does not fit: 0.

gemma4-e4b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 2}`. Cumulative backend counter coverage 6/6; terminations `{"output-cap": 1, "stopped-below-token-limits": 5}`. Does not fit: 0.

gemma4-26b-a4b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 2}`. Cumulative backend counter coverage 6/6; terminations `{"stopped-below-token-limits": 6}`. Does not fit: 0.

[Decision 0214](../decisions/0214-original-line-range-proposals.md)'s line-reference
protocol stays frozen: instruction, version 2 source headers, strict parser and
trusted `coding_lines_v1` schema. Source bodies remain exact. The original
preflight's nine changed excerpt sets are preserved. Finish all 30 development
tasks on all six profiles before further tuning or capability work.

Context stays 32,768 tokens, with per-task output
`min(32768, 32768 - rendered input tokens - 32)`. Model/runtime identities,
temperature, seed, thinking request and 300-second transport deadlines remain
unchanged. Requests, responses, backend counters, raw logs and complete candidate
projections are retained. Requested thinking settings do not prove the absence
of internal reasoning. API usage is not requested; backend counters are separate.

Muse dev-005 reaches the unchanged 300-second transport deadline. The relay
retains a 16,384-byte incomplete response prefix with HTTP status 200; its
prefix is checked only for incompleteness; no answer is recovered, repaired or
graded. The task's captured backend slice
has generation progress but no terminal token counters. Missing counts are
unavailable, not zero or extrapolated from later logs. Reporting-only validation
binds this outcome to its timeout event, retained prefix, unchanged base
candidate and missing terminal counters. Twenty negative controls reject
altered classifications, events, deadlines, captures and terminal-counter claims.
The same controls rerun portably from the sealed capsule. No inference reruns.

Only Rust retrieval, credential-owning chat HTTP transport, local inference and
the production strict checker run. Native executable admission and original
execution/grading controls remain unavailable and are not reconstructed.
No native effects, project tests, feedback, retries, candidate, hidden or
held-out code executes. No model qualification or role adoption follows.

The unchanged controller resumes six two-task jobs. Each independently acquires
both host wrappers with a 900-second deadline, low priority, nice 19, a 16 GiB
RAM/four-CPU scope and the existing 22,528 MiB sampled GPU guard. Both reservations
release between pairs, including five 30-second pauses. Controller wall time,
including queues and pauses, is 1618.801 seconds. Reserved-job body time excludes
queues and is neither kernel time nor complete reservation duration. Preparation,
reporting and complete end-to-end latency remain unmeasured. The lane memory.high
threshold constrains these timings; do not compare them with unthrottled runs.
No limit is raised, foreign process signalled or model downloaded. Launch wrapper
bytes are retained; per-pair wrapper provenance stays T-309 backlog.

Twelve new token retirement receipts are persisted and checked before unlinking;
earlier receipts remain unchanged. Settings are revoked, owned API/model
processes reaped, and keys/sockets/tokens absent. The complete read-only source
view retains 9,823 original files, 2,355 repository bindings and 78 private
bindings. The reporting-only parent diff is explicitly bound. The complete
preceding checkpoint and original protocol capsules are retained and authenticated,
with all earlier attempts and candidates unchanged. The portable verifier checks
the whole inventory, original source archive, 30 contexts, 165 prior checks,
185 range equivalences and all 36 attempts. Candidate projections are compared
as data, including every unedited file. Only effect-free verification and the
authenticated helper execute; candidate code does not execute.

The preceding checkpoint's [unchanged independent core review](../reviews/78ea34c2e623-line-range-development-checkpoint24.txt)
returns **Verdict: PASS** for exact `78ea34c2e623abd915b81a284bfeb87ba887f420`. This accepts only that reporting
increment. It does not close T-303/T-306 or review the current checkpoint.
Low L1 (executed-finalizer visibility) and L2 (published metrics not independently
derived by the verifier) remain backlog for T-309.

The source review and the earlier checkpoint review remain retained unchanged.
The source review has no findings. Earlier low findings, including historical
post-seal receipt visibility, remain T-309 backlog. New post-seal receipts and
verification logs live beside the capsule in the review mount. Cross-mount
packaging preserves executable modes, and all 30 frozen payloads are included.
The previous failed and corrected packages remain intact inside the predecessor.

Reporting fast gate, Markdown lint/render, privacy, licence, raw-attempt audit,
cleanup, preservation and complete capsule reproduction are checked for this
batch. An auxiliary final recheck omitted the pinned Node binary from PATH;
its retained log reports exit 127 for Markdown lint. The corrected recheck
uses the verified existing binary; no measurement input, attempt or candidate
changes. No repository source changes occur. The preceding full source gate
remains **FAIL**: 1,043 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets. No failure is waived. Earlier Python, licence and supply-chain
limitations remain. No SBOM regeneration, long gate, acceptance, release or
Git push occurs.

All 512 ledger table lines, 49 historical path blobs, README, inventory,
Decision 0051, status model and supply-chain outputs remain unchanged. Earlier
scoreboard bytes remain an exact prefix. Independent core review is required
for this checkpoint; no row closes.
