# T-303/T-306: consent for the exact live worktree handoff

Base: `4f73d93ea3ac1b685e0932e2688b8d3f839b815c`, under independent review
when this batch began. Its [independent core report](../reviews/4f73d93ea3ac-local-coding-python-initializer-scope.txt)
subsequently arrived with **Verdict: PASS** and no open high or medium finding;
it is retained unchanged. No reviewed commit is amended or moved. This same-row
batch addresses low L2 of the [terminal ownership review](../reviews/1fde68be241f-local-coding-terminal-ownership.txt).
Both parents remain open; independent core review is required.

## Behavior

Each successfully admitted process-held ownership handoff gets a fresh random
identity before pending manual decisions are discarded. Failure to obtain the
complete nonce returns the owner intact. Clones and busy recovery preserve this
identity. A new handoff of the same created root has a different identity, so
consent for an earlier attachment cannot recover the later one.

The control-only owner snapshot shows the root and handoff digests for attached
and pending recovery. It reads no source or activation path and does not lock the
inner owner. Its identity remains readable when that owner is poisoned; recovery
still discards poisoned creation authority and pending approvals. These digests
cannot construct a lease or grant an effect.

Recovery requires both displayed digests and explicit consent. Missing or malformed
bindings refuse. Under the same service lock as ownership publication, both the
attached and pending paths compare the immutable lease identity before extracting,
dropping or recovering anything. Stale consent leaves the current owner intact.
The existing release, exclusivity, poison and fresh original-apply rules remain.

The terminal reads the owner snapshot, requires the exact handoff digest in its
confirmation and sends those same digests once. A successful response must echo
both. Missing identities refuse before confirmation; stale, malformed, refused
or lost responses fail without automatic replay. The [guide](../guides/coding-owned-runtime.md)
documents the required fields. Older unbound recovery requests now refuse.

## Verification

Focused checks ran locked and offline with two build jobs and four test threads
under the shared build reservation, after waiting for capacity. All 49 methods
passed, with no failures or ignores:

| Check | Result | Compilation / test time |
| --- | --- | --- |
| Ownership API unit target | 6 passed, 22 filtered | 9.98 / 0.62 seconds |
| Complete terminal runtime target | 22 passed, none filtered | 4.04 / 3.28 seconds |
| Native ownership/original-apply component tests | 21 passed, 301 filtered | 2.36 / 4.30 seconds |

These are Cargo's reported preparation and test durations, excluding reservation
wait. The actual loopback refusal probe made 16 exchanges in 6,378 microseconds.
The held-root same-worktree reattachment regression took 481,887 microseconds.
The poison regression intentionally panics while the native writer holds the owner;
it then verifies stable consent identity and recovery without creation authority
or pending approvals. The expected panic is not a failed test.

The native tests use synthetic creation provenance with real held directories,
kernel grants and atomic writes. Terminal tests use synthetic HTTP peers and real
controlling terminals; server refusals use actual authenticated loopback sockets.
They do not establish native Git creation, a positive full coding workflow or a
model score. API recovery with successfully created live ownership remains
unobserved in this lane; both attached and pending branches require independent
source review as well as these component contracts.

The complete source gate remains **FAIL**: 937 passed, 72 failed, 63 pre-existing
ignores and zero filtered across 22 target summaries. Its 72 failure identities
exactly match the retained full linked-worker repair baseline: 53 host and 19
Linux failures. All new and expanded methods pass unfiltered. The 63 ignored
identities also match that baseline exactly; this batch changes no ignore.
An initial audit assumption used an older 62-ignore baseline and refused the
result. The additional ignored native linked-inventory test was already present
in the base commit; its entire source file is byte-identical. The audit now
compares both failures and ignores to the later retained baseline. The test gate
itself was neither changed nor rerun to alter this outcome.

Formatting and Clippy with warnings denied pass, taking 1.00 and 7.96 seconds.
The complete test command took 133.43 seconds including compilation. Seven gate
commands totaled 143.79 seconds, excluding reservation wait. The eleven-path
source gate includes the predecessor report. The source freeze binds all six
Rust files; no Rust changes follow that gate.

Final whitespace, Markdown, retained-report format, source-policy and privacy
checks cover the completed evidence. The private request binds the original gate
logs, unchanged failure/ignore identities, source freeze and read-only audit.
The final documentation-only follow-up does not repeat source tests or promote
the complete gate to PASS. Only the independent reviewer can accept this batch.

## Remaining scope

Configured-endpoint admission into the native coordinator needs a deliberate
profile, tokenizer, codec and runtime path; this repair changes none of them.
Native positive execution, durable ownership, new-file transfer, approved
commit/push, MCP and parent measurements remain open. T-301.1 and the prior low
lifecycle/continuation and factory-allocation backlog remain. The predecessor
review's new low L1 also remains backlog: an explicitly selected top-level Python
source can shadow a standard-library module that the runner imports lazily. This
batch changes no Python source admission or test command. Only the unchanged
frozen scorer supplies coding scores. No hidden or held-out
input, threshold, score, dependency, SBOM, lifecycle status or release authority
changes. No model inference or GPU work is required by this batch.
