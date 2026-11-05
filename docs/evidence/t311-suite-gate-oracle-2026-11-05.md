# T-311.17 independent process and file checks, 2026-11-05

The same expanded suite batch remains open under
[Decision 0154](../decisions/0154-local-coding-delivery-refocus.md). No row or
review checkpoint is added. Parent T-311, generated scoring and the unchanged
harness baseline remain open. These are evaluator measurements, not model scores.

## Change

Oracle v32 adds the two remaining development adapters, dev-003 and dev-004.
The complete 23-method original module and nine-method original selection execute
unchanged in the trusted interpreter. Candidate code executes in separate
networkless mounts with no procfs, hidden tests, result directory or capabilities.
A fixed x86-64 syscall filter denies process inspection and x32 calls. Further
user namespaces stay disabled. Temporary fixtures share only quota-backed data.

The [guide](../guides/coding-evaluation-oracle.md) describes the PID boundary:
trusted bootstrap pauses before candidate import, kernel ancestry and pidfds
establish ownership, launcher groups are terminated and reaped, and the actual
confined child becomes the trusted parent's native waitable child. The original
assertions inspect real process states and reservation locks. Direct zombie-only
helper checks replay finite wait and signal observations against the actual
unreaped child, preserving the original frozen signal mocks and assertions.

Direct calls use fresh interpreters and bounded typed values and exception
categories. Module-global continuity, authenticated diagnostic history, running
`observe_exit` inputs, exact signal enum classes, exception messages and
unasserted statement clauses remain ungraded. No single aggregate candidate exit
exists for these multiprocess tests; reports use null with an explicit scope.
Neither candidate summary text nor patched candidate-side assertions supplies
trusted test counts. The existing host startup-failure hook remains exercised:
its fixed false command really runs, and zero-assertion refusal reports cannot
validate a historical regression. The shutdown frame there is a compatibility
marker recorded by the supervisor, not a candidate reply.

The hard process limit is 16, with inherited 512 MiB address space and 60 CPU
seconds per process; the case execution deadline stays 90 seconds. Shared private
fixtures have 64 MiB/8,192 inodes and the case mount 128 MiB/32,768 inodes.
Anonymous mount release follows worker termination. No lane ceiling is raised.
Inherited counters cover the whole lane, including trusted work; their memory
peak is a lifetime peak, not a task or GPU peak. Whole-suite admission remains open.

## Verification

The final sequence ran in one shared build reservation. Its source/test snapshots
match at both boundaries and still match the final tree. The audit binds all
commands, timings, raw logs and reports to that stable source. Times exclude
reservation waits. Every hidden byte, selection, statement, pin and threshold
remains unchanged.

| Check | Result | Command seconds |
| --- | --- | --- |
| Ordinary scoped fast gate | PASS, 127 methods | 335.078873 |
| Source adapter module | PASS, 16 methods | 9.443095 |
| Development evaluator, collection and suite | PASS, 73 methods | 19.264384 |
| Fixture and resource modules | PASS, 16 methods | 5.554726 |
| Historical-control module | PASS, 11 methods | 1.824733 |
| Historical and independent Rust modules | PASS, 16 methods | 54.895587 |
| Boundary module | PASS, 9 methods | 0.586139 |
| All historical development controls | PASS, 30 base/reference pairs; 89 reference methods | 53.425455 |
| All independent development controls | PASS, 30 base/reference pairs; 89 reference methods; 1,029 calls | 79.538968 |
| Frozen split input check | PASS, 40 definitions; zero held-out assertions | 5.395419 |

All 268 unit/boundary methods pass. Independent development coverage now
includes all 30 tasks and all 89 frozen reference methods, with 1,029 reference
calls. The new eleven-method module checks both control pairs, assertion/summary
forgery, early exit and malformed output, real process cleanup, missing
supervision, syscall and mount privacy, capability removal, user-namespace
refusal, closed value types, foreign-signal refusal and kernel-ancestry refusal.
The original 128 MiB cleanup-memory bound remains unchanged and passes.

The audit verifies 120 reports and 300 log bindings, inherited resource
observations with zero observed OOM kills, all old control profiles and frozen
inputs, unchanged ordinary worker/tests and older ledger rows, and public-file
hygiene. The ordinary worker test module overlaps held-out selections and is
still deferred intact to row closing. Post-write Markdown and whitespace checks
run separately. No held-out assertion or model attempt occurs.

Private artifact commitments:

- Final command manifest: `7b29b3afe8aa98a1a52d2596195ae8e318724accee87693d23bf5c8e3eb9d778`.
- Matching source snapshot: `2aaec47352ea53f85806d2c8b46b88f5f8489c79288b9b0089608ff641f52221`.
- Independent control summary: `b7241134dcef654ab45c3ebd2d2fe4824338649adfca4ff29d18370572312a59`.

## Retained failures and remaining work

Exploratory failures remain private: a scratch script shadowed a standard module;
launcher PID/signal translation and signal-enum encoding were wrong; bootstrap
adoption was incomplete; a diagnostic pipe read blocked until the unchanged
90-second case deadline. That owned probe was interrupted after recording its
timeout, and all owned workers exited. These failed probes are not validation.
An effect-free launcher experiment was also rejected by the installed tool.

The first complete gate ran 125 methods with three failures: the new namespace
probe assumed EPERM instead of the observed quota-denial errno ENOSPC; an ordinary
summary assertion still required 28 adapters; and the new path bypassed the
existing host launch-failure injection. The correction requires 30 adapters,
restores the failure path and preserves every original failure assertion. No
frozen test or threshold changed. Failed logs and the successful preliminary
control run remain retained.

An earlier verification passed all 267 then-selected unit methods, then its
historical verifier read an empty PID file between creation and its first
content write. The ValueError disrupted later subcases. The failed verifier
remains retained; every historical pair was repeated together in a new output,
with matching source bytes and original unit-check provenance. No hidden test,
threshold or source byte changed for that repeat. This race and the previously
documented process race remain failed attempts with raw logs; neither earns a
pass or is removed from a model-score denominator.

Before publication, code inspection found that trusted fixture Git calls could
execute untrusted repository configuration outside the candidate boundary. The
Git filesystem-monitor regression proves the bypass variant sees the synthetic
assertion mount and fails, while the confined variant passes. The driver now
confines Git as well as Python, refusing other fixture executables. The final
sequence above verifies those changed bytes and all 268 current methods; the
earlier passing source is not used as verification of this boundary repair.

Held-out independent grading and final coverage/resource admission remain in
T-311.17. The frozen held-out suite measures a narrow evaluation-protocol domain;
it is not general coding qualification. No harness/prompt tuning, GPU/model run,
download, supply-chain cascade, long acceptance gate, release, owner-ready
statement or independent closure occurs here. T-311.18 still owns the baseline.
