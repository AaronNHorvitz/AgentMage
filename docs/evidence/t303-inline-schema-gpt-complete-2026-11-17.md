# Inline-schema gpt-oss development completion — 2026-11-17 UTC

**30/30 frozen development attempts; 6 strict ungraded proposals.**
First-attempt and within-budget task passes remain **unavailable**.
30 attempts remain before tuning or new capabilities.
T-303/T-306 remain open; independent core review is required.

Measured source: `72c002a42957340de0b6fa0eb4d48385db2c7e62`. The available path is existing Rust
context selection → credential-owning chat HTTP API → original strict edit
checker. Native executable admission and original grading controls remain
unavailable. Full repository retrieval, native effects, project tests, feedback
and retries cannot run. No control is reconstructed. No candidate, hidden or
held-out test ran. Proposals are unapplied and untested.

| Outcome / cause | Attempts |
| --- | --- |
| `generation-refused:complete:length` | 1 |
| `generation-refused:error:chat.response.incomplete` | 8 |
| `strict-edit-refused:code.proposal.edit` | 15 |
| `ungraded-proposal` | 6 |
| Does not fit | 0 |

Request/check time: **2605.278 seconds**. Completed-job reserved work:
**2839.246 seconds / 47.321 minutes**, including model verification,
loading, requests and cleanup after frozen-input validation. Queues, wrapper
overhead, pre-entry validation and released pauses are excluded. The recorded
sum of controller launch-to-exit UTC intervals is **8257.204 seconds**, including
queues and released pauses; the gap between sessions, authentication, preflight
and reporting are outside those intervals. The preceding segment is 623.220 seconds and the resumed
segment is 7633.984 seconds. GPU kernel time and complete
end-to-end latency are unmeasured.
Backend counters cover **30/30**, with **188409 input / 470692 output tokens**.
Backend terminations: `{"output-cap": 9, "stopped-below-token-limits": 21}`.
Captures: `{"complete-choice-json": 30}`. Reasoning metadata:
`{"non-empty": 30}`. Missing metadata is not zero.

Changed variable from this model's preceding repeat: trusted named inline
`json_schema` with `strict: true`, verified in upstream request captures.
Contexts and messages match that repeat byte for byte. Model/profile, runtime,
helper, prompts, strict checker, complete framing, requested thinking option
and 300-second total/idle deadlines remain fixed. The model profile retains
`reasoning_effort: medium`; the transport still requests thinking disabled.
These record the requests, not a claim about the model's internal reasoning.
Context is 32,768 tokens; output follows
`min(32768, 32768 - rendered input tokens - 32)`, ranging **24219–29748**.
Input preflight ranges **2988–8517**. No answer repair or fallback is added.
Strict acceptance does not establish a correct solution, improvement,
model admission or role qualification.

## Frozen inputs and resources

The licence batch changed 195 working-tree files covered by the frozen source
manifest. The launcher authenticates the complete **9,630-file Git archive**
and mounts it read-only at the original source address, with existing Git
metadata also read-only and existing device nodes preserved. All **2,283
repository and 26 private frozen hashes** pass. No binding is removed, narrowed
or renewed. This measures frozen source, not the relicensed working tree.

Before inference, all **30 development contexts** reproduce byte for byte
against this model's own authenticated preceding repeat. Its comparison report
comes from its sealed archive. The preceding four-attempt gpt-oss capsule, postcommit receipt,
saved attempts and candidate source bytes also pass authentication. The first four attempts are carried byte for byte; this session adds the
remaining 26, without replay. Earlier source-only preparation failures remain
in the completed 27B capsule, linked by the preceding review chain. This resume
uses the established full Git-backed mount without a new failed GPU preparation.

Each two-task job uses both shared wrappers, low priority, nice 19, 16 GiB/four
CPUs, a 900-second GPU reservation and 30-second released pauses. Optional
wrapper overrides are recorded absent. The controller changes only its
reservation-log parent. Original batch, collector, binary, helper, prompts,
checker and limits stay fixed. No download, privilege or device is added.

All 15 completed jobs revoke settings and reap API/model owners; model keys
and sockets are absent. Cleanup counts **30 retired API token files** from the
actual disjoint list. Exact-argument scanning covers this PID namespace only.
Sampled total GPU peaks range **13807–13876 MiB**. No foreign process is signalled.

## Verification and limits

Source/outcome audit, capture/context/profile comparison and original-helper
reproduction pass. Only the original context and strict checker process saved
answers; generated code and hidden tests never execute. The sealed review
capsule retains raw requests/responses, complete candidate projections,
contexts, records, all frozen inputs, the complete source archive, comparison
receipts and exact methods/invocations. The verifier reads and hashes files and
Git objects; it imports no archived module. Run it from the exact pinned
checkout, because Git and preservation reads use the caller's working directory.

Reporting gate, Markdown lint/rendering, privacy/licence checks and preservation
checks pass. The earlier scoreboard is an exact prefix and all **509 ledger
table lines** remain unchanged, as do the original 49 historical private-path
blobs, inventory, Decision 0051 and status model. The full source/acceptance
gates are not rerun. Inherited source **FAIL: 1,169 passed / 53 failed / 19 ignored**
remains disclosed in the
[source review](../reviews/ac1c30201171-trusted-inline-output-schema.txt).
The licence batch's two inherited Python failures and supply-chain limits
remain in its [evidence](readme-and-dual-licence-2026-11-16.md).
No row closes; no runtime, platform, model, release or owner-ready promotion.

## Observed response fields

The response capture, rather than internal model behavior, supplies these
counts. Every request still carries `enable_thinking: false` and the profile
still sets `reasoning_effort: medium`. These settings are not changed here.
Character counts are Unicode characters, not tokens. The sealed capsule
retains UTF-8 byte lengths and raw capture hashes as well.

| Task | Finish | Answer characters | Reasoning field characters |
| --- | --- | --- | --- |
| dev-001 | length | 0 | 74721 |
| dev-002 | length | 0 | 100587 |
| dev-003 | stop | 2837 | 40195 |
| dev-004 | length | 0 | 139904 |
| dev-005 | stop | 909 | 45433 |
| dev-006 | stop | 2156 | 23018 |
| dev-007 | stop | 2488 | 19815 |
| dev-008 | length | 0 | 106672 |
| dev-009 | stop | 1377 | 11774 |
| dev-010 | stop | 3849 | 71194 |
| dev-011 | length | 0 | 114014 |
| dev-012 | stop | 3843 | 42613 |
| dev-013 | length | 274 | 108747 |
| dev-014 | stop | 8708 | 62618 |
| dev-015 | stop | 887 | 73519 |
| dev-016 | stop | 4309 | 25799 |
| dev-017 | stop | 1514 | 34779 |
| dev-018 | length | 0 | 106718 |
| dev-019 | stop | 535 | 4487 |
| dev-020 | stop | 1965 | 15527 |
| dev-021 | length | 0 | 104431 |
| dev-022 | length | 0 | 102329 |
| dev-023 | stop | 2468 | 64119 |
| dev-024 | stop | 2057 | 24112 |
| dev-025 | stop | 5851 | 101887 |
| dev-026 | stop | 1051 | 11397 |
| dev-027 | stop | 3877 | 68991 |
| dev-028 | stop | 2048 | 53251 |
| dev-029 | stop | 18942 | 54230 |
| dev-rust-001 | stop | 3899 | 45455 |

Field combinations: `{"empty content / non-empty reasoning_content": 8, "non-empty content / non-empty reasoning_content": 22}`.
