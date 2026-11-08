# T-303/T-306: retained endpoint owner through authenticated HTTP

Date: 2026-11-08. Tier: core. Immutable predecessor:
`77eeb69607dfe2147b9275c9b7b0a40d7b4aa30b`.
Both mission parents remain open; no row or sub-row is added or closed.
The predecessor's [independent core review](../reviews/77eeb69607df-local-coding-shared-config.txt)
has **Verdict: PASS**, with no findings, for that exact commit. The report is
retained unchanged. This HTTP increment requires its own independent core review.

## Change

A dedicated trusted-host library entry point now projects one retained
configured-endpoint coordinator through the ordinary local HTTP parser,
authentication and four bounded connection workers. It generates fresh random
run/session identities and calls its trusted factory once. Any replacement
identity refuses before admission. The factory supplies original approved
configuration, already consented snapshots and original permission, tool,
verifier and clock ports. Wire input cannot reconstruct any of these objects.
Temporary source snapshots drop after the context has captured them.

The service retains one coordinator and fixed configuration for its whole
lifetime. It exposes only request inspection, advancement, canonical event
paging, cancellation and terminal/fault release. Manual settings, workspace,
repository and native-host routes are unavailable. Scope or configuration
changes require cancellation and a new service with fresh identities and
consent. Every advance requires current explicit model-context consent through
the control credential; refusal leaves the original challenge intact. Every
operation other than request inspection binds the exact service-issued run
identity and original request digest.

Exact approval validation, cumulative budgets, context preparation, dispatch,
verification, terminal publication, release and spent identity remain in the
original owner/coordinator. No native model profile is invented. An independent
atomic cancellation latch remains available while an advancement holds the
owner. It cannot be cleared or authorize disclosure. Advancing a cancelled run
needs no disclosure consent because the kernel stops before dispatch. Cancellation borrows the original coordinator correlation, including before its
first event; it never recreates a derived identity from the run id. Shutdown
uses the same probe and joins every owned worker. The ordinary service now
shares those same HTTP worker mechanics; no second agent loop or detached
worker is introduced.

Events are paged one original canonical event at a time. The index identifies
an immutable in-process prefix, not a durable recovery cursor. Concurrent
advancement, event reads and release return busy while the owner is locked.
Cancellation responses acknowledge requests only. Event export precedes
release or shutdown. A dropped response does not cause an automatic retry.

## Verification scope

Five new methods use actual HTTP and endpoint sockets with synthetic tools and
permission observations. Their verifier deliberately rejects completion; no
software-task success, native filesystem/grant execution or model score is
claimed. They cover authentication and authority, fixed configuration, fresh
identities, pre-dispatch consent/binding refusal, protected approval retention,
malformed input, concurrent ownership, original canonical events, complete
endpoint terminal framing, release/spent identity and cancellation/shutdown.
The initial development compile found an unsupported test digest formatter;
it was replaced with ordinary hexadecimal byte formatting. The first focused
execution passed approval/terminal retention but caught a cancellation identity
error: using the run id as correlation caused refusal. The repair exposes the
coordinator's original correlation through its retained owner and uses that
identity, including before the first event. No cancellation check is weakened.
The repaired focused run passes all five methods. Earlier focused runs are
development checks only; final unfiltered verification follows.

## Limits

This is a library service entry point. No executable activates it yet. Actual
native filesystem/grant composition remains open: the existing production
boundary still requires its native model profile and is not bypassed here.
CLI/MCP integration, edit/test/retry, durable recovery, approved Git, parent
measurements and both T-303/T-306 remain open. A trusted factory must supply
real confined ports before any native workflow can be claimed. The adapter
adds no source-read, tool, command, write, network or completion authority.

No evaluation/hidden/held-out task, scorer, threshold, ignore, dependency,
native admission, model, GPU, scoreboard, SBOM, status or release change.
Existing evidence remains bound to its historical inputs. No supply-chain
regeneration occurs during this source batch. There is no independent
self-approval or row closure.

## Final unfiltered verification

The complete ordinary gate covers all four affected crates and 44 targets:
**2,611 passed, 72 unchanged failures, 71 unchanged ignores, zero filtered**.
All 2,606 prior passes remain; exactly the five new socket methods pass.
Every original failed/ignored identity and normalized failure diagnostic is
unchanged. No target was omitted for its result. The source gate remains
**FAIL** on the deferred T-301.1 native-manifest blocker; nothing is waived.
Formatting and strict Clippy pass. All five changed Rust files stayed
hash-identical throughout the gate.

Seven commands took 342.85 seconds including shared reservations
(0.10, 0.10, 0.10, 0.05, 2.15, 23.01, 317.34).
The initial documentation command returned 127 because the predecessor's
scratch Node path had disappeared. The already-installed durable tooling
copy was found locally; no download was needed. The exact documentation
command was rerun successfully, alongside final review lint, whitespace and
hygiene checks, with its corrected result retained separately from the initial
failure. No Cargo command was repeated to repair that environment-only error.
The final light audit allows only this specifically repaired non-Cargo error;
it still requires all ordinary Cargo completions and unchanged source hashes.

The baseline combines all 43 targets from the complete owner-faults run with
the complete 18 host/API targets from the newer shared-configuration run,
replacing those same older targets. The whole kernel and Linux platform source
trees are unchanged between those accepted commits; the audit checks that Git
relationship. It compares whole targets and exact test identities, not selected
passing outcomes. Private validation retains both complete prior logs, this
complete raw run, exact source copies, command plan, source hashes, diagnostic
comparison, corrected light checks and review helpers.

The final audit checks unchanged ledger row lines, the byte-identical retained
review, the exact nine-file change scope, newly authored public content and
exact final Git blobs. Its initial hygiene helper checked the entire historical
ledger and encountered a pre-existing path; the helper was corrected to check
newly authored diff content while preserving the owner's unchanged old ledger.
No product source or policy was changed for that check. Independent core review
of this exact increment remains required; neither parent closes.
