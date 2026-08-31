# Closed Tool Observation Contract

Status: Story 16.4 local runtime contract

## One terminal truth

`assemble_tool_observation` accepts a verified terminal attempt from the Story 16.3 composition
layer and publishes exactly one `ClosedToolObservation` per tool-call identity. Its closed
disposition distinguishes success, verified no-op, denial, cancellation, timeout, resource
exhaustion, crash, malformed output, partial effect, and uncertainty. The ledger refuses a second
terminal event even when the first event was a failure or crash reconciliation.

Successful and no-op observations require a worker receipt, passed deterministic verification,
complete cleanup, and a completion-verified terminal attempt. No other disposition projects to a
canonical successful outcome. A failed observation may truthfully retain incomplete or unknown
cleanup; the canonical validator rejects unclean success while preserving non-success evidence for
diagnosis and recovery.

## Complete output and bounded disclosure

Standard output, standard error, binary output, and structured output are four separate atomic
content-addressed artifact candidates. The artifact authority must return each exact kind once with
the complete byte length and SHA-256 digest. Missing, duplicate, reordered, forged, or mismatched
references fail before the observation becomes durable. The complete bytes never enter the closed
observation record.

Only bounded UTF-8 display excerpts enter the canonical projection. Sensitive-looking output is
replaced with a fixed withholding marker; truncation is explicit. The full artifact reference,
length, and digest remain available independently of the excerpt. Each stream has a 16 MiB hard
assembly ceiling and excerpts have an 8 KiB ceiling.

## Bound identity and measurement

The observation binds the validated call and arguments, tool schema, call/attempt/task/step,
process identity, policy digest, consumed grant, approval, composition receipt and optional worker
receipt, trusted timing, exit or signal, resource usage, admitted limits, truncation, state change,
cleanup, generated artifacts, and all four output references. The canonical receipt field is the
SHA-256 digest of the composition receipt identity. A final observation digest covers the complete
closed record, including the richer lineage and resource-limit fields not present in the legacy
canonical schema.

Measured elapsed time, memory, CPU, descendant count, and aggregate output are checked against the
admitted ceilings. Any overage presented as success is refused before artifact publication. The
resource-exhausted disposition may preserve over-limit measurements without converting them into
success.

## Failure and platform boundary

Crash, supervisor boundary, output-pipe failure, cancellation/timeout race, partial effect, and
unknown effect are all terminal non-success observations. Tests exercise the assembler with a fake
atomic artifact authority and with output captured from a real local process. Native installed
worker campaigns, macOS XPC validation, and independent release review remain separate platform
and release gates; this contract does not manufacture those authorities or claim their evidence.
