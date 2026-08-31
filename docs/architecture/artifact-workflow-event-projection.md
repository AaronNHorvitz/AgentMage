# Artifact and Workflow Event Projection

Status: Story 21.3 local runtime-journal contract

## Closed correctness families

The existing `RuntimeEvent` envelope now admits thirteen additional content-minimized families:
source admitted; extraction started, completed, or blocked; section indexed; context disposition;
preflight observed; attempt started or ended; verification observed; retry decided; recovery
decided; and terminal diagnostic. Each is a correctness event. It therefore commits through the
existing canonical SQLCipher journal transaction and never enters the optional progress or metric
queue.

All new events carry only bounded identities, stable approved codes, SHA-256 digests, and verified
runtime-artifact references. Source admission requires the exact source reference. Extraction
completion and terminal diagnosis require external payload references. Large parser output,
source bytes, tool output, diagnostics, prompts, token fragments, paths, secrets, credentials, and
hidden reasoning are not event fields. Extraction failure uses a closed nine-code reason dictionary
so arbitrary content cannot be smuggled through a diagnostic label.

## Ordering and correlation

Source admission precedes one extraction start; that extraction reaches exactly one completed or
blocked state. Sections require the matching completed extraction. Context dispositions require an
admitted source and a unique context/source pair.

Inside a turn, one tool request precedes unique preflight observations and a globally unique
attempt start. Worker start and terminal tool receipt precede the attempt's unique terminal
observation. One deterministic verification follows that observation, and at most one retry
decision follows verification. Retry eligibility is impossible after a completed tool result.
Recovery decisions and a single terminal diagnostic occur only at quiescent turn boundaries.
Successful or no-op terminal events cannot coexist with a terminal diagnostic or an unfinished
source extraction.

The sequence validator checks the existing run, session, task, policy, correlation, causation,
sequence, timestamp, and previous-digest bindings before applying these transitions. Rejected
duplicates do not mutate verifier state. Cross-source, cross-extraction, cross-attempt,
cross-observation, and cross-verification substitution is refused.

## Replay and compatibility

`replay_runtime_workflow_projection` first validates the complete canonical event chain and then
reconstructs sorted content-free source and attempt views, context decisions, recovery identities,
and the terminal diagnostic. The projection binds the exact event count and terminal chain digest.
`verify_runtime_workflow_projection` treats journal replay as authoritative and rejects drift in a
materialized view.

Existing journals remain valid without migration because no prior event shape or ordering rule was
changed. A client that declares no artifact/workflow projection support receives the explicit
`runtime.event.kind_unsupported` result for a new family; old families remain renderable.

## Durability and pressure boundary

The new events reuse the Story 21.2 single-writer journal, atomic correctness transaction, bounded
progress queue, nonblocking subscriber, slow-client eviction, replay cursor, terminal flush, and
encrypted reopen behavior. The Story 21.3 process-stop matrix terminates the process immediately
before and after all fifteen new-event occurrences (thirty cases), then verifies the exact durable
prefix, appends only the missing suffix, and performs two verified reopens with no duplicate or
invented event. Existing saturation, optional-sink, cancellation, and slow-client tests continue to
exercise the shared infrastructure; no second bus, store, transcript, or telemetry path is added.
