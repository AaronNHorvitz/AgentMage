# T-302.1: Local chat protocol components, 2026-10-25

Builder evidence for the effect-free T-302.1 slice under Decision 0152.
Independent exact-commit review is pending; every mission row stays unchecked.

## Inherited review and queue

The third T-301 review is retained byte-for-byte as
[the independent report](../reviews/e88a9b42cd35-local-agent-t301-second-repair.txt).
Its SHA-256 is
`17ce746fa31baf621f1bffb9cd9f20a29e1410f48fa74740c7c4db80687a9ae1`.
The verdict is INCONCLUSIVE, with no open high or medium finding. Its tooling
checks reproduced, but the required native Rust verification remains blocked
by T-301.1. The three-strike rule leaves T-301 open; its remaining blocker already
has that sub-row. L2 Python caller selection, L6 contradictory review endings and
the earlier four low findings remain backlog for T-309. No review was edited.

The active queue decomposes T-302 into protocol, transport, local API/UI and a
reserved real-model observation. T-305a adds the owner's vision/speech scope after
research and before coding help. The preserved historical ledger is byte-identical
to the batch base. No historical checkbox or status-model entry changed.

## Implemented scope

The [protocol guide](../guides/local-chat-protocol.md) describes the exact component
and its integration obligations. New original Rust code in the Linux inference
package supplies:

- Explicit HTTP loopback endpoint parsing, a configurable API base path and
  numeric destination. No DNS, credentials in URLs or remote destinations.
- Bounded, single-choice streaming request encoding over explicit conversation
  messages, with no hidden prompt, tools or fallback.
- Incremental UTF-8 event decoding, early text, explicit token-ceiling completion,
  finish-reason-plus-marker termination and poisoned state after any failure.
- Fixed request/event/body/text ceilings, model-ID checks, tool-proposal refusal
  and content-free errors. Output stays untrusted plain text.

The component opens no socket or file, stores no key, launches no process and
activates no route. Existing qualification-bound gateway code is unchanged.
No third-party implementation or documentation was read or copied, no dependency
was added and no model was acquired.

## Verification

The reserved, locked/offline inference-package all-target test command passed:
180 tests, zero failures, eight pre-existing ignored tests and no filtered tests.
Compilation took 16.63 seconds; the library tests took 0.99 seconds. All fourteen
new synthetic protocol tests passed on the first run, including every two-way byte split of a Unicode response and byte-at-a-time
delivery; EOF at every incomplete prefix; model/choice/type/tool refusals; URL
ambiguity; JSON escaping; comment/metadata budgets; and aggregate text limits.
These fixtures are not model inference or a fake-server transport contract test.

Formatting, scoped Markdown lint, retained-report format and byte comparison,
new-text privacy checks and preservation of the historical ledger have passed.
The document/link/policy validator passed for 679 files before this report and
the protocol guide were added. A subsequent whole-tree validator run encountered
a disappearing ignored Cargo incremental-cache lock file while compilation was
active; that run failed. With Cargo idle, the final validator rerun passed for
681 Markdown files and all policy invariants. The final evidence/queue text was
then linted again. This is not the long documentation gate.

The complete touched-crate fast gate ran with the shared build reservation,
two build jobs and four test threads. Cargo was locked/offline; no tests were
filtered or newly ignored. The gate finished **FAIL**, exit 1: its Cargo test
command exited 101. All 53 failed test identities exactly equal the host list in
the [T-301 baseline appendix](t301-baseline-failures-2026-10-25.md), with no new or
missing failed identity. This comparison is not a failure allowlist or a passing
native result. Package totals include all binary and integration targets:

| Package | Passed | Failed | Existing ignored | Filtered |
| --- | --- | --- | --- | --- |
| Inference | 180 | 0 | 8 | 0 |
| Host reverse dependent | 532 | 53 | 11 | 0 |
| Total | 712 | 53 | 19 | 0 |

| Fast-gate command | Exit | Seconds including reservation wait |
| --- | --- | --- |
| Range whitespace | 0 | 0.10 |
| Untracked whitespace | 0 | 0.10 |
| Retained-review format | 0 | 0.10 |
| Markdown lint, four files | 0 | 1.20 |
| Touched-package formatting | 0 | 70.42 |
| All-target warnings-denied Clippy | 0 | 57.84 |
| All-target tests, no fail fast | 101 | 141.31 |

The fourteen new protocol tests passed both in the package-only run and in the
complete gate. No new implementation change was needed after their first run.
The old five standalone process tests also passed, but use their existing
term-match fixture, not this codec or a language model. Private raw logs and the
exact failure-identity comparison remain in lane state. No private build paths
or raw model content are copied into this evidence.

The native blocker was re-observed: the lane maps only the working user, and
the trusted system executables still appear owned by UID 65534. No trust check,
test, threshold, feature default or ignore marker was changed. A failing full
Rust gate must remain FAIL, even when its failures match the inherited baseline.

## Remaining scope

T-302.2 through T-302.4 still require transport, credential ownership, first-run
detection/testing, fake-server contracts, authenticated versioned local HTTP API,
UI, cancellation and one reserved real streaming model conversation. Time,
wire/header/framing limits and permission enforcement belong to transport and
the runtime, not to this body decoder. A compatibility-shaped codec is not model
qualification, an agent loop or a completed task.

No GPU, backend inference, external provider, install kit, public release or
owner-test readiness was exercised. Source work remains in the T-302 batch;
no per-edit SBOM or historical evidence cascade was regenerated, and no hash
binding was narrowed. The unchanged long gate remains scheduled for T-310.
