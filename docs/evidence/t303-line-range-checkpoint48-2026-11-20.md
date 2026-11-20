# T-303/T-306: frozen line-range forty-eight-attempt checkpoint

Frozen source: `53c09dac184fab2a75ee57f0f540a3ae4ad2d9bb`.
Reporting parent: `7b1ecdf54063d3323ea54b28d56b4671d7f9aedb`.

This increment records dev-007 and dev-008 on all six existing profiles:
**12 new attempts; 48/180 cumulative, 132 remaining**. Cumulative strict
ungraded proposals: gemma4-31b 4/8; qwen35-27b 6/8; gpt-oss 3/8; muse 2/8; gemma4-e4b 3/8; gemma4-26b-a4b 3/8. These are not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

| Profile | New strict ungraded proposals | Cumulative strict ungraded proposals | New rendered input / requested output tokens | New backend input / output tokens | New request and strict-check seconds | New reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- | --- |
| gemma4-31b | 1/2 | 4/8 | 9272–9545 / 23191–23464 | 18817 / 4493 (2/2 terminal counters) | 177.572 | 3.911 |
| qwen35-27b | 1/2 | 6/8 | 8057–8331 / 24405–24679 | 16388 / 3379 (2/2 terminal counters) | 127.793 | 2.857 |
| gpt-oss | 0/2 | 3/8 | 7428–7668 / 25068–25308 | 15096 / 45318 (2/2 terminal counters) | 285.493 | 5.288 |
| muse | 0/2 | 2/8 | 7445–7676 / 25060–25291 | Unavailable (0/2 terminal counters) | 600.038 | 10.877 |
| gemma4-e4b | 1/2 | 3/8 | 9268–9541 / 23195–23468 | 18809 / 13266 (2/2 terminal counters) | 164.417 | 3.063 |
| gemma4-26b-a4b | 1/2 | 3/8 | 9272–9545 / 23191–23464 | 18817 / 5776 (2/2 terminal counters) | 108.100 | 2.714 |

gemma4-31b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 4}`. Cumulative backend counter coverage 8/8; terminations `{"stopped-below-token-limits": 8}`. Does not fit: 0.

qwen35-27b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 6}`. Cumulative backend counter coverage 8/8; terminations `{"stopped-below-token-limits": 8}`. Does not fit: 0.

gpt-oss: new causes `{"generation-refused:error:chat.response.incomplete": 1, "strict-edit-refused:code.proposal.edit": 1}`; cumulative causes `{"generation-refused:error:chat.response.incomplete": 2, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 3}`. Cumulative backend counter coverage 8/8; terminations `{"output-cap": 2, "stopped-below-token-limits": 6}`. Does not fit: 0.

muse: new causes `{"generation-refused:error:chat.transport.timeout": 2}`; cumulative causes `{"generation-refused:error:chat.transport.timeout": 3, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 2}`. Cumulative backend counter coverage 5/8; terminations `{"stopped-below-token-limits": 5, "transport-timeout-no-terminal-counters": 3}`. Does not fit: 0.

gemma4-e4b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 3}`. Cumulative backend counter coverage 8/8; terminations `{"output-cap": 1, "stopped-below-token-limits": 7}`. Does not fit: 0.

gemma4-26b-a4b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 3}`. Cumulative backend counter coverage 8/8; terminations `{"stopped-below-token-limits": 8}`. Does not fit: 0.

