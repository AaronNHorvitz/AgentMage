# T-303/T-306: endpoint run admission

Non-closing core implementation under Decisions 0153 and 0154. Both parents
remain open; no new sub-row or row closure. Independent core review is required.

The kernel now statically frames endpoint runs with the shared task, scope,
policy, catalog, cursor and resource checks. Endpoint runs carry a selected
address/model and byte-only context ceilings, without a native profile or
tokenizer assertion. The original native request schema and model admission
remain intact. Controlled-write input coverage counts the full prepared body
per context refresh, including encoding overhead.

The existing endpoint model phase can bind to the sealed run, compare selection
and original-context/dispatch ceilings, and retain the exact immutable run
identity. It delegates to the same original-message and streaming phase checks.
No second loop, native profile fabrication, live route, prompt, permission,
grant, write, retry or command is introduced. See the
[crate API guide](../guides/runtime-endpoint-runs.md).

Six new synthetic regression methods cover closed/versioned native and
endpoint decoding; shared task/scope/catalog/cursor/limit refusals; digest-bound
selection, context, task, policy and resource changes; all required
controlled-write budgets including full-body input coverage; phase selection,
context, output and exact-time ceilings; and single dispatch with immutable run
identity retained after the source request is dropped. The exact-ceiling case
uses a synthetic model port and does not execute a native tool or real model.

The first complete gate had an unused `Value` import in the new test file,
which failed Clippy with warnings denied. Tests compiled and ran fully:
76 targets, 3,259 passed, 72 failed, 79 ignored and zero filtered. All six new
methods passed. The failure identities and normalized diagnostics matched the
previous gate. Only the unused import was removed; no test case or assertion
changed. The original source, full failed log and source freeze are retained.

The corrected complete scoped fast gate selected ten crates because contracts
are touched. All seven unmodified commands completed under one shared
reservation in **299.15 seconds**, excluding reservation wait. Formatting passed
in 2.55 seconds and Clippy with warnings denied in 24.99 seconds. Tests took
270.16 seconds: **76 targets, 3,259 passed, 72 failed, 79 ignored, zero filtered**.
Cargo tests returned 101 and the source gate remains **FAIL**.

The audit compares all 43 targets selected by the previous four-crate gate:
all 2,528 previous passing identities remain, plus exactly six new engine
methods. Failure identities, normalized diagnostics and all 71 previous ignores
match. The additional 33 targets have 725 passes, no failures and eight existing
ignores; no earlier same-target result is claimed for that expansion. The 72
failures remain 53 host and 19 Linux library cases under deferred T-301.1
executable ownership. Matching that baseline does not make the gate green.

All nine Rust input hashes remained fixed during the corrected run. The only
source change between runs is the unused import, verified against immutable
initial copies. No source edit or further Cargo run followed. Final light
checks verify exact paths, the unchanged ledger table, unchanged Cargo commands,
source hashes, byte-identical retained review, and no added ignores or private
host material. Private raw logs and reproducible audits are retained. No owned
validation or model job remains.

This does not compose configured endpoints into the full coordinator or enforce
cumulative multi-turn execution. Endpoint turn/result admission, native
edit/test/retry observations, durable ownership, approved Git publication and
parent scoreboard runs remain open. No model/GPU, frozen development or held-out
task, hidden test, scorer, threshold, ignore, supply-chain output, release or
publication behavior is changed.

The predecessor's [independent core review](../reviews/3f573cca96d5-local-coding-endpoint-context.txt)
returned **Verdict: PASS** for exact commit
`3f573cca96d5e3cecbc849a844b3b549d7fe437a` during this session, with no new
findings. The report is retained byte-identically; previous lows remain backlog.
It accepts that context increment only, not this run framing or either parent.
