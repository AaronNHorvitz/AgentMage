# T-303/T-306: native endpoint edit decision handoff

Builder record for a non-closing core increment. Independent core review is
required. T-303/T-306 remain open; no row or sub-row is added or closed.

The original native prepared-edit owner can retain an exact endpoint challenge
and pass its protected response to the existing native atomic transaction.
Construction requires live creation ownership and the original full diff, sealed
controlled-write request, catalog, mutable/protected scope and canonical call.
There is no new writer, permission system, grant issuance at construction,
completion verifier or fabricated runtime receipt. The original writer alone
issues and consumes its native grant and returns its actual per-operation chain.

A matching decision spends its handle before observing original run control or
any native dependency. Denial, stop, refusal or unwind cannot reopen it. Invalid
responses leave the original pending decision intact. Original native epoch and
monotonic expiry, preimage/link checks, creation provenance and write lineage
remain owners. Atomic transactions remain non-interruptible after admission.

Verification completed after the shared build reservation became available.
The ordinary complete scoped gate remains **FAIL** (exit 1): 25 target summaries,
1,106 passed, 72 failed, 63 inherited ignored, zero measured and zero filtered.
All 72 failure identities exactly match the unchanged T-301 baseline: 53 host
and 19 Linux failures, with no new or missing identity. No filter, failure
allowlist, new ignore, waiver or green gate claim is used. The ignored tests
remain unrun. T-301.1 stays deferred; no native command admission is changed.

The seven gate commands have exits 0, 0, 0, 0, 0, 0 and 101. Formatting passes
in 1.35 seconds, warnings-denied Clippy in 22.65 seconds and full all-target
source tests finish in 208.99 seconds. Total time including reservations is
234.74 seconds. All five Rust inputs remain unchanged throughout that gate.
Current whitespace, untracked-file, unchanged-review and Markdown checks pass
again after this evidence update. The separate data audit verifies scope,
privacy, unchanged ledger rows, exact failure identities and all eight new
methods; its exit 0 does not turn the failed gate green.

All eight new methods pass in both the focused development run and the ordinary
unfiltered gate. They cover the actual multi-file native transaction and original
receipt chain; denial and four stopped controls; eight altered decisions; seven
native drift/expiry/link cases; 23 changed scope/call/catalog/path cases;
dependency unwind; cancellation after inspection; and returning the same live
owner without retaining an executable pending preview. Creation provenance is
explicitly synthetic. The real writes establish neither native Git creation
nor a model-driven coding workflow.

The earlier focused development run compiled but returned exit 101 (seven pass,
one fail): a test tried to seal an invalid packet with exact mutable/protected
path overlap. The existing packet validator correctly returned
`WorkPacketDenied`. The test now explicitly asserts that refusal and also checks
that the handoff refuses those unsealable bytes without effects. Production
validation and thresholds are unchanged. The corrected focused run exits 0:
eight passed, zero failed/ignored/measured, 332 unrelated tests filtered by this
development-only command. The complete gate above uses no test filter.

Reproduce the focused development and ordinary complete checks with:

```sh
cargo test --locked --offline -p agentmage-platform-linux --lib endpoint_native_edit --no-default-features
python3 -B scripts/local_agent_fast_gate.py --base bc98651997e99c19fd3ddb6b3a58c01a09f21c77 --timeout 600
```

Each Cargo invocation reserves the lane's existing build slot, uses two build
jobs and a private external target directory. The gate reserves its own Cargo
commands and uses four test threads. The existing installed Node runtime and
license are reused; no dependency or runtime is downloaded. Raw logs, source
freeze, result exits and failure comparison remain private. An earlier queued
command from the previous session never started Cargo; this session also
replaced only its own queued command to make locked/offline mode explicit.
Neither queue cancellation establishes a compile or test result.

The final ordinary run's owned temporary namespace retains four tiny inherited
host fixtures (22 entries, 36 file-content bytes); it was not broadly deleted.
No model or GPU process was started. The predecessor's independent PASS report
is retained byte-for-byte; its low findings stay same-row backlog. No row closes
on the builder's tests or on that predecessor's component review.

This is a library decision handoff, without a `RuntimeToolBoundary` implementation
or executable service activation. Coordinator edit/test adaptation, trusted
controlled startup, model-driven coding, durable recovery, approved Git and parent
measurements remain unfinished. There is no new model run or scoreboard entry.
Native test/worker executable admission remains unchanged. Evaluation tasks,
hidden/held-out tests, thresholds, dependencies, status and supply-chain outputs
are unchanged. No owner-ready, package acceptance or release claim is made.
