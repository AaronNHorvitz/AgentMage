# T-303 and T-306: existing native coordinator through HTTP

Builder evidence for a non-closing implementation batch based on
`3bd1720d778bbb77c9b37aa433535fa6d4c9fcba`. Independent core review is required.
Neither parent closes, and no new sub-row is created.

The previous [independent control/repair review](../reviews/3bd1720d778b-local-coding-control-repair.txt)
is retained byte-for-byte. It reports **Verdict: PASS** for that exact commit's
non-closing scope, no open high/medium finding. Its low cancellation-status,
repair call-site guard and synthetic cancellation-collision findings remain
backlog; this batch advances coordinator access rather than another low-only
repair cycle.

## Running path

`serve-runtime-development` launches the existing verified sibling coding host,
authenticates through the existing one-use Linux IPC handshake and projects
`LinuxRuntimeIpcClient` into authenticated HTTP. The existing
`LiveCodingRuntimeService`, host `coding_harness`/`linux_coding_runtime` and kernel
`runtime_loop` remain the owners of execution. The adapter adds no loop, model
controller, command runner, grant, store, model profile or approval responder.

The new terminal commands use that HTTP API for preparation, model disclosure,
canonical events, current exact approvals, durable job control, declarations
and release. API consumers can also page existing terminal artifacts. Startup
inspection is explicit, source/model/profile selection is bound to the existing
marked disposable activation, and the dedicated service excludes the separate
manual effect routes. All runtime routes require the control credential.

[The guide](../guides/coding-runtime-api.md) specifies limits, exact JSON,
commands, process ownership, signal cleanup and scope. Native host configuration
currently selects one of the two existing exact evaluation models. It does not
use or qualify the separate chat endpoint. Arbitrary configured endpoint and
manual strict-edit/test composition remain open; no exact tokenizer/profile
binding is fabricated to bridge that gap.

## Verification

All source edits were completed before the final gate. The lockfile adds only
four already-declared workspace/registry dependency edges; no registry package,
version or checksum changes. Its change correctly selects all eleven workspace
crates rather than only the touched API and Linux crates.

| Check | Observed result |
| --- | --- |
| Complete API/terminal targets | PASS: 79 tests, including 18 library, 10 existing terminal, 5 new terminal-peer, 11 worktree and 35 socket/API cases |
| New absolute frame deadline/closure cases in the full source run | PASS: all 6; partial header/payload, trickle progress, backpressure, expiration, permanent closure and complete-frame reuse |
| Full selected formatting | PASS; 18.68 seconds including reservation wait |
| Full selected all-target Clippy, warnings denied | PASS; 24.54 seconds including reservation wait |
| API with its explicitly selected host features, all-target Clippy | PASS; no warnings |
| Complete selected Rust source targets | **FAIL**; 3,109 passed, exactly 72 retained baseline failures, 78 existing ignored, zero filtered; 296.13 seconds including reservation wait |

Every failed identity equals the unchanged 53-host/19-Linux baseline set, with
no additions or omissions. The complete source gate remains **FAIL**. No test,
threshold, ignore or trust boundary is weakened. The long documentation gate
remains reserved for T-310. Shared capacity was queued without stopping another
worker or raising limits. No source edit follows this source run.

Real authenticated HTTP requests verify control-only access, fixed startup
selection, host refusals, dedicated-service isolation and responsive status
under IPC contention. The private synthetic peer records exact forwarded
requests and proves that no frontend grant or direct cancellation is invented.
Actual terminal executables verify controlling-terminal disclosure and exact
approval, denial, modified/foreign challenge refusal, and revision-bound job
cancellation. These are transport checks, not native execution.

The rebuilt executable's native startup was also attempted against a disposable
synthetic Git repository. It exited 1 in 0.013 seconds with
`linux.repository.git_artifact.invalid`; the sentinel remained unchanged. It
started no inference and did not reach the native HTTP workflow. The private
probe initially expected the older wrapper prefix; that assertion failed while
retaining the actual native refusal. A fresh run checks the exact observed
refusal code, exit and sentinel. Both logs remain retained.

All 30 ordinary development contexts match both historical campaigns through
the rebuilt API in 3.04 seconds. The unchanged synthetic-upstream driver reads
no hidden assertions, runs no grader or inference, and produces **no new score**.
No held-out task was run. Binary bindings are:

| Input | SHA-256 |
| --- | --- |
| API executable | `54bb675101404b0b7299b839a9fde99c8d8040be684ebb501b11801a7a6d8e58` |
| Native host executable | `31d911be8670c6b2f290ad76dffab664ebd40b4fd8ddf5fe9625c850c580cf95` |
| Unchanged development-context driver | `8e2b1b5443c8521d03979ff80e969c335fcf611f71230a4ac7acef9010281366` |

Markdown, exact retained-review bytes, source freeze, ledger status, frozen
input, dependency and public-hygiene audits pass. The unchanged policy checker
passes on a complete tracked/current-source mirror, including its existing
ignored historical link target: 773 Markdown files and all policy invariants.
The direct root scan was stopped after it traversed ignored build objects; that
aborted invocation is not a PASS. The mirror does not modify the checker or any
evidence input. It excludes ignored build caches, so this is explicitly a source
policy check, not the full acceptance documentation gate.

Raw initial/final logs, source bindings, the comparison, HTTP/context records,
startup observations and audit are retained privately under
`tmp/t303-runtime-api/`. The initial manifest alias error, initial Node entry
point error, initial narrow host-feature warnings and aborted root policy scan
are retained. The corrected final manifest and selected host features compile
and lint. No failed invocation is relabelled as a successful test run.

## Limits and next work

The lane still presents system executables and ancestors as overflow-owned.
The native trust boundary is unchanged. Synthetic HTTP and terminal peers are
protocol checks, not genuine model inference, native command termination or
edit/test/retry acceptance. The existing development host retains its own
source and validation restrictions; this transport does not generalize them.

Native positive execution, actual configured-model/manual-tool composition,
durable HTTP restart/recovery, original-checkout apply, approved commit/push,
MCP and measured parent acceptance remain open. A development campaign must run
through the composed executable coding workflow; context equivalence is not a
substitute for a model score. Held-out tasks run only at row closes.

No hidden tests, frozen suite, grading threshold, model weights, public release
or owner-ready claim is changed. No model is downloaded or bundled, and no GPU
process is launched by the builder for protocol verification.
