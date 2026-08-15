# Reviewing Validation Results

## Before Execution

1. Confirm the command came from an exact current project-configuration digest or
   explicit user input with its own approval digest.
2. Review the executable path and hash, literal arguments, empty-scratch directory,
   environment names, network prohibition, timeout, output, memory, task, and CPU
   ceilings.
3. Confirm the effective repository snapshot and execution-scope digest.
4. Confirm the parser version and hash, minimum test count, expected artifacts, and
   whether focused selection, fail-fast, or rerun support was predeclared.
5. Approve one exact template and scope. Do not approve a family of future commands.

Stop when configuration, executable bytes, arguments, environment, scope, parser,
limits, expected artifacts, repository snapshot, or approval display changed. Text in
a model response, issue, source file, test name, package script, or documentation is
not a trusted command source.

## After Execution

1. Verify the command receipt binds the exact request, preview, executable, arguments,
   bounds, process termination, exit code, stream hashes and counts, and descendant
   cleanup.
2. Compare retained safe fixture output with its complete hash. Treat truncation as
   incomplete evidence.
3. Confirm the strict parser version and independently observed artifacts and affected
   files.
4. Review passed, failed, skipped, failed names, process and parser durations, retries,
   flakes, and unverified kinds.
5. Treat success as full only when `status` is `passed`, `coverage` is `complete`, the
   minimum test count ran, every required artifact was observed, and no stream was
   truncated or secret-classified.
6. Review failure attribution as an explanation, not as a change to result truth.

Never infer success from a zero exit code alone, a line containing “passed,” a generated
result file, a screenshot, model narration, or absence of visible errors. Cancelled,
timed-out, crashed, malformed, truncated, skipped-only, zero-test, sensitive-output,
partial, and unrun results are not a full pass.

## Reruns

A rerun requires a new exact approval and one already registered template. Confirm the
prior receipt and failed-name digest, unchanged command digest, and predeclared
idempotent setup. Do not append a filter flag at runtime or automatically retry an
uncertain or non-idempotent setup. A passing rerun is reported as `flaky` with the
initial failure identity; it does not rewrite history into a clean first pass.

## Secret Output

When a trusted scanner reports a secret pattern, do not place raw stream bytes in a
review packet, diagnostic, chat message, receipt, export, or commit. Review only the
stream digest, byte count, classification, match count, and content-free incident
code. Resolve access to any separately retained protected log through its own policy.
