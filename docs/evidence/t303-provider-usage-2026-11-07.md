# T-303: Optional endpoint token reports, 2026-11-07

Builder increment based on `b6d2829368e7e01716f4fc78c84e1154a62ed3f8`.
T-303 and T-306 remain open; independent core review is required. No new row,
coordinator admission, coding score or real-model observation is claimed.

The configured endpoint previously discarded usage-only events. Add an explicit
opt-in through the existing codec, credential-owning transport, chat and strict
coding proposal APIs, including selected-workspace and failed-test repair
requests. The old request and response shapes remain the default. The
[API guide](../guides/local-chat-api.md) documents the additional field and its
provenance. No prompt, decoding profile, model selection or grant is changed.

The transport returns three optional provider-reported counts only after valid
finish, completion marker, HTTP framing and physical EOF. Validate unsigned
32-bit integers, checked totals, ordering, uniqueness and the requested output
ceiling. An omitted report remains unavailable, never zero. A zero report is
retained only as the provider's claim. Refuse malformed reports without a
fallback request. Cancellation, late bytes, missing completion and other
transport failures publish no completed usage observation.

For strict coding proposals, a rejected edit still consumed a completed
inference: preserve its valid counts on the terminal rejection, including a
length rejection. Counts grant no edit or test authority. Repair preflight
encodes the same complete messages and usage option before consuming the failure
receipt; all previous disclosure, single-use, workspace and output boundaries
remain in force. This is the existing proposal path, not a second agent loop.

The [recording client](../guides/local-chat-measurement.md) supports
`--include-usage`. Its version 2 report records per-turn counts and sums both
turns only if both supplied counts after normal completion. Missing counts stay
null, failed turns contribute no counts, and invalid envelopes fail the
measurement. Without the flag, its version 1 report and request remain available.
The Python client still does not guarantee erasure of earlier credential copies.

These reports are untrusted, post-dispatch server statements. They are not exact
pre-dispatch tokenizer counts, cache/reasoning subtotals, served context,
verified weights, local memory observations, GPU minutes or model qualification.
They cannot satisfy the current native coordinator's exact-profile admission
contract. Connecting that endpoint safely remains open; neither its profile nor
any native observation has been fabricated to bridge the gap.

## Verification

The first run below predates the separately documented
[creation-publication repair](t306-create-publication-repair-2026-11-07.md).
It is historical for the combined patch; the final shared rerun below includes
that repair. The seven usage Rust files and two Python files are
unchanged by that repair.

The initial usage-only build reservation ran the focused targets and complete source gate
under one verified lock, with two build jobs and four test threads. Source was
frozen before checks; all seven Rust and two Python input hashes remain
unchanged. No build or test ran outside the reservation except the lightweight
Python client checks. An earlier redundant compile-only waiter was terminated
before Cargo started; it produced no compile/test result.

Test-target times below are execution times reported by the test harness,
excluding compilation. The complete fast-gate wall time includes compilation
and all eight commands.

| Check | Observed result |
| --- | --- |
| Codec target | 22 passed, 0 failed/ignored, 160 filtered; 0.28 seconds |
| Complete transport target | 18 passed, 0 failed/ignored/filtered; 0.46 seconds |
| Repair target | 3 passed, 0 failed/ignored, 25 filtered; 0.01 seconds |
| Complete API contract target | 37 passed, 0 failed/ignored/filtered; 4.38 seconds |
| Recording client | 16 passed, 0 failed/skipped; 0.518 seconds |
| Formatting / Clippy with warnings denied | PASS; 0.90 / 12.02 seconds |
| Full unfiltered Rust run, 26 targets | **FAIL**: 876 passed, 53 failed, 19 pre-existing ignored, 0 filtered |
| Complete fast gate, 8 commands | **FAIL**; 135.69 seconds excluding reservation wait |

All six new Rust methods pass in the unfiltered run. The expanded existing
repair checks exercise legacy and opted-in requests with both accepted and
rejected edits. The three new Python methods cover strict envelopes, missing
counts and failure of a later turn. Codec cases span every two-fragment split,
finish-chunk reports, invalid numeric types, overflow, mismatched sums, ordering,
duplicates, missing completion and late bytes. Socket cases cover exact approval
before I/O, unsupported requests without fallback, cancellation, omitted reports,
length finishes, malformed reports and rejected strict edits. These are actual
loopback connections to synthetic servers, not inference measurements.

The 53 failing test identities and normalized diagnostics exactly match the
host failures retained with the base commit. Eleven ignored identities match
that baseline's shared targets; the other eight belong to inference targets not
selected by that earlier gate. All four files defining those eight are
byte-identical to the base, including their existing ignore annotations. No
ignore was added. The initial audit wrongly expected all nineteen in the older
host selection; its refusal prompted this source-backed classification. The
original full gate remains
red under deferred T-301.1. No test, threshold, trust check or failure handling
was weakened. Focused passes do not establish native sandbox/model acceptance.

The final combined reservation also tests the creation-publication repair.
All nine Rust and two Python source hashes stayed fixed. Its two publication
components pass (601 filtered in that focus); codec 22, complete transport 18,
repair 3 and complete API 37 also pass with no failures or ignores. The API test
execution takes 4.43 seconds. All eight newly added Rust methods pass again in
the full **unfiltered** run: 26 targets, **878 passed / 53 failed / 19 existing
ignored / 0 filtered**. Both expanded repair cases and all 16 Python checks pass.
Formatting takes 0.90 seconds, Clippy with warnings denied 6.76 seconds and
complete Rust tests 91.13 seconds. All nine fast-gate commands total 101.09
seconds, excluding reservation wait; the full result remains **FAIL**.

The final audit confirms identical host failure identities and diagnostics,
the eleven shared and eight source-verified inference ignores, the creation
entry's guarded dispatch and completion ordering, the byte-identical review,
privacy, source hashes and unchanged ledger statuses. The earlier ignored-test
classification refusal and both source runs remain retained. A prose-only
waiter was also cancelled before it started when the review finding arrived;
it is not a product test result.

Final documentation and policy verification is recorded in the private run logs;
it does not change the full source gate's FAIL result. Raw logs, frozen input
hashes, selected commands and read-only audit helpers remain in private lane
storage for independent reproduction.

No evaluation descriptor, hidden test, held-out input, prompt, threshold,
dependency, lockfile, status model or supply-chain output is changed. The long
acceptance gate remains T-310 work. Existing source and evidence bindings are
preserved. No model download, push, public release or owner-ready claim.
