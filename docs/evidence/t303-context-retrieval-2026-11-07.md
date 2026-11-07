# Task-aware repository context through the existing coordinator

T-303 and T-306 remain open. This non-closing implementation batch adds no row
or sub-row. The [independent Python project review](../reviews/69250fca75df-local-coding-python-project.txt)
has **Verdict: PASS** for exact commit
`69250fca75df16f95e97c71fc003da5fdd6a2600` and is retained byte for byte. Its four
low findings remain backlog. The guide now explicitly states that approved
source edits can alter the in-process test runner's reported counts; independent
grading remains required before scored use. No parent acceptance follows.

## Behavior

Previously, prepared repository files competed for nonessential context space
by their path-hash item IDs. A relevant file could be omitted while unrelated
files filled the packet. The existing context owner now ranks those native
repository sources against the sealed task objective before the same composer
allocates its existing byte, token and message budgets. This behavior is used
by the existing coordinator and its CLI/HTTP callers. No coordinator, model
adapter, tool executor, frontend loop or permission path is added.

Distinct matching path words rank first, then distinct matching excerpt words,
then the digest of the original allocation ID. Matching is case-insensitive
ASCII word overlap, with punctuation and underscores as separators. Query work
is bounded to 128 distinct words, 3–64 bytes each, from the first 8 KiB; source
scans stop at 64 KiB or after finding every query word. Repeated words cannot
increase the score. This is a lexical heuristic, without a claim of semantic,
multilingual or dependency-aware relevance. Untrusted content can influence
selection within this tier, never authority.

Only eligible, nonessential supporting sources with both native repository
identity shapes participate. Ranked allocation IDs change; complete source IDs,
revisions, content hashes, excerpts, sensitivity, admission and the original
source list remain intact. Duplicate original IDs remain available for the
existing composer's refusal. Essential sources, stale/denied sources,
instructions, evidence, corrections and unrelated source namespaces retain
their existing treatment. The same complete inclusion/omission accounting,
retained-constraint checks, native rendered-token preflight and whole-source
reflow still apply. Ranking cannot authorize a read, write, command or grant.
No evidence input set or binding is narrowed.

## Verification

The complete scoped gate runs formatting, warnings-denied all-target Clippy,
and every default-feature host/API test target with locked offline dependencies,
two build jobs and four test threads through the shared build reservation.
No test filter, failure allowlist or new ignored method is used in that gate.
The environmental failure comparison does not make a failed gate green.

| Final check | Observed result |
| --- | --- |
| Formatting | PASS, 0.80 s including reservation |
| Clippy, all targets, warnings denied | PASS, 6.11 s |
| Complete scoped source tests | **FAIL**, 634 passed / 53 failed / 11 existing ignored / 0 filtered; 88.53 s |
| Failure identities | Exact equality with the prior host target's 53 environmental failures; no added or missing host failure |
| New retrieval methods | All five PASS in the complete run |
| Complete API contracts | All 92 methods PASS, no ignored or filtered cases |
| Focused component observation | Five methods PASS; fixture measurement below; deliberately filtered observation, not the full gate |

The Linux platform crate is unchanged and is not selected for direct retesting
by this host/API batch. Its 19 failures from the preceding 72-failure gate are
not relabelled as passes. The scoped gate remains **FAIL**; T-301.1 is unchanged.

Five new methods cover path priority over body overlap, distinct-word scoring,
case/punctuation handling, deterministic ordering under reversed inputs,
unchanged provenance and authority classification, excluded source classes,
duplicate original identity refusal, bounded query work, long-word rejection
and relevant words late in otherwise unrelated source. The integrated method
constructs real context packets through the existing profile/context/composer
pipeline, checking every omitted source against its exact source ID/hash,
unchanged originals, the system role, budget limits and byte-identical packets
under reversed input order.

The integrated fixture supplies five original synthetic source excerpts, totaling
239,608 bytes. Its relevant file has a deliberately late original allocation ID.
An explicitly labelled unranked control uses the same bytes and budgets with a
separate fixture namespace, retaining the original identity ordering. That
control omits the relevant file; the ranked packet retains it and records the
omitted unrelated file. This is a deterministic component comparison, not a
coding task, native run, semantic quality measurement or model score.
The fixture's token counter is explicitly synthetic; production still uses the
unchanged exact native token binding.

The final focused observation retained four of five fixture sources, with one
unrelated source omitted. The packet contains 237,152 input bytes and 59,290
fixture-counter tokens. Total elapsed time for three context constructions
(ranked, reversed-input ranked, unranked control), including their assertions,
was 169,798 microseconds in this observation. This is not inference latency or
an exact native-token measurement. The focused five-method run took 0.43 s.

Changed Markdown, retained-review format and whitespace checks pass. The
unchanged source-mirror policy validator checks the complete tracked/current
source set while excluding ignored build caches; it is not the T-310 long gate.

The first focused test run failed because four unrelated fixtures had identical
bytes, making a test's byte-only inclusion check ambiguous. Those excerpts now
carry distinct file identifiers, and the check requires exact source IDs and
hashes in omission records. Initial focused logs are retained. Final source
hashes are frozen through verification and the publication audit. A later duplicate-ID guard also preserves collisions with already prepared
continuity items. Its first full check exposed a test-helper name shadowed by
a local variable; the helper was renamed without changing assertions. Those
compile failures and the shared reservation wait are retained. No failed
attempt or threshold was removed.

Raw commands, source bindings, complete gate output, focused observations and
publication audit are retained privately under
`<lane-state>/tmp/t303-context-retrieval/`. The audit verifies byte-identical
review retention, unchanged task status/old ledger/current implementation truth,
unchanged evaluation/dependency inputs, source hashes and public hygiene. It is
builder verification, not the independent verdict. Independent core review of
the exact commit is required.

## Remaining work

Configured chat endpoint composition remains open: the native coordinator
requires an exact admitted model profile and exact rendered-token accounting.
A generic chat endpoint does not establish those facts, and this batch invents
neither. Native positive execution still refuses executable trust in this lane;
T-301.1 stays deferred. Service-created worktree composition, durable HTTP
recovery, original-repository apply, approved branch commit/push, MCP and the
measured parent workflow remain open.

No GPU/model process, coding scoreboard update, held-out execution, hidden-test
edit, dependency change, SBOM/evidence regeneration, status-model change,
release or owner-ready declaration occurs. The development campaign remains
required at parent acceptance; held-out tasks run only at row closes.
