# T-303/T-306: frozen line-range one-hundred-forty-four-attempt checkpoint

Frozen source: `53c09dac184fab2a75ee57f0f540a3ae4ad2d9bb`.
Reporting parent: `6647ae778ca88341f9d7b74fd1a6e4361e06c9ef`.

This increment records dev-023 and dev-024 on all six existing profiles:
**12 new attempts; 144/180 cumulative, 36 remaining**. Cumulative strict
ungraded proposals: gemma4-31b 19/24; qwen35-27b 19/24; gpt-oss 10/24; muse 10/24; gemma4-e4b 13/24; gemma4-26b-a4b 15/24. These are not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

| Profile | New strict ungraded proposals | Cumulative strict ungraded proposals | New rendered input / requested output tokens | New backend input / output tokens | New request and strict-check seconds | New reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- | --- |
| gemma4-31b | 2/2 | 19/24 | 5650–8420 / 24316–27086 | 14070 / 1574 (2/2 terminal counters) | 102.926 | 2.883 |
| qwen35-27b | 2/2 | 19/24 | 4915–7282 / 25454–27821 | 12197 / 1484 (2/2 terminal counters) | 76.725 | 1.808 |
| gpt-oss | 1/2 | 10/24 | 4604–6784 / 25952–28132 | 11388 / 5647 (2/2 terminal counters) | 47.010 | 1.250 |
| muse | 2/2 | 10/24 | 4580–6783 / 25953–28156 | 11363 / 10784 (2/2 terminal counters) | 270.626 | 5.196 |
| gemma4-e4b | 1/2 | 13/24 | 5646–8416 / 24320–27090 | 14062 / 27131 (2/2 terminal counters) | 254.779 | 4.483 |
| gemma4-26b-a4b | 1/2 | 15/24 | 5650–8420 / 24316–27086 | 14070 / 28523 (2/2 terminal counters) | 306.568 | 5.806 |

gemma4-31b: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 19}`. Cumulative backend counter coverage 24/24; terminations `{"stopped-below-token-limits": 24}`. Does not fit: 0.

qwen35-27b: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 19}`. Cumulative backend counter coverage 24/24; terminations `{"stopped-below-token-limits": 24}`. Does not fit: 0.

gpt-oss: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"generation-refused:error:chat.response.incomplete": 7, "strict-edit-refused:code.proposal.edit": 7, "ungraded-proposal": 10}`. Cumulative backend counter coverage 24/24; terminations `{"output-cap": 7, "stopped-below-token-limits": 17}`. Does not fit: 0.

muse: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 7, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 10}`. Cumulative backend counter coverage 18/24; terminations `{"stopped-below-token-limits": 18, "transport-timeout-no-terminal-counters": 6}`. Does not fit: 0.

gemma4-e4b: new causes `{"generation-refused:complete:length": 1, "ungraded-proposal": 1}`; cumulative causes `{"generation-refused:complete:length": 2, "strict-edit-refused:code.proposal.edit": 9, "ungraded-proposal": 13}`. Cumulative backend counter coverage 24/24; terminations `{"output-cap": 2, "stopped-below-token-limits": 22}`. Does not fit: 0.

gemma4-26b-a4b: new causes `{"generation-refused:complete:length": 1, "ungraded-proposal": 1}`; cumulative causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 8, "ungraded-proposal": 15}`. Cumulative backend counter coverage 24/24; terminations `{"output-cap": 1, "stopped-below-token-limits": 23}`. Does not fit: 0.

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
No complete-response check is relaxed. The previously recorded Muse dev-012 request also times
out at 300 seconds with no captured response, answer or terminal counters.
Its unchanged base candidate, exact timeout event and missing response are
validated with the same twenty-one mutation controls. It is counted as a
generation refusal; its counters remain unavailable. This reporting-only
observation adds no recovery, retry or runtime change.
Historical Muse dev-014 also reaches the unchanged 300-second client deadline with
no response capture, answer or terminal counters. Its exact timeout event,
unchanged base candidate and missing response pass the same twenty-one
negative controls. It remains a generation refusal; missing counts are not zero.
Dev-007's late backend release appears
before dev-008's slot launch in the latter task's slice. The independent
checkpoint-48 review quantifies this wait as roughly 11 seconds, leaving about
289 seconds of its unchanged 300-second deadline for generation. Dev-008's
timeout may have been affected by carry-over; whether the extra time would
have completed its answer is unknown. It remains a timeout under the frozen
protocol. Later events are not borrowed to fill missing per-task terminal
counters.

Historical Muse dev-017 reaches the unchanged 300-second client deadline with an
incomplete 16,384-byte relay prefix and no answer or terminal counters in its
own task slice. The same reporting-only checks and twenty mutation controls
validate this timeout; no partial answer is repaired or graded. Its backend
finishes later: 3,600 input / 11,048 output counters appear before dev-018's
slot launch in the dev-018 slice, and stay excluded by the frozen protocol.
Dev-018 request time includes roughly eight seconds waiting for that abandoned
generation. No time or missing counters are borrowed from the later task.

Only Rust retrieval, credential-owning chat HTTP transport, local inference
and the strict checker run. Original native executable admission and grading
controls remain unavailable and are not reconstructed. No native effects,
project tests, feedback, retries, candidate, hidden or held-out code executes.
No task pass, qualification or role adoption is claimed.

The unchanged controller resumes six two-task jobs. Each separately acquires
both host wrappers with a 900-second deadline, low priority, nice 19, 16 GiB RAM,
four CPUs and the existing 22,528 MiB sampled GPU guard. Both reservations
release after every pair, with five 30-second pauses. Controller wall time
including queues and pauses is 1452.669 seconds. Reserved-job body time excludes
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
30 contexts, 165 prior checks, 185 range equivalences and all 144 attempts.
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

The [preceding independent core review](../reviews/6647ae778ca8-line-range-development-checkpoint132.txt) returns **Verdict: PASS** for exact `6647ae778ca88341f9d7b74fd1a6e4361e06c9ef`. Its unchanged report is retained; the metric-derivation low remains T-309 backlog. The review confirms the earlier edit-receipt and editorial lows are resolved. Refusal sub-reasons remain later analysis backlog. No row closes.
Independent core review is required for this increment; no row closes.
Earlier accepted low findings remain T-309 backlog. Any newly available high
or medium finding takes priority over further work.
