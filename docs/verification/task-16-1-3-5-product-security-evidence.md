# Task 16.1.3.5 Product Security Evidence

## Result

Pass for the locally executable Story 16.1 security-control mapping and retained
Linux worker evidence. The product-security sub-task remains blocked because
the required independent worker review and native macOS evidence are absent.

Mapped controls and review protocols: **14 of 14**.

This result organizes and hash-binds the evidence that can be produced locally.
It does not complete Task 16.1.3.5, Story 16.1, Sprint 16, a supported-platform
gate, an independent review, or a release gate.

## Control Map

| Requirement | Story 16.1 contribution | Retained proof | Remaining gate |
|---|---|---|---|
| `SR-PLT-003` | Blocked on macOS | Linux Bubblewrap and systemd confinement traces are retained without substitution | Native App Sandbox and XPC entitlement dump and attacks |
| `SR-PLT-004` | Partial Linux evidence | Held workspace objects, sealed projections, path attacks, and metadata invariance | Native bookmark, alias, mount, case, and Unicode campaign |
| `SR-ACC-001` | Demonstrated in story scope | Kernel-issued one-use grants and one terminal receipt per launched attempt | Independent worker review and supported-platform composition |
| `SR-ACC-002` | Partial story evidence | Exact tool, target, schema, projection, policy, preview, and nonce bindings | Complete grant-field mutation corpus |
| `SR-ACC-003` | Demonstrated in story scope | Atomic one-use consumption and replay denial for success and interrupted attempts | Supported-platform and independent review |
| `SR-ACC-004` | Partial story evidence | Closed relative targets and traversal, separator, and malformed-input denial | Full path corpus and native macOS parity |
| `SR-ACC-005` | Demonstrated for Linux story scope | Held descriptors, sealed memfds, stale-object refusal, and forced link races | Native macOS descriptor/bookmark race campaign |
| `SR-ACC-006` | Demonstrated for Linux story scope | Ambient filesystem, process, environment, socket, network, device, and secret-canary attacks | Native macOS ambient-access campaign |
| `SR-AI-005` | Partial story evidence | Tool output cannot authorize another call and sensitive output is withheld before model context | Complete labeled direct and indirect injection corpus |
| `SR-TST-002` | Deferred by recorded project decision | Closed schemas and deterministic hostile fixtures are retained | Manual fuzzing campaign at the end of locally implementable development |
| `SR-TST-004` | Demonstrated for Linux story scope | Malformed, hostile, oversized, partial, stale, conflicting, and interrupted cases | Native macOS worker campaign and independent review |
| `SR-TST-006` | Partial story evidence | File, input, output, depth, match, process, time, and call limits plus cleanup | Complete supported-platform resource and responsiveness campaign |
| `RV-03` | Demonstrated for Linux story scope | Strict-offline installed-worker operation, attack, and lifecycle matrices | Native macOS sandbox campaign and independent review |
| `RV-04` | Demonstrated for Linux story scope | Traversal, stale identity, symlink race, workspace invariance, and cleanup proof | Native macOS path and race campaign |

## Retained Proof

- [`local-evidence-report.json`](../../artifacts/sprints/sprint-16/local-evidence-report.json)
  binds the closed schemas, golden behavior, host authority flow, effect boundary,
  source identities, exact command outcomes, and current blockers.
- [`installed-linux-worker-matrix.json`](../../artifacts/sprints/sprint-16/installed-linux-worker-matrix.json)
  retains strict-offline Fedora 44 and Ubuntu 26.04 package, operation, attack,
  lifecycle, workspace-invariance, and teardown evidence.
- The installed campaign covers all ten production operations, eleven attack
  classes, and twelve cancel, timeout, kill, and crash positions without using
  private user data or repository credentials.
- The machine-readable security map independently binds this document, the
  governing security requirements, the two evidence reports, and its validator
  to one committed source revision.

## Limits

- Native macOS XPC implementation, operation evidence, sandbox attacks, and
  lifecycle evidence are absent and Linux evidence is not substituted.
- Independent human worker review is absent. The automated mapping and
  validators were produced in the same development process.
- Manual fuzzing remains deliberately deferred and no fuzzing claim is made.
- The Linux VM campaign uses synthetic fixtures. It does not claim private-data,
  credential, installed-interface, release, signing, or product-wide acceptance.
