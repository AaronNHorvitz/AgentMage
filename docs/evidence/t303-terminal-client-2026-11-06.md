# T-303 and T-306: terminal coding client

This is a non-closing implementation batch under Decisions 0153 and 0154.
T-303 and T-306 remain open. The preceding edit primitive has an independent
[exact-commit core PASS](../reviews/d41c21eea0ec-local-coding-reviewed-edits.txt)
at `d41c21eea0ecdf4fa5ca9fcf0ac10a969c898b2e`; that verdict does not cover
this client or close either parent. The new client requires its own separate
read-only core review. No new task row or per-finding split is introduced.

## Delivered path

The `agentmage-code` binary is a thin client of the existing version-1 local
HTTP API. It provides explicit source selection/revocation, read/search,
streaming strict proposals, preparation, review, exact apply/deny, status and
cancellation. The endpoint and key still belong to the existing API settings
controls. The client reuses the engine, protected credential loader and native write
driver; it has no direct filesystem writer or model transport.
The [terminal guide](../guides/coding-terminal.md) gives exact commands.

Read-only selection has no disclosure or write consent. `select-model` displays
the root, complete path list, endpoint and settings revision on the controlling
terminal, then requires `share <revision>`. Its selection includes the optional
`settings_revision` and `settings_endpoint` fields, checked under the API
operation lease. The exact endpoint also prevents a restarted service from
reusing a revision for a different destination. A change since
display returns `code.workspace.settings-stale` without replacing scope or
pending preview. Existing callers may omit the fields, preserving the old API
contract and their responsibility for trustworthy endpoint display.

Only a complete successful checked proposal triggers preparation. The client
retains the exact context source hashes, task, message ceiling and complete
answer, then calls the existing strict checker/native preview API. Malformed,
partial, oversized, truncated and failed streams cannot prepare. Read-only
status polling handles the documented proposal-terminal/lease-release race;
generation, preparation and apply are never automatically retried.

For apply, a native metadata-only identity check matches the person's supplied
absolute root to the pending preview's canonical path/device/inode hash. The
controlling terminal shows that root, the complete pending preview and risks,
and requires `apply <preview-sha256>`. It checks the root again, then sends one
exact control request. A missing terminal, piped stdin, wrong confirmation or wrong
root refuses approval. The API independently checks revision/hash bindings and
consumes its kernel grant once. Noncommitted native outcomes fail the client
exit status. Denial remains inert.

The client reuses the protected token loader. Model requests receive neither
token. Fixed routes and numeric loopback sockets prevent model-selected routes,
DNS, proxies or redirects. It bounds requests at 256 KiB, replies at 4 MiB,
headers at 8 KiB, streaming events at 8,192, connect/write at two seconds each
and total response reads at 180 seconds. It rejects ambiguous HTTP framing,
compression, chunking, mismatched lengths and unfinished NDJSON. JSON and ASCII
Unicode escaping prevent terminal controls or bidi text in model/repository
bytes from becoming terminal instructions.

## Verification

All Rust checks use the shared build reservation, locked offline dependencies,
two build jobs and four test threads. The final per-change gate uses
`d41c21eea0ecdf4fa5ca9fcf0ac10a969c898b2e` as its base and the existing shared
Node runtime. No host-trust workaround, filtered test, new ignore or allowlist
is used.

| Check | Observed result |
| --- | --- |
| Native `cargo test -p agentmage-platform-linux --test coding_write` | 6 passed, zero failed/ignored/filtered |
| API `cargo test -p agentmage-local-api --lib --test coding_cli --test local_chat_api` | 2 library, 5 executable/PTY and 33 API tests passed; zero failed/ignored/filtered |
| Selected host/API/Linux formatting and all-target Clippy with warnings denied | PASS |
| Full ordinary per-change gate | FAIL: 771 passed, 72 failed, 62 existing ignored, zero filtered; test command 77.81 seconds |
| Failure identity comparison | All 72 exactly match the original T-301 baseline: 53 host and 19 Linux; no new or missing identity |
| Cargo metadata | Both API and terminal executables registered; both API integration test targets accounted for |
| Changed Markdown and retained review format | PASS |
| Documentation policy | 754 Markdown files and all policy invariants PASS |
| Publication audit | PASS: review bytes, source/binary/driver hashes, 209 retained files, original campaign bindings, native receipt chains, unchanged frozen inputs, ledger/checkbox preservation, privacy and local links |

