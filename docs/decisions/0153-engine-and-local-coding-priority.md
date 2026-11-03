# Decision 0153: Engine and Local Coding Priority

| Field | Value |
| --- | --- |
| Status | Accepted under explicit owner instruction |
| Date | 2026-11-02 |
| Authority | Owner engine decision, 2026-10-25; coding directive, 2026-11-02 |
| Scope | Importable Linux engine; measured coding on local models; no public release |

## Decision

1. Build self-contained engine crates with a versioned local HTTP API, an MCP
   server and a CLI over that same API. Build no desktop UI here. A separate
   shell can host panels. The thin editor client remains T-310a, with the
   recorded TypeScript exception and its own security review. This supersedes
   Decision 0151's browser UI choice and conflicting UI requirements in 0152.
2. Complete only the honest inventory and minutes-scale gate needed from T-301.
   Account for every crate and binary, marking works, partial or scaffold with
   evidence and an explicit carry-forward disposition. Delete nothing. T-301.1
   remains a deferred owner-action blocker and blocks no mission work. Use the
   available host and fast gates; report any unavailable host facility plainly.
   T-301.2 records the newly requested engine inventory, not a fourth attempt
   to close the original T-301 review cycle.
3. Implement the minimum T-302 coding connection: configurable endpoint/key,
   credential-owning transport, streaming, model listing and a real recorded
   local-model run through the GPU slot. Keep its local HTTP API; defer its UI.
4. Before tuning the agent, build T-311: at least 30 real coding tasks from owner
   repositories, each with a frozen base, statement and hidden tests. Version
   the suite and provide one command. Separate development and held-out tasks;
   never tune prompts or harness code on held-out tasks or edit hidden tests.
   Establish the current harness baseline first. Record first-attempt pass,
   pass within retry budget, wall time, GPU minutes, tokens and failure causes.
   Preserve raw logs privately when they contain host data. Public benchmark
   additions are optional and require suitable licence/provenance; publish no
   comparisons to other products.
5. Implement T-303 and T-306 together ahead of documents, research and media:
   bounded context from repository mapping/retrieval, strict edits that apply
   cleanly or fail, confined project tests, failure analysis and budgeted retry,
   isolated worktrees, diff inspection and approved apply. Commit only on an
   agent-created branch after approval. Each push needs fresh approval showing
   branch, commits and configured remote. Never push the default branch or
   force-push. Let Git use existing credentials without reading/storing them.
   Measure every meaningful harness change on T-311.
6. Follow with T-312: evaluate available local models fitting the 24 GB device
   on that same suite, using existing inference binaries and the GPU slot.
   Choose models per planner/editor/reviewer role from actual measurements.
   No model download or bundling is authorized. Batch, checkpoint and resume
   GPU work so it does not monopolize the device. The lane retains its four-CPU,
   16 GiB RAM and shared-build limits; each GPU reservation is at most one hour.
7. T-313 may add bounded planner, builder and independent reviewer roles and
   candidate attempts selected by tests. Keep this product capability only if
   it improves the held-out score. It never substitutes for independent row
   acceptance. T-314 keeps actual-run entries in
   docs/evidence/coding-scoreboard.md with date, commit, models, development and
   held-out scores, time, GPU minutes and raw-log references.
8. Then continue documents, research, vision/speech and the preserved remaining
   mission queue. Documents include PDF and common office formats parsed as
   untrusted input without macro or embedded-object execution. Existing
   permission, sandbox, offline/network opt-in, citation and transient-media
   boundaries continue across API, MCP and CLI. Long multi-file changes remain
   unqualified until measured; model size alone is no evidence of coding quality.
9. Prepare carried-forward code for publication under `MIT OR Apache-2.0` at
   import, with original ownership, accurate headers/notices and declared
   permissive dependencies. Inventory current licence discrepancies before
   changing notices. Do not claim existing package metadata establishes this
   readiness. Never name a future product or private sibling in public files.
10. The fixed builder and separate read-only reviewer remain distinct. Only an
    exact-commit independent `Verdict: PASS` without open high/medium findings
    closes a row. Retain reports unchanged after hygiene checks. Low findings
    go to backlog. Three non-PASS reviews or three failed approaches leave the
    original open, record remaining findings and continue dependency-ready work;
    return before T-309. Use at most one level of sub-rows, each with a running,
    measured outcome. Never split a row to evade a failing review or weaken a
    threshold. End the builder session when requesting independent review.

## Scope and truth

This changes work selection, not implementation status. The old ledger, historical
evidence bindings and status model remain intact. Record the protocol component's
existing exact-commit PASS separately; it establishes no transport or coding run.
Preserve supply-chain batching and acceptance checks. No model acquisition,
other-platform work, hosted operation, accounts, billing or public release is
authorized. Prepare owner testing only after the install kit and independent
acceptance; the owner tests locally before any release.
