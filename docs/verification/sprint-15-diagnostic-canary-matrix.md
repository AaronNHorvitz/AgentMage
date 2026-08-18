# Sprint 15 Diagnostic Canary Matrix

**Status:** Passed for the currently reachable Linux/local diagnostic surfaces; broader Sprint 15 remains blocked.

## Boundary

The campaign injects a distinct synthetic value for each prohibited diagnostic source family: prompt, file content, credential, private key, environment value, absolute path, hostname, username, and stable device identifier. These test values are not private user data.

The production `DiagnosticObservation` schema has no raw-source fields and rejects unknown fields. The authenticated host bridge and native Chat renderer also require exact closed report and item shapes. The export workflow receives only the resulting content-free report.

## Reconciliation

The retained matrix executes and binds three boundaries:

1. The kernel harness rejects every prohibited source field, exposes no raw value through its error, and produces the same non-sensitive report before and after rejection.
2. The host export path preserves the exact safe report while preview, receipt, and exported JSON contain no synthetic value.
3. The authenticated bridge and Chat renderer reject unknown raw-source fields with one stable content-free denial.

Successful command output is retained as the current diagnostic log surface and scanned for the synthetic prefix. The product path has no diagnostic logger or external telemetry call.

The retained machine-readable result is [`diagnostic-canary-matrix.json`](../../artifacts/sprints/sprint-15/story-15.2/diagnostic-canary-matrix.json), bound to source revision `fa8846c5e01157657a0b2b56e171f7a2146f6f10`. Its private mode-`0600` command trace is retained beside the report and verified by byte count and SHA-256.

## Limits

This closes the currently reachable Linux/local surfaces for Sub-task `15.2.2.2`; it does not establish native keyboard or screen-reader behavior, macOS or Windows parity, installed-package behavior, independent review, process-memory absence, storage remanence, or manual-fuzz results. Any new diagnostic source, logger, transport, receipt, or export invalidates the evidence until the campaign is extended and rerun.