The executable tests traverse real sockets, controlling pseudo-terminals, the
API, kernel grants and native file replacements in disposable synthetic
workspaces. They prove literal selected read/search, invalid path refusal,
revocation, exact typed approval and inert denial, refusal of missing/piped/wrong
confirmation, wrong roots, consumed previews, revocation while displayed,
linked credential files and incomplete model output. API regression checks
exercise mismatched settings revisions and exact endpoints. Native tests also
refuse replacement roots and symlink aliases. Wire tests cover redirects,
truncation, mismatched lengths, duplicate headers, compression/chunking,
oversized declarations and unfinished NDJSON. Terminal escaping round-trips
non-ASCII text, surrogate pairs and bidi controls without emitting them raw.

A reserved external-process check ran the actual CLI and API against a
synthetic upstream. All **30/30 development contexts** exactly match both
historical local-model campaigns, including serialized upstream messages.
It then replayed all **3/3 previously accepted development replies** through
streaming, preview preparation, fresh typed PTY approval and native writes.
Resulting bytes exactly match those historical candidates; unselected sentinel
files stay unchanged, and kernel receipts run proposed → approved → applied →
verified. A separate Python audit recomputes every receipt digest and chain.
The complete check took **4.251 seconds**. This is recorded-reply replay and
synthetic context verification, **not inference, grading or a new model score**.
No held-out task, hidden assertion or grader runs. The scoreboard is
unchanged.

The retained development summary SHA-256 is
`f66e5c63185b6e442f56f3561161c354c795f663072d07aa75d7054130efbc27`;
its 209-file manifest SHA-256 is
`d1349f3410d37ab8a77ca0b70b04e6e87ef4228e19a87ae1758ef143b604e1bd`.
The tested API digest is
`9d820a5dc4d21f9cd730b313b7bab272290f303513d7ffb12432c64c937218e3`;
the tested terminal client digest is
`474e9d07c66aadebba86adc9544346517b3b5a5883588824de4e3d55239b69ef`.
The private review request identifies retained binaries, exact commands,
source/context/answer logs and the driver. Raw inputs and terminal transcripts
remain private because they contain temporary host paths.

Initial checks caught a literal bidi control in the safety-test fixture, a
client request using the ordinary token on the existing control-only settings
route, and a redundant unit expression rejected by Clippy. The fixture now
uses a visible escape, settings inspection uses the existing control authority,
and the expression is removed. The first full gate was explicitly interrupted
through its supervisor after its Clippy failure; it is not counted as a passing
or complete test run. The final full run above completed. Compiler, diagnostic,
interrupted-gate and final logs are retained. No check or threshold was weakened.

## Limits and review disposition

The native worktree and command adapters remain unchanged. This lane still
observes overflow ownership for system executables and their ancestors; their
root-trust checks refuse execution. No copied executable, ownership change,
trust exception or alternate unsandboxed runner is introduced.

This client writes explicitly selected workspace files, not an agent-created
isolated worktree. Project tests, bounded test/fix retries, repository map
exploration, durable run records/resume/replay, Git apply/commit/push and MCP
remain open. The initial selection is still at most 16 exact paths and 128 KiB
of source, as before. The client does not start a model process, take a GPU
reservation, download a model or enable a production platform profile.

Previews/receipts are still process-only. A lost reply can follow file effects;
no automatic retry, rollback or durable recovery is claimed. Manual recovery
now explicitly includes inspection of `.agentmage-write-*.tmp` files left by an
interrupted native transaction. A complete line-oriented diff and the prior
low cancellation-coverage, conservative effect-label, lease-release,
snapshot-validation-order and campaign grade/candidate-binding findings remain
backlog. Full coding-loop and parent acceptance require separate evidence.

No frozen task, hidden test, grader, prompt, dependency, threshold or status
model is changed. No held-out task is used for tuning or a non-closing run.
Historical scoreboards remain historical; synthetic transport/replay checks
cannot establish a new local-model score. No public release or owner-ready
claim is made.
