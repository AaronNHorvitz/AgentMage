# T-303/T-306: terminal event records and offline replay

Date: 2026-11-08. Tier: core. Immutable predecessor:
`ea856c414862031771d36818fdc5db4c61a076a9`.
Its independent worker-selection review remains in flight at session start.
It arrived during verification with **Verdict: PASS** and no high/medium
finding. The report is retained unchanged. Its low L1 (executable refusal
discrimination) and L2 (startup trust selection visibility) remain same-row
backlog; neither parent closes on that predecessor's acceptance.
This is a non-closing increment in the existing parent rows. No row is added.

## Behavior and boundaries

The terminal client now exports one complete terminal endpoint event record
through the existing authenticated request/event routes, and replays an
explicitly supplied record offline without a service, credentials or model.
Export prints to stdout; it does not create a file or change the service's
retention. The caller chooses whether and where to save it. No model-disclosure,
advance, approval, cancellation, release or filesystem-effect route is called.

The original sealed request and every original event envelope remain intact.
The original kernel event state machine verifies hash continuity, immutable
scope, causation, legal transitions and terminal ordering. The start must name
this exact request digest. Empty/running/faulted prefixes, changing totals,
invalid scopes, duplicate/unknown members and positional records refuse.
The record is bounded to 4 MiB, 4096 events and the run's own event ceiling;
collection shares a 180-second socket deadline. Output is escaped using the
existing terminal renderer and fully checked before emission.

Replay declares its limits in output: chain consistency does not authenticate
the writer, verify referenced payloads, prove model claims or restore execution.
No tool-result body is fetched, no pending approval or grant reconstructed,
and no model, tool, port or budget is recreated. The original native admission
boundary is unchanged. Durable execution recovery, full payload retention,
edit/test/retry, approved Git and parent measurements remain open.

No evaluation inputs, hidden/held-out assertions, thresholds, dependencies,
models, GPU work, SBOM, status model, release or push change.

## Verification

The complete ordinary affected-package fast gate passes: **ten targets,
149 passed, zero failed, ignored or filtered**. This preserves all 147 passing
identities from the predecessor's retained complete gate and adds exactly two
integration methods. Existing test bodies and ledger table rows are unchanged.
Formatting, strict Clippy, changed-document lint, whitespace, scope and privacy
checks pass. All six Rust input hashes stayed frozen throughout the complete
gate; its six commands took 106.75 seconds including build reservations.
Current documentation and the newly retained review pass their separate
lightweight checks after the source gate. This is an affected-package result,
not a whole-workspace or deferred host-admission pass.

The first added method exercises actual retained HTTP cancellation and export,
release, service shutdown and removal of the synthetic credential directory,
then identical fresh-process offline replay. The endpoint, tools and verifier
are synthetic; no native worker or model runs. Negative controls include
truncation, reordering, duplicate events, altered digests, a valid replacement
request, independently resealed foreign scope/start/causation/duplicate-identity
chains, duplicate/unknown keys, positional records, trailing JSON and oversized
stdin. The second method uses a recording synthetic HTTP peer to check exact
observation-only routes, page indexes, Unicode escaping, changing/empty/partial
or oversized totals and lost replies without retry or automatic cancellation.

An attempted initial baseline rerun overlapped new module creation while
waiting for a build reservation and failed compilation; it is not a passing
baseline. The comparison above uses the already retained exact predecessor's
ordinary gate. The first focused export/replay run reached offline inspection
but failed a new assertion expecting lowercase `cancelled`; the original
contract emits `CANCELLED`. The assertion now expects that original spelling,
and both new methods pass in the focused repeat and complete gate. Raw failures,
complete logs, source snapshots and identity audits are retained privately.
No threshold or existing assertion was relaxed. Independent core review of
this increment remains required.
