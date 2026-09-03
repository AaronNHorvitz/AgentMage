# Sprint 48 Local Verification

## Scope

The Sprint 48 local recorder covers the closed request and event protocol, all
five client surfaces, strict command parsing, status and event rendering, stable
exit codes, shell completion, predeclared headless grants, event ordering,
replay and resume controls, deterministic cancellation, JSON Schemas, and the
40-case adversarial corpus.

It also builds and invokes the real local `agent` binary. Help, version, and
completion return successfully. Operational commands fail closed with exit code
`5` and a content-minimized `client.transport.failed` record because no
authenticated product transport is composed.

## Deterministic Campaigns

- Thin-client unit tests seal and verify requests, compare the
  surface-independent kernel operation across native Chat, interactive CLI,
  JSON, SDK, and ACP-compatible surfaces, and reject stale or mismatched grants.
- Event-stream tests reject malformed, oversized, replayed, reordered, partial,
  policy-drifted, request-drifted, approval-bearing headless, and receipt-free
  success streams.
- CLI unit and executable tests cover every command family, human and JSON
  rendering, stable errors, help, version, completion, noninteractive JSON
  enforcement, and the unavailable transport boundary.
- Closed JSON Schemas reject unknown fields, direct-access fields, malformed
  command variants, authority drift, output-bound drift, malformed event
  payloads, and protocol-version drift.
- The adversarial corpus records 40 fail-closed or uncertain cases without raw
  content, command arguments, paths, remotes, credentials, or prompts.
- The effect-boundary gate rejects process-launch APIs in shell production code.
  The thin client has no direct storage, filesystem, tool, model, connector,
  secret, browser, external-application, or raw-host-socket API.
- The gate-owned automated review independently rederives the seven-requirement
  map, closed protocol/rendering boundary, exact headless authority controls,
  5 client surfaces, 4 command families, 9 exit codes, 23 runtime records,
  2 thin-client schemas, 40 adversarial cases, zero unauthorized effects, and
  every false missing-proof marker. It makes no human-review, transport,
  coordinator, installed-package, model, or platform claim.

These are deterministic unit, executable, schema, static-boundary, and
content-minimized adversarial tests. They are not native package acceptance,
independent review, or manual or coverage-guided fuzzing.

## Security Mapping

| Requirements | Local Sprint 48 contribution | Remaining product evidence |
|---|---|---|
| `SR-PLT-005`, `SR-PLT-006` | Closed transport/display-only client contracts, no process launch, and bounded protocol input/output | Authenticated native product transport, platform package acceptance, and native disconnect campaign |
| `SR-ACC-001`, `SR-ACC-007` | Exact command-to-grant binding, policy and argument digests, expiry, single use, replay denial, and content-minimized terminal evidence | Product coordinators, protected native approval integration, installed-product audit, and independent review |
| `SR-OPS-001` | Stable content-free error classes, explicit blocked status, and reproducible command-output hashes | Installed diagnostics and support-package evidence |
| `SR-TST-001`, `SR-TST-004` | Unit, binary, schema, static-boundary, replay, cancellation, and 40-case adversarial coverage | Cross-platform campaigns, independent tests, and deferred manual fuzzing |

No product-wide requirement is marked complete by this local contribution.

## Truthful Disposition

A green local report proves the source contracts, parser, schemas, static
boundary, real source-built CLI failure behavior, and local deterministic tests
at the immutable revision named in the report. It does not close Sprint 48.
Sprint 47 and upstream gates remain blocked. No authenticated product transport,
canonical conversation/knowledge/operational coordinator, complete native
disconnect and descendant cleanup campaign, supported-platform acceptance,
trusted installed-package execution, independent human review, or manual fuzzing
exists.
