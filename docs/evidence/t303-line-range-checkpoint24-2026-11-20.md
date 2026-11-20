# T-303/T-306: frozen line-range twenty-four-attempt checkpoint

Frozen source: `53c09dac184fab2a75ee57f0f540a3ae4ad2d9bb`.
Reporting parent: `d258cfe20b06b89be40f2e69e8012630bf334a3f`.

This increment records dev-003 and dev-004 on all six existing profiles:
**12 new attempts; 24/180 cumulative, 156 remaining**. Cumulative strict
ungraded proposals: gemma4-31b 1/4; qwen35-27b 4/4; gpt-oss 2/4; muse 2/4; gemma4-e4b 1/4; gemma4-26b-a4b 1/4. These are not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

| Profile | New strict ungraded proposals | Cumulative strict ungraded proposals | New rendered input / requested output tokens | New backend input / output tokens | New request and strict-check seconds | New reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- | --- |
| gemma4-31b | 1/2 | 1/4 | 3061–4457 / 28279–29675 | 7518 / 3720 | 168.775 | 4.020 |
| qwen35-27b | 2/2 | 4/4 | 2804–4063 / 28673–29932 | 6867 / 4069 | 125.308 | 2.835 |
| gpt-oss | 1/2 | 2/4 | 2710–3871 / 28865–30026 | 6581 / 37832 | 200.509 | 3.616 |
| muse | 1/2 | 2/4 | 2654–3824 / 28912–30082 | 6478 / 18402 | 441.018 | 7.998 |
| gemma4-e4b | 1/2 | 1/4 | 3057–4453 / 28283–29679 | 7510 / 3771 | 101.601 | 2.023 |
| gemma4-26b-a4b | 0/2 | 1/4 | 3061–4457 / 28279–29675 | 7518 / 7895 | 130.291 | 3.202 |

gemma4-31b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 1}`. Cumulative backend counter coverage 4/4; terminations `{"stopped-below-token-limits": 4}`. Does not fit: 0.

qwen35-27b: new causes `{"ungraded-proposal": 2}`; cumulative causes `{"ungraded-proposal": 4}`. Cumulative backend counter coverage 4/4; terminations `{"stopped-below-token-limits": 4}`. Does not fit: 0.

gpt-oss: new causes `{"generation-refused:error:chat.response.incomplete": 1, "ungraded-proposal": 1}`; cumulative causes `{"generation-refused:error:chat.response.incomplete": 1, "strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 2}`. Cumulative backend counter coverage 4/4; terminations `{"output-cap": 1, "stopped-below-token-limits": 3}`. Does not fit: 0.

muse: new causes `{"strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 1}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 1, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 2}`. Cumulative backend counter coverage 4/4; terminations `{"stopped-below-token-limits": 4}`. Does not fit: 0.

gemma4-e4b: new causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; cumulative causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 1}`. Cumulative backend counter coverage 4/4; terminations `{"output-cap": 1, "stopped-below-token-limits": 3}`. Does not fit: 0.

gemma4-26b-a4b: new causes `{"strict-edit-refused:code.proposal.edit": 2}`; cumulative causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 1}`. Cumulative backend counter coverage 4/4; terminations `{"stopped-below-token-limits": 4}`. Does not fit: 0.

[Decision 0214](../decisions/0214-original-line-range-proposals.md)'s complete
line-reference protocol stays frozen: instruction, version 2 source framing,
strict parser and trusted `coding_lines_v1` schema. Source bodies remain exact.
The original preflight records nine changed excerpt sets against the preceding
raw-source experiment; this increment changes none. All 30 development tasks
on all six profiles must finish before further tuning or capability work.

Context stays 32,768 tokens, with per-task output
`min(32768, 32768 - rendered input tokens - 32)`. Model/runtime identities,
temperature, seed, thinking request, 300-second transport deadlines and profile
settings stay unchanged. Wire requests, response fields, observed backend
counters, raw logs and all candidate bytes are retained. Requested thinking
settings do not prove the absence of internal reasoning. API usage is not
requested; observed backend token counters are reported separately.

