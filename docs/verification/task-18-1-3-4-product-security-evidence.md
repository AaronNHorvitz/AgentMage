# Task 18.1.3.4 Product Security Evidence

## Result

Pass for the locally executable Story 18.1 security-control mapping and retained
Fedora repository-map evidence. Sprint closure remains blocked because native
Ubuntu, macOS, and Windows evidence, the deferred manual parser fuzz campaign,
and independent review are absent.

Mapped controls: **7 of 7**.

This result organizes and source-binds locally produced evidence. It does not
complete Sprint 18, a supported-platform gate, an independent review, a
user-facing product entrypoint, or a release gate.

## Control Map

| Requirement | Story 18.1 contribution | Retained proof | Remaining gate |
|---|---|---|---|
| `SR-ACC-008` | Demonstrated for Fedora story scope | Repository instructions and hostile filter configuration remain untrusted bytes and cannot change projection, policy, authority, or execution | Other native-platform campaigns and independent review |
| `SR-AI-003` | Demonstrated in story scope | The deterministic map and renderer have no model input or model authority | Integrated model-context and interface evidence |
| `SR-AI-007` | Demonstrated in story scope | Excluded, unread, unsupported, malformed, truncated, symbolic-link, and Gitlink states remain explicit rather than inferred away | Installed-interface rendering evidence |
| `SR-AI-010` | Demonstrated in story scope | Every structural fact binds exact repository, worktree, Git, path, content, range, syntax, grammar, parser, policy, and freshness identity | Product-wide model/runtime inference provenance |
| `SR-OPS-001` | Partial story evidence | Content-free typed failures, source-bound command outcomes, correlation-safe cache scope, and exact result hashes are retained | Complete durable product audit-event registry |
| `SR-TST-002` | Deferred by recorded project decision | Closed parser, inventory, projection, cache, and trust-boundary corpora remain deterministic and repeatable | Manual parser fuzz campaign at the end of locally implementable development |
| `SR-TST-004` | Demonstrated for Fedora story scope | Hostile, malformed, oversized, stale, cross-repository, tampered, interrupted, timeout, linked, unsupported, and excluded cases fail closed | Other native-platform campaigns and independent review |

## Retained Proof

- [`local-evidence-report.json`](../../artifacts/sprints/sprint-18/local-evidence-report.json)
  binds committed source identities, exact command outcomes, the grammar BOM,
  implemented bounds, native Fedora claims, and every current blocker.
- [`pinned-repository-map.md`](../architecture/pinned-repository-map.md)
  records the authority boundary, grammar set, source-resolution contract,
  encrypted derivative-cache lifecycle, and one-use synchronization rule.
- The Fedora fixture retains tracked, untracked, ignored, staged, conflicted,
  regular-file, symbolic-link, and Gitlink facts while keeping hostile Git
  filters inert and rejecting cross-repository projection.
- Cancellation and timeout cases kill and reap inventory children. Cache
  restart, expiry, tamper, capacity, reconciliation, and one-use permit cases
  execute without filesystem, process, model, network, or canonical-state
  authority in the map capability.

## Limits

- Native Ubuntu, macOS, and Windows repository-map campaigns are absent.
  Fedora evidence is not substituted for them.
- Independent human review is absent. The automated mapping and validators
  were produced in the same development process.
- Manual fuzzing remains deliberately deferred and no fuzzing claim is made.
- Synthetic fixtures were used. No private data, repository credentials,
  external publication, release, or product-wide acceptance is claimed.
