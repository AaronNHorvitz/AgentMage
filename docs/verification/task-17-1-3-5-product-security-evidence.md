# Task 17.1.3.5 Product Security Evidence

## Result

Pass for the locally executable Story 17.1 security-control mapping and retained
Linux Git and instruction evidence. The product-security sub-task remains
blocked because native macOS evidence, the deferred manual Git parser fuzz
campaign, and independent review are absent.

Mapped controls and review protocols: **9 of 9**.

This result organizes and hash-binds locally produced evidence. It does not
complete Task 17.1.3.5, Story 17.1, Sprint 17, a supported-platform gate, an
independent review, a user-facing product entrypoint, or a release gate.

## Control Map

| Requirement | Story 17.1 contribution | Retained proof | Remaining gate |
|---|---|---|---|
| `SR-ACC-006` | Demonstrated for Linux story scope | Held repository projection, prohibited-read canaries, strict offline worker, and invariant synthetic workspaces | Native macOS ambient-access campaign |
| `SR-ACC-007` | Demonstrated in story scope | Repository content cannot grant tools, broaden roots, transfer authority, or claim completion across 200 attacks | Independent review and supported-platform composition |
| `SR-ACC-008` | Demonstrated in story scope | Twenty source classes remain cited untrusted data; guidance is narrowing-only after an explicit user decision | Independent review and native macOS parity |
| `SR-AI-005` | Demonstrated in story scope | The 200-case direct and indirect injection matrix produces zero unauthorized action or disclosure | Product-wide injection campaign across later tool surfaces |
| `SR-AI-008` | Demonstrated for Linux story scope | Metadata-only discovery, separate bounded reads, no raw instruction ledger content, and prohibited-source canaries | Native macOS and later integrated model-context campaign |
| `SR-NET-001` | Partial Linux evidence | Both native Linux workers execute strict offline with zero loopback contact and retained socket/process observations | Full required observation interval on every supported platform |
| `SR-TST-002` | Deferred by recorded project decision | Deterministic fixed parser and trust-boundary corpora are retained | Manual Git parser fuzz campaign at the end of locally implementable development |
| `SR-TST-004` | Demonstrated for Linux story scope | Malformed, hostile, oversized, stale, conflicting, linked, and partial cases fail closed | Native macOS campaign and independent review |
| `RV-11` | Demonstrated in story scope | At least 200 attacks span source, comments, docs, filenames, Git metadata, tool results, model output, and authority-escalation requests | Re-execution as later untrusted-content surfaces are added |

## Retained Proof

- [`local-evidence-report.json`](../../artifacts/sprints/sprint-17/local-evidence-report.json)
  binds the source contracts, exact local command outcomes, native matrix
  identity, implemented controls, and current blockers.
- [`installed-linux-git-matrix.json`](../../artifacts/sprints/sprint-17/installed-linux-git-matrix.json)
  retains strict-offline Fedora 44 and Ubuntu 26.04 operation, fixture,
  discovery, hostile-configuration, network-observation, invariance, and
  teardown evidence.
- The native campaign covers thirteen operations, seven repository states,
  four real discovery classes, and nine hostile cases with zero canary
  execution and zero network use during subject execution.
- The machine-readable security map binds this document, the governing
  security requirements, both evidence reports, and its validator to one
  committed source revision.

## Limits

- Native macOS implementation and evidence are absent. Linux evidence is not
  substituted.
- Independent human review is absent. The automated mapping and validators
  were produced in the same development process.
- Manual fuzzing remains deliberately deferred and no fuzzing claim is made.
- Dependency acquisition occurred before each strict-offline native execution.
  The report claims network closure only for the subject-execution phase.
- Synthetic fixtures were used. No private data, repository credentials,
  external publication, release, or product-wide acceptance is claimed.
