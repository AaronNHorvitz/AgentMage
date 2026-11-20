# T-303/T-306: frozen line-range sixty-attempt checkpoint

Frozen source: `53c09dac184fab2a75ee57f0f540a3ae4ad2d9bb`.
Reporting parent: `a1e8b1bcdb42036cc75f42e40ddec5c4f5010a73`.

This increment records dev-009 and dev-010 on all six existing profiles:
**12 new attempts; 60/180 cumulative, 120 remaining**. Cumulative strict
ungraded proposals: gemma4-31b 6/10; qwen35-27b 8/10; gpt-oss 3/10; muse 4/10; gemma4-e4b 5/10; gemma4-26b-a4b 5/10. These are not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

| Profile | New strict ungraded proposals | Cumulative strict ungraded proposals | New rendered input / requested output tokens | New backend input / output tokens | New request and strict-check seconds | New reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- | --- |
| gemma4-31b | 2/2 | 6/10 | 3237–4116 / 28620–29499 | 7353 / 349 (2/2 terminal counters) | 65.173 | 2.043 |
| qwen35-27b | 2/2 | 8/10 | 2853–3744 / 28992–29883 | 6597 / 838 (2/2 terminal counters) | 48.555 | 1.401 |
| gpt-oss | 0/2 | 3/10 | 2703–3597 / 29139–30033 | 6300 / 31168 (2/2 terminal counters) | 183.136 | 3.431 |
| muse | 2/2 | 4/10 | 2619–3533 / 29203–30117 | 6152 / 9064 (2/2 terminal counters) | 229.668 | 4.321 |
| gemma4-e4b | 2/2 | 5/10 | 3233–4112 / 28624–29503 | 7345 / 930 (2/2 terminal counters) | 45.445 | 1.039 |
| gemma4-26b-a4b | 2/2 | 5/10 | 3237–4116 / 28620–29499 | 7353 / 558 (2/2 terminal counters) | 60.574 | 1.996 |

gemma4-31b: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 6}`. Cumulative backend counter coverage 10/10; terminations `{"stopped-below-token-limits": 10}`. Does not fit: 0.

qwen35-27b: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 8}`. Cumulative backend counter coverage 10/10; terminations `{"stopped-below-token-limits": 10}`. Does not fit: 0.

gpt-oss: new causes `{"generation-refused:error:chat.response.incomplete": 1, "strict-edit-refused:code.proposal.edit": 1}`; cumulative causes `{"generation-refused:error:chat.response.incomplete": 3, "strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 3}`. Cumulative backend counter coverage 10/10; terminations `{"output-cap": 3, "stopped-below-token-limits": 7}`. Does not fit: 0.

muse: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"generation-refused:error:chat.transport.timeout": 3, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 4}`. Cumulative backend counter coverage 7/10; terminations `{"stopped-below-token-limits": 7, "transport-timeout-no-terminal-counters": 3}`. Does not fit: 0.

gemma4-e4b: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 5}`. Cumulative backend counter coverage 10/10; terminations `{"output-cap": 1, "stopped-below-token-limits": 9}`. Does not fit: 0.

gemma4-26b-a4b: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 5}`. Cumulative backend counter coverage 10/10; terminations `{"stopped-below-token-limits": 10}`. Does not fit: 0.

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
before dev-008's slot launch in the latter task's slice. The independent
checkpoint-48 review quantifies this wait as roughly 11 seconds, leaving about
289 seconds of its unchanged 300-second deadline for generation. Dev-008's
timeout may have been affected by carry-over; whether the extra time would
have completed its answer is unknown. It remains a timeout under the frozen
protocol. Later events are not borrowed to fill missing per-task terminal
counters.

Only Rust retrieval, credential-owning chat HTTP transport, local inference
and the strict checker run. Original native executable admission and grading
controls remain unavailable and are not reconstructed. No native effects,
project tests, feedback, retries, candidate, hidden or held-out code executes.
No task pass, qualification or role adoption is claimed.

The unchanged controller resumes six two-task jobs. Each separately acquires
both host wrappers with a 900-second deadline, low priority, nice 19, 16 GiB RAM,
four CPUs and the existing 22,528 MiB sampled GPU guard. Both reservations
release after every pair, with five 30-second pauses. Controller wall time
including queues and pauses is 1022.682 seconds. Reserved-job body time excludes
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
30 contexts, 165 prior checks, 185 range equivalences and all 60 attempts.
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

The [preceding independent core review](../reviews/a1e8b1bcdb42-line-range-development-checkpoint48.txt)
returns **Verdict: PASS** for exact `a1e8b1bcdb42036cc75f42e40ddec5c4f5010a73`, with no high or medium finding.
The report is retained byte-identically. Its low findings concern the late
backend carry-over before dev-008 and independent metric derivation in the
portable verifier. Both remain T-309 backlog; no row closes on this report.
Independent core review is required for this increment; no row closes.
Earlier accepted low findings remain T-309 backlog. Any newly available high
or medium finding takes priority over further work.
