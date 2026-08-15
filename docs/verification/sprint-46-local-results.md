# Sprint 46 Local Verification

## Scope

The Sprint 46 local recorder covers trusted validation-template provenance, exact
executable and command identity, authority-free focused selection, separate approval
requirements, terminal command-receipt verification, strict result parsing,
non-conflated result states, independent artifact and affected-file observations,
partial coverage, failure classification, idempotent rerun planning, the detailed
receipt schema, the synthetic classification corpus, documentation, strict linting,
and supply-chain metadata.

## Deterministic Campaigns

- Template tests cover all nine validation kinds, exact project/user provenance,
  parser selection, execution-scope binding, cross-workspace artifact refusal,
  focused selection, stale mappings, argument and executable mutation, and unsafe
  rerun refusal.
- Command-runner tests cover exact request/preview/receipt binding, command grammar,
  credential environment rejection, one consumed grant per launch, cancellation
  before launch, descendant cleanup claims, and receipt-field mutation.
- Result tests cover pass, assertion, compile, infrastructure, timeout, cancellation,
  crash, flaky, skipped-only, malformed, truncated, zero-test, unverified,
  sensitive-output, and partial-coverage behavior. They also cover forged green text,
  ANSI/control sequences, parser size bounds, injected artifacts, secret-safe
  receipts, approval-bound reruns, and classification precedence.
- Runtime schema tests reject false pass, ambiguous partial coverage, secret-count
  drift, unsorted evidence, unknown properties, and raw output fields.
- The synthetic failure corpus covers change-caused, baseline, flaky, dependency,
  environment, permission, unrelated, and unclassified dispositions without raw
  process bytes or sensitive values.

These are deterministic local tests, not manual or coverage-guided fuzzing.

## Security Mapping

| Requirement | Local Sprint 46 contribution | Remaining product evidence |
|---|---|---|
| `SR-SUP-003` | Exact executable, parser, source, template, scope, and artifact identities | Trusted installed-package execution and independent dependency review |
| `SR-TST-001` through `SR-TST-006` | Closed test families, exact process evidence, failure-state matrix, fixture safety, bounds, and adversarial parser cases | Production native runner, platform resource traces, and independent campaigns |
| `SR-TST-010` | Exact local command inventory and hash-bound output evidence | Native supported-platform and release-candidate execution |
| `SR-OPS-003` | Content-free receipts, cancellation states, cleanup claims, and explicit blockers | Production diagnostics, protected raw-log retention, and incident workflow |

No product-wide requirement is marked complete by this local contribution.

## Security Inventory

- Sprint 46 introduces no `unsafe` Rust block, FFI declaration, dynamic library load,
  plugin hook, shell interpreter, response-file expansion, credential environment
  value, network-enabled validation template, or raw-output receipt field.
- Every admitted template requires an empty scratch working directory, complete
  environment replacement, network disabled, interactive mode disabled, ambient
  environment inheritance disabled, fixed parser bytes, and fixed process/output
  bounds.
- A secret-classified stream produces `sensitive_output`; only hashes, counts, and
  classifications enter the normalized receipt.
- Cancellation and timeout receipts require descendant cleanup, but this local
  increment does not constitute a native process-tree campaign on supported release
  packages.

## Truthful Disposition

A green local report proves the current platform-neutral validation contracts and
Fedora toolchain tests only. It does not close Sprint 46. Sprint 45 and upstream gates
remain blocked. No production Chat-to-validation coordinator, trusted installed-parent
execution, native worker/resource/process-tree campaign, protected raw-log storage,
required cross-platform evidence, trusted-package execution, independent review, or
manual fuzzing exists.
