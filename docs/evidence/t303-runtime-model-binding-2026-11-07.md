# T-303/T-306: distinct runtime model bindings and token provenance

Date: 2026-11-07. Non-closing core batch under Decisions 0153 and 0154.
Both parents remain open; independent exact-commit review is required.

## Change

The shared contracts now distinguish a complete exact model profile from a
person-selected endpoint and model name. These serializable inspection types
have no deserializer and grant no authority. An exact profile still needs the
existing native admission, serving and dispatch verification; an enum variant
does not establish that verification. Endpoint names do not attest to weights,
tokenizer, serving process or hardware.

The existing runtime model port exposes this distinction. Its default preserves
native ports. Coordinator construction requires both the exact-profile variant
matching the complete request tuple and the original native profile comparison.
A selected endpoint refuses with the existing model-binding error before any
context/model/tool work, without falling back to a native profile accessor.
An override cannot use the new descriptor to bypass the old comparison.

The endpoint transport projects its immutable prepared request into the shared
selected-endpoint descriptor. Snapshot, workspace and failure-repair proposal
events expose the same descriptor in context and terminal events. The full
prepared-body binding remains unchanged and still identifies the exact body,
usage request and limits; the smaller identity projection cannot replace it.

Completed responses additionally expose shared selected-endpoint token
accounting. Optional provider counts retain their original checked sum, output
ceiling, opt-in and stream validation. The transport's existing `ChatUsage`
name now re-exports the shared count type; wire parsing and JSON fields are
unchanged. Missing reports and reported zero counts stay distinct. Verified
input/output counts always remain null, and no exact prompt capacity is inferred.
Partial, malformed and cancelled responses supply no completed accounting.
Completed length finishes and invalid edits retain inference accounting while
still refusing a candidate. No request or prompt bytes change.

## Verification

The corrected complete source gate selected all ten affected workspace crates
and ran six commands under one shared build reservation in **312.57 seconds**,
excluding queue time. Whitespace, untracked-file checks and touched-document
lint pass. Formatting passes in 2.65 seconds and Clippy with warnings denied
passes in 17.08 seconds. Complete all-target tests run in 291.34 seconds:
**76 targets, 3,227 passed, 72 failed, 79 ignored, 0 filtered**. The gate remains
**FAIL**: Cargo tests return 101 and the gate returns 1. No filter, failure
allowlist, new skip or gate rewrite is used.

All four new regressions pass unfiltered. They distinguish absent, zero and
nonzero provider reports without verified counts; reject an endpoint selection
before any native-profile fallback; retain the whole native tuple comparison
and successful synthetic completion; and refuse an override that contradicts
the original native profile accessor. Strengthened real-loopback transport,
API and synthetic repair tests check the shared identity and accounting on
completed proposals, absent/invalid usage, invalid edits, length finishes,
truncation, late bytes and cancellation. No native inference is claimed.

The 53 host failures match the retained endpoint-binding source witness. The
19 Linux-platform failures match the retained created-file transfer source
witness. The combined 72 failure identities and normalized diagnostics match
exactly. Existing host/inference/platform ignore identities also match. The
larger crate selection adds eight existing engine helper/campaign ignores;
their six complete defining files are byte-identical to the immutable batch
base, with the original ignore attributes. This accounts for all 79 ignores.
The first bookkeeping audit omitted those eight engine helpers and failed;
the corrected audit proves their unchanged source rather than changing a test
or converting the failing gate into a pass.

The initial reserved attempt failed compilation because a new regression test
used the nonexistent terminal state `Succeeded`. The assertion now uses the
existing `Success` state. Both original compile failures and the original source
freeze are retained. No assertion or threshold was removed. The complete
corrected gate above ran after this single-word correction.

Raw command logs, source hashes, plans, comparison helpers and review-input
manifests remain private in lane state. Final lightweight checks verify the
same ten Rust source hashes and Cargo selection, exact fourteen-path scope,
unchanged ledger rows, byte-identical retained review, whitespace and privacy
after these result paragraphs were added. They run no build or full suite.

The preceding endpoint-binding [independent review](../reviews/0e5ed222cc36-local-coding-endpoint-binding.txt)
is retained byte-identically, with **Verdict: PASS** for exact commit
`0e5ed222cc36c44bc2d62a8030c18a22b6e3444d` and no open high/medium finding.
Its low L1 (API fixture raw-body digest checks) and L2 (preparing transport
before consuming repair feedback) remain backlog in these same parent rows.
That review accepts the preceding increment only, not this new batch.

## Limits

This is a shared identity/accounting boundary and executable proposal integration,
not configured-endpoint admission to the coordinator. Its current run request,
context, model result, native profile accessor and exact token/resource contracts
remain native. Endpoint-specific versions and their integration into the same
coordinator remain necessary before an endpoint can drive tools. No second agent
loop or provider-to-native adapter is introduced.

Synthetic fixture execution is separate from native model/worktree execution.
No GPU work, real inference, generated-task score, held-out run or scoreboard
entry is claimed. Native executable trust retains the deferred host-ownership
blocker. No source scope, grant, sandbox, credential, hidden test, frozen suite,
threshold, prompt, scorer, SBOM, status promotion or release boundary changes.
Durable worktree ownership, native edit/test/retry measurements, approved Git
publication and parent closure remain open.