The complete line-reference protocol stays frozen under
[Decision 0214](../decisions/0214-original-line-range-proposals.md): instruction,
version 2 headers, parser and trusted schema. Source bodies remain exact; the
original preflight's nine excerpt differences remain recorded. Finish all 30
development tasks on all six profiles before tuning or capability work.
Context remains 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)` per task. Model/runtime
identities, seed, temperature, thinking request and 300-second transport
deadlines remain unchanged. Requested thinking settings do not demonstrate
absence of internal reasoning. API usage is not requested; backend terminal
counters are reported separately, with coverage. Missing counts remain
unavailable, never zero or extrapolated. Historical Muse dev-005 remains a
transport timeout with an incomplete 16,384-byte prefix; its reporting-only
validation and twenty negative controls are retained and reproduced. As the
preceding review notes, that backend continued after the client deadline:
its terminal counters (6,090 input / 11,553 output) appear in dev-006's slice,
after dev-005's deadline. They remain excluded by the frozen per-task-slice
protocol. Dev-006's request time includes roughly 6.5 seconds waiting for that
abandoned generation. Backend cancellation on client timeout is recorded for
T-309 after this frozen repeat; the protocol is unchanged now.

Muse dev-007 and dev-008 also reach the unchanged 300-second deadline, with no
captured HTTP response or retained response bytes. Their exact timeout events,
base fallbacks and each request's missing terminal counters are checked without
reconstructing an answer. Twenty-one mutation controls per timeout reject unobserved keys,
altered classifications, successful grading, invented captures or bytes and
terminal-counter claims. The portable verifier reruns them from the capsule.
No complete-response check is relaxed. Dev-007's late backend release appears
before dev-008's slot launch in the latter task's slice; dev-008's request time
includes that wait. Later events are not borrowed to fill missing per-task
terminal counters.

Only Rust retrieval, credential-owning chat HTTP transport, local inference
and the strict checker run. Original native executable admission and grading
controls remain unavailable and are not reconstructed. No native effects,
project tests, feedback, retries, candidate, hidden or held-out code executes.
No task pass, qualification or role adoption is claimed.

The unchanged controller resumes six two-task jobs. Each separately acquires
both host wrappers with a 900-second deadline, low priority, nice 19, 16 GiB RAM,
four CPUs and the existing 22,528 MiB sampled GPU guard. Both reservations
release after every pair, with five 30-second pauses. Controller wall time
including queues and pauses is 1892.214 seconds. Reserved-job body time excludes
queues and is neither kernel time nor complete reservation duration.
Preparation, reporting and full end-to-end latency remain unmeasured. The lane's
memory.high threshold constrains timings; do not compare with unthrottled runs.
No limit is raised, foreign process signalled or model downloaded. Launch wrapper
bytes are retained; per-pair wrapper provenance remains T-309 backlog.

Twelve new token retirement receipts are persisted before unlinking; all earlier
receipts remain intact. Settings are revoked, owned API/model processes reaped,
and keys/sockets/tokens absent. The read-only source retains 9,823 original
files, 2,355 repository bindings and 78 private bindings. The launcher binds the
reporting HEAD and its exact allowed diff from the source. The complete preceding
checkpoint and original protocol capsules are retained and authenticated.
The portable verifier checks every inventory entry, original source archive,
30 contexts, 165 prior checks, 185 range equivalences and all 48 attempts.
Complete candidate projections are compared as data, including unedited files.
Only authenticated effect-free helpers execute. All 30 payloads are included;
cross-mount packaging preserves executable modes. New post-seal receipts and
logs live beside the capsule. Earlier failed and corrected packages stay intact.

Reporting fast gate, Markdown lint/render, privacy, licence, raw-attempt audit,
cleanup, preservation and whole-capsule reproduction are checked. No runtime
source changes occur. The preceding full source gate remains **FAIL**:
1,043 passed, 53 inherited failures, 11 ignored, zero filtered, 31 targets.
No failure is waived. Existing Python, licence and supply-chain limitations
remain. All 512 ledger table lines, 49 historical path blobs, README, inventory,
Decision 0051, status and supply-chain outputs remain unchanged. Earlier
scoreboard bytes remain an exact prefix. No SBOM regeneration, long gate,
acceptance, release, owner-test-ready or Git push occurs.

The [preceding independent core review](../reviews/7b1ecdf54063-line-range-development-checkpoint36.txt)
returns **Verdict: PASS** for exact `7b1ecdf54063d3323ea54b28d56b4671d7f9aedb`, with no high or medium finding.
The report is retained byte-identically. Its low findings concern the late
backend completion disclosed above and independent metric derivation in the
portable verifier. Both remain T-309 backlog; no row closes on this report.
Independent core review is required for this increment; no row closes.
Earlier accepted low findings remain T-309 backlog. Any newly available high
or medium finding takes priority over further work.
