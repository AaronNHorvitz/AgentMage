# Copy original fragments: first four 31B development attempts — 2026-11-17 UTC

The next frozen profile records **2/4 strict ungraded proposals** on dev-001 through dev-004.
**First-attempt and within-budget task passes remain unavailable.** The 30
completed A4B attempts are preserved without replay. This brings coverage to
34/180 attempts; **146/180 remain** before further tuning or capability work.
T-303/T-306 remain open; this non-closing increment requires independent core review.

The [unchanged independent A4B report](../reviews/a17e95952736-copy-original-fragment-a4b-development-completion.txt) returns **Verdict: PASS** for exact `a17e9595273683fcdc477cba4ef2601a1b3f0688`. It accepts only the prior measurement and closes no row.
The review's low L1 joins T-309: correction provenance should record every
reporting-script edit and retain every edited method. Earlier notes, including
helper build provenance and native executable admission wording, remain open.
This batch retains its reporting methods, including its review-request writer.
No helper or measured setting changes.

## Measurements

| Observed value | First four 31B attempts |
| --- | --- |
| Development tasks | dev-001 through dev-004; one attempt each |
| Prompt/checker source | `35217c60a56a5d3e065148eb9a6aea898b5e1b16` |
| Served context | 32,768 tokens |
| Rendered input preflight | 3229–9570 tokens |
| Requested output budget | 23166–29507 tokens |
| Strict ungraded proposals | 2/4 |
| First-attempt / within-budget task passes | Unavailable / Unavailable |
| Request and checker time | 309.096 seconds |
| Completed-job reserved work | 384.063 seconds / 6.401 GPU-slot work minutes |
| Backend counters | 27065 input / 8960 output; 4/4 coverage |

Causes, using original checker identifiers: `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 2}`.
Does not fit: 0.
Backend terminations: `{"stopped-below-token-limits": 4}`.
Captured HTTP statuses: `{"200": 4}`.
Captured response fields: `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:absent-or-empty": 4}`.
These are observed response fields, not evidence about internal reasoning.
A strict proposal is untested data; refusals are not repaired or counted as passes.

The campaign's changed variable remains the fixed original-fragment construction
instruction in [Decision 0212](../decisions/0212-copy-original-fragments-before-replacement.md).
This resumes the next profile in the frozen policy order. Its own profile,
source excerpts, helper, API binary, inline schema, complete framing,
disabled-thinking request and 300-second total/idle deadlines stay frozen.
Output stays `min(32768, 32768 - rendered input tokens - 32)`.
All 30 original development contexts and these four saved answers reproduce
with the authenticated unchanged helper. No tuning occurs on this checkpoint.

The measured path is Rust context selection → authenticated credential-owning
chat HTTP API binary at `72c002a42957340de0b6fa0eb4d48385db2c7e62` → Rust strict
checker. Native executable admission and original grading controls remain
unavailable. Repository exploration, native effects, project tests, feedback
and retries cannot run; no control is reconstructed or bypassed. No candidate,
hidden or held-out test executes. This is a partial-path measurement and gives
no complete coding-loop pass rate or model qualification.

## Resources, preservation and checks

Two two-task jobs each acquire both shared wrappers, releasing them between
pairs. Low priority, nice 19, 16 GiB RAM, four CPUs and the 900-second job cap
are unchanged. The complete 9,706-file source snapshot passes its off-GPU
preflight and is mounted read-only. All 2,283 repository and 44 private frozen
bindings remain intact. The earlier three capsules and their complete committed
diff receipts authenticate before inference; their every file remains bound.

The controller interval is 416.471 seconds, including queues and the
released pause. Outer authentication, preparation and reporting are outside
that interval. Completed-job work excludes pre-entry validation, wrapper
overhead and queues. GPU kernel time, full reservation duration and total
end-to-end latency remain unmeasured.

Both jobs complete normally. Settings are revoked; API and model owners are
reaped; keys and sockets are absent; four new API tokens are retired; and the
exact-owner process scan is empty. Earlier raw attempts and retirement receipts
are unchanged. No foreign process is signalled, model downloaded or external
service used. No owned GPU reservation remains.

Reporting fast gate, direct Markdown/table rendering, private-path and licence
checks, raw request/response/counter audit, helper reproduction, full prior
capsule authentication and preservation checks pass. No runtime source changes,
so source tests are not rerun; the last source gate remains **FAIL: 1,030 passed,
53 inherited failures, 11 ignored, zero filtered**. The two inherited Python
failures and supply-chain limits stay disclosed in the
[licence evidence](readme-and-dual-licence-2026-11-16.md). No full acceptance gate
or SBOM regeneration runs.

All 512 ledger table lines, 49 historical private-path blobs, original inventory,
Decision 0051, status model and frozen evaluation inputs are preserved. The old
scoreboard is an exact byte prefix. No row, lifecycle, platform, model role,
release or owner-readiness status is promoted. Resume the first absent frozen
pair; finish all 180 attempts before tuning or new capabilities.