Only Rust retrieval, credential-owning chat HTTP transport, local inference and
the production strict checker run. Native executable admission and original
execution/grading controls remain unavailable and are not reconstructed.
No native effects, project tests, feedback, retries, candidate, hidden or
held-out code executes. No model qualification or role adoption follows.

The unchanged controller resumes six two-task jobs. Each independently acquires
both host wrappers with a 900-second deadline, low priority, nice 19, a 16 GiB
RAM/four-CPU scope and the existing 22,528 MiB sampled GPU guard. Both reservations
release between pairs, including five 30-second pauses. This controller's wall
time, including queues and pauses, is 1637.523 seconds. Reserved-job body time
excludes queues and is neither kernel time nor complete reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
No limit is raised, foreign process signalled or model downloaded. The lane
has a memory.high threshold; these constrained timings must not be compared
with unthrottled runs. Launch wrapper bytes are retained; per-pair wrapper
provenance stays T-309 backlog.

All twelve new token retirement receipts are persisted and checked before
unlinking. Earlier receipts remain unchanged. Settings are revoked, owned API
and model processes reaped, and keys/sockets/tokens absent. Complete source
and input checks retain 9,823 original source files, 2,355 repository bindings
and 78 private bindings. The reporting-only parent diff is explicitly bound.
The complete preceding checkpoint capsule and original protocol capsule are
retained and authenticated, with every earlier campaign/candidate byte intact.
The portable verifier checks the whole inventory, original source archive,
30 contexts, 165 prior checks, 185 range equivalences and all 24 attempts.
It executes only the authenticated effect-free helper, never candidate code.

The separate source review returns **Verdict: PASS** for exact
`53c09dac184fab2a75ee57f0f540a3ae4ad2d9bb`, with no high, medium or low finding;
its [unchanged report](../reviews/53c09dac184f-original-line-range-protocol.txt)
is retained. This does not review or close this measurement increment.
The preceding twelve-attempt checkpoint also returns **Verdict: PASS** for exact
`d258cfe20b06b89be40f2e69e8012630bf334a3f`, with no high or medium finding;
its [unchanged report](../reviews/d258cfe20b06-line-range-development-checkpoint12.txt)
is retained. Its low L1 remains T-309 backlog: historical post-seal packaging
receipts under private state were outside the reviewer mount; future external
receipts belong beside capsules in the review mount. The current packer uses
`copy2` for cross-mount fallback and keeps final verification logs beside the
capsule. The old capsule and its external receipts remain unchanged.

Reporting fast gate, Markdown lint/render, privacy, licence, raw-attempt audit,
cleanup, preservation and complete capsule reproduction are checked for this
batch. The initial auxiliary render check incorrectly treated the two unchanged
archived `.txt` review reports as authored Markdown. Its failed log is retained;
the corrected check lints authored Markdown and renders all five files, while
the established fast gate separately validates review metadata. No archived
review byte or Markdown rule changes.
The first precommit reproduction then failed because the packager omitted
`work/payloads`, although all 30 unchanged payloads were present in the complete
preceding capsule. The failed package and log remain intact; the corrected
package restores those exact files and authenticates both complete inventories.
No checker assertion, measured input, attempt or candidate changes, and no
inference reruns. No repository source changes occur. The preceding full source gate
remains **FAIL**: 1,043 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets. No failure is waived. Earlier Python, licence and supply-chain
limitations remain. No SBOM regeneration, long gate, acceptance, release or
Git push occurs.

All 512 ledger table lines, 49 historical path blobs, README, inventory,
Decision 0051, status model and supply-chain outputs remain unchanged.
Earlier scoreboard bytes remain an exact prefix. This checkpoint requires
independent core review and closes no row.
