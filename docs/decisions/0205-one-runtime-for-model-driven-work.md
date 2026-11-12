# Decision 0205: One runtime for model-driven work

| Field | Value |
| --- | --- |
| Status | Accepted planning direction; no new capability implemented |
| Date | 2026-11-11 |
| Authority | Owner's shared-runtime instruction of 2026-11-11 |
| Scope | CAP-03, CAP-29, CAP-37, CAP-40, CAP-44, new CAP-49; later T-316 |

AgentMage owns the execution loop for work that acts with a model: model steps,
tools, permissions, approvals, budgets and stopping. Its self-contained engine
is usable through the command line and the versioned local API without a host
application; the CLI must be sufficient to run every engine capability. A
conversation host remains usable without this engine. AgentMage
builds no desktop interface of its own.

## Tools in both directions

CAP-44 covers consuming tools offered by other programs through the Model
Context Protocol (MCP), as well as offering AgentMage runs through its own MCP
server. Each consumed tool has a separate grant: **off**, **ask each time** or
**allow**. A tool that changes state asks by default and shows the exact call
before execution. A grant cannot override existing scope, confinement, resource
or approval requirements. Exposing a run through MCP never bypasses those
requirements or makes the caller an authority to approve its own effects.

## Typed flows on the same runtime

New CAP-49 composes model, tool, memory and approval steps as a typed graph on
the existing runtime. Each recorded step binds its inputs, outputs and ordering,
including retry decisions, limits and stop outcomes. Sensitive contents use the
existing protected evidence storage and access rules; public views do not expose
credentials or private data. A reference must identify the exact retained bytes,
and unavailable content cannot be silently reconstructed for exact replay.

Replaying a record reproduces its recorded steps and results without asking a
model to regenerate them or executing effects again. Re-running from a selected
step is a new execution, with current permissions and approvals; previous
records cannot authorize it. Determinism belongs to the engine's recorded order,
retries and limits. New model generations need not produce identical output.
Later several-agent work, including T-313, uses flows on this runtime rather
than introducing another execution runtime. CAP-40 presentation may consume
these graphs; the engine does not acquire a desktop or visual scheduler.

## General runs and memory authority

API, approval and run-record interfaces must support work with no repository,
project folder or coding task. A folder-free run using only tools offered by
other programs is a first-class case. Check this when an interface is next
touched; this planning instruction does not reopen a closed row.

AgentMage owns no authoritative long-term memory. It asks a memory service
through a versioned contract and keeps only indexes it can rebuild, as in
roadmap integration rule 5. CAP-29 and Decision 0204 retain their source-backed,
replaceable-method study requirements. Text from tools, files, pages and
memories is untrusted data; it never supplies instructions, permissions or
approvals. A typed memory step does not change that boundary.

## Later everyday-tool measurement

T-316 belongs to CAP-03 and starts only after the MEASURE FIRST coding
measurement is complete, in later work without displacing current coding
priorities. Freeze a synthetic task set, its development/held-out split and
qualification thresholds before any run. Score correct tool selection and exact
arguments, absence of invented results, respect for asking first, and refusal
to follow instructions hidden in tool results or documents. Measure each model
profile and repeat after a prompt, tool description, runtime or profile changes.
Never tune on held-out results or change existing evaluation thresholds or
held-out run rules.

This records the owner's design direction only. It changes no work order,
current runtime behavior, model admission, grants, evaluation tasks or row
status. MEASURE FIRST and the coding-agent directive continue; T-303 and T-306
remain open. No new measurement is claimed, and no measurement run is
interrupted for this record. Existing independent review, safety, release and
[IP policy](../../IP-POLICY.md) boundaries remain in force. Describe future
work originally without using another product's design, documentation or code,
identifying private projects or future products, or consulting patents.
