# T-303/T-306: frozen line-range one-hundred-eight-attempt checkpoint

Frozen source: `53c09dac184fab2a75ee57f0f540a3ae4ad2d9bb`.
Reporting parent: `0c01c9802ce54e22e9ec4236081c53d7619ab976`.

This increment records dev-017 and dev-018 on all six existing profiles:
**12 new attempts; 108/180 cumulative, 72 remaining**. Cumulative strict
ungraded proposals: gemma4-31b 14/18; qwen35-27b 14/18; gpt-oss 9/18; muse 6/18; gemma4-e4b 9/18; gemma4-26b-a4b 11/18. These are not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

| Profile | New strict ungraded proposals | Cumulative strict ungraded proposals | New rendered input / requested output tokens | New backend input / output tokens | New request and strict-check seconds | New reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- | --- |
| gemma4-31b | 2/2 | 14/18 | 3373–4323 / 28413–29363 | 7696 / 878 (2/2 terminal counters) | 78.930 | 2.236 |
| qwen35-27b | 1/2 | 14/18 | 2974–3910 / 28826–29762 | 6884 / 904 (2/2 terminal counters) | 38.446 | 1.179 |
| gpt-oss | 2/2 | 9/18 | 2785–3650 / 29086–29951 | 6435 / 31747 (2/2 terminal counters) | 208.381 | 3.859 |
| muse | 0/2 | 6/18 | 2743–3600 / 29136–29993 | 2743 / 8079 (1/2 terminal counters) | 522.150 | 9.379 |
| gemma4-e4b | 0/2 | 9/18 | 3369–4319 / 28417–29367 | 7688 / 3548 (2/2 terminal counters) | 96.529 | 1.882 |
| gemma4-26b-a4b | 1/2 | 11/18 | 3373–4323 / 28413–29363 | 7696 / 1114 (2/2 terminal counters) | 128.538 | 3.460 |

gemma4-31b: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 14}`. Cumulative backend counter coverage 18/18; terminations `{"stopped-below-token-limits": 18}`. Does not fit: 0.

qwen35-27b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 14}`. Cumulative backend counter coverage 18/18; terminations `{"stopped-below-token-limits": 18}`. Does not fit: 0.

gpt-oss: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"generation-refused:error:chat.response.incomplete": 4, "strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 9}`. Cumulative backend counter coverage 18/18; terminations `{"output-cap": 4, "stopped-below-token-limits": 14}`. Does not fit: 0.

muse: new causes `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 1}`; cumulative causes `{"generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 5, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 6}`. Cumulative backend counter coverage 12/18; terminations `{"stopped-below-token-limits": 12, "transport-timeout-no-terminal-counters": 6}`. Does not fit: 0.

gemma4-e4b: new causes `{"strict-edit-refused:code.proposal.edit": 2}`; cumulative causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 8, "ungraded-proposal": 9}`. Cumulative backend counter coverage 18/18; terminations `{"output-cap": 1, "stopped-below-token-limits": 17}`. Does not fit: 0.

gemma4-26b-a4b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 7, "ungraded-proposal": 11}`. Cumulative backend counter coverage 18/18; terminations `{"stopped-below-token-limits": 18}`. Does not fit: 0.

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

New Muse dev-017 reaches the unchanged 300-second client deadline with an
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
including queues and pauses is 1488.525 seconds. Reserved-job body time excludes
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
30 contexts, 165 prior checks, 185 range equivalences and all 108 attempts.
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

The [preceding independent core review](../reviews/0c01c9802ce5-line-range-development-checkpoint96.txt) returns **Verdict: PASS** for exact `0c01c9802ce54e22e9ec4236081c53d7619ab976`. Its unchanged report is retained; its carried low on independent metric derivation stays T-309 backlog. The preceding adaptation-provenance low is resolved. No row closes.
Independent core review is required for this increment; no row closes.
Earlier accepted low findings remain T-309 backlog. Any newly available high
or medium finding takes priority over further work.
