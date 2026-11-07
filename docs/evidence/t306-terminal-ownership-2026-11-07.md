# T-303/T-306: terminal worktree handoff and recovery

Base: `a8bdb80dc506603b0a8335013e586214b894be69`. Same-row client implementation
under Decisions 0153, 0154 and 0159; both parents remain open. No new sub-row,
model admission or native workflow acceptance is claimed.

## Behavior

The existing terminal client can now hand the manual service's created worktree
to the existing coding coordinator and recover its live owner afterward. It uses
the existing authenticated HTTP ownership routes and introduces no new loop,
effect implementation, authority store or API permission.

`runtime-attach` accepts three absolute private roots, the closed candidate and
scenario labels, and optional relative Python source files for project scenarios.
Local parsing does not inspect files. The source list uses the same canonical
selection constructor as owned attach: at most sixteen existing files, with the
same test-path, hidden-component, duplicate and unsafe-path refusals. The server
still checks actual existence, parsed-source admission, exact creation ownership
and activation. Omission keeps the default source selection.

The client reads the service's held root identity, displays the complete request
and requires `attach <held-root-digest>` in the controlling terminal before sending
the inspection/handoff decision. It sends those same displayed values once.
This transfers process-held ownership and discards pending manual decisions; it
does not approve model disclosure, a tool or original-checkout application.
Request size is checked before confirmation; ordinary HTTP limits remain intact.

`runtime-recover` shows current service state and the owned runtime selection,
or explicitly a null selection when retrying the service's retained pending
owner. It requires `recover worktree owner` in the controlling terminal. Recovery
targets this service's current owner rather than adopting a caller-supplied path.
Busy, unavailable and separate-process runtime states refuse. The API still
requires explicit release of every run and exclusive live ownership. The command
does not cancel or release a run or approve applying changes to the original.

Piped input, mismatched confirmation and malformed state never send an ownership
decision. Ownership requests and lost replies are never automatically retried.
Successful replies must explicitly confirm ownership transition with model/tool
or apply permission still false. All displays retain existing terminal escaping.
The guide documents the private activation and GPU reservation prerequisites.

## Verification

Locked/offline checks used two build jobs and four test threads, after waiting
for the shared reservation. The focused pilot and complete gate each acquired
that reservation. The gate helper verifies its held lock identity and invokes
the unchanged fast-gate planner/executor; Cargo commands have an `env` prefix
only to prevent recursively acquiring the already-held outer reservation.

| Check | Observed result |
| --- | --- |
| Focused terminal ownership methods | 6 passed, 15 filtered, no failures or ignores; 0.48 seconds plus 3.39 seconds compilation |
| Complete local API unit/binary/integration targets | 106 passed, no failures, ignores or filters; 7 target summaries; 18.98 seconds including compilation |
| Formatting / Clippy with warnings denied | PASS; 0.20 / 8.56 seconds |
| Whitespace, untracked checks and four-file Markdown gate | PASS; 0.10 / 0.10 / 1.10 seconds |

The complete selected fast gate is **PASS**, with 29.04 seconds of reported
command time excluding reservation wait. All six new methods also pass in the
unfiltered run. They cover both candidate labels, all three allowed scenarios,
default and canonical explicit sources, complete disclosure before consent,
pipe/denial and malformed selection refusals, the wire ceiling, missing held
identity, attached and pending recovery, busy or remote service refusal, genuine
HTTP 409 responses, malformed success and lost replies without replay. The
synthetic HTTP peer checks client/control credential selection on every request.
Existing model-start, exact tool-consent, event/artifact verification, terminal
backpressure, worktree and chat contracts also pass.

This batch changes only the terminal consumer within the local API crate. The
gate selects that crate's complete targets; it does not select the unchanged host
or Linux test suites. Their inherited 53 host and 19 Linux failures remain
unresolved and are not repaired or skipped by this green client gate. No ignore,
failure allowlist or threshold changes. The three Rust input hashes remain
unchanged from the snapshot recorded before reserved compilation. Final report
lint, complete source-policy validation, exact scope and ledger preservation are
recorded with the private request and raw logs. No SBOM is regenerated.

## Limits

The terminal tests use synthetic HTTP peers and actual controlling terminals.
They establish request, disclosure, consent and refusal behavior, not native Git
creation, a positive coordinator fail/repair workflow, inference or a coding score.
The existing T-301.1 executable-ownership blocker still prevents native acceptance.
The [source-selection core review](../reviews/a8bdb80dc506-local-coding-explicit-python-sources.txt)
arrived after the terminal implementation commit. It has **Verdict: PASS** for
exact commit `a8bdb80dc506603b0a8335013e586214b894be69`, with no open high or medium
finding, and is retained unchanged. Its low L1 records that a writable package
initializer can use the discovery hook to hide root-layout tests. Prior L1 is
therefore partly addressed; prior L2 is fixed. This remaining low is backlog,
not a new row, and must be resolved before citing root-layout tests as coding
success evidence. Neither that review nor this client batch closes T-303 or T-306.

Retaining the review and recording this limit changes only documentation after
the terminal implementation commit. The unchanged source snapshot and its
106-test result remain bound to that implementation. The final documentation
follow-up has its own selected gate and complete policy check; no source test
is inferred from documentation lint.

Arbitrary configured coding endpoint composition, durable ownership, new-file
transfer, approved commit/push, MCP and parent measurements remain open. Native
candidate dispatch already uses exact rendered-token binding; this client change
adds no configured-endpoint admission or tokenizer claim. Prior lifecycle and
factory-allocation low findings remain backlog. No evaluation task, hidden or
held-out input, score, threshold, dependency, SBOM, lifecycle or release authority
changes.
