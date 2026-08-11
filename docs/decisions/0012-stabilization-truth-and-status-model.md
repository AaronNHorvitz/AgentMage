# Decision 0012: Stabilization Truth and Status Model

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-11 |
| Scope | Current implementation truth, status vocabulary, and stabilization sequencing |
| Amends | Decision 0004 machine architecture and the execution posture under Decisions 0008 through 0011 |
| Preserves | All 17 epics, 169 sprints, 227 stable requirements, accepted capability scope, and historical evidence |

## Context

AgentMage has substantial planning, contract, validation, and isolated Linux
security work, but it does not yet have an integrated end-user workflow,
supported package, supported platform, or enabled model. Existing documents and
the language/build matrix used words such as `enabled` and `shipped` for future
targets or scaffolded components. Those words could be read as current product
claims.

Implementation progress, verification depth, disposition, and support are
different facts. A single linear label cannot represent them honestly. Current
truth also must not rewrite historical evidence simply to make a present gate
green.

## Decision

1. [`architecture/status-model.json`](../../architecture/status-model.json) is
   the machine-readable source for current component, platform, model, and
   product status.
2. Every current status record has exactly one value in each of four orthogonal
   dimensions:
   - lifecycle: `planned`, `designed`, `scaffolded`, `implemented`,
     `integrated`, `packaged`, or `shipped`;
   - verification: `not-run`, `contract-tested`, `native-tested`, or
     `release-verified`;
   - disposition: `active`, `blocked`, `rejected`, or `superseded`;
   - support: `unsupported-pre-release`, `supported`, or `end-of-support`.
3. Lifecycle and verification promotion is sequential. A skipped promotion,
   unsupported value, missing evidence basis, or support claim without release
   verification fails validation.
   Machine references use the explicit
   `architecture/status-model.json#<collection>=<record-id>` selector contract;
   validators resolve the named record ID and reject stale or missing targets.
4. Historical records retain the vocabulary, state, source revision, and
   meaning recorded when they were produced. Current status references those
   records as evidence; it does not edit or relabel them.
5. The current integrated product lifecycle is `scaffolded`. No end-to-end user
   workflow is integrated, no model is enabled, no platform is supported, no
   package is released, and the first-GA gate is blocked.
6. Gemma 4 E4B and Gemma 4 12B Unified remain named candidates, but their
   current dispositions are `rejected` and disabled. Reopening either candidate
   requires a new revision-bound admission and evaluation decision. No fallback
   is automatic.
7. Fedora and Ubuntu remain first-GA Linux targets. Windows 11 x64 is a required
   first-GA target whose implementation is currently `planned`. Apple Silicon
   macOS remains a blocked, retained post-GA lane. No platform evidence
   substitutes for another platform.
8. Decision 0008 remains the authority that added Windows and changed the
   first-GA platform boundary. This decision updates the machine architecture
   to express that accepted scope without claiming Windows implementation or
   support.
9. New capability families are frozen during stabilization. An exception
   requires explicit user approval and a written impact statement covering
   architecture, authority, security, privacy, platforms, tests, evidence,
   packaging, support, recovery, and release gates.
   The stabilization scope freeze remains active until the final gate.
10. The original numbered roadmap remains preserved but paused until the final
    stabilization gate explicitly authorizes resumption. Stabilization progress
    is measured by integrated verified paths, not document volume, checklist
    volume, isolated scaffolds, or retained historical evidence.

## Current Baseline

| Surface | Lifecycle | Verification | Disposition | Support |
|---|---|---|---|---|
| Integrated AgentMage product | `scaffolded` | `not-run` | `active` | `unsupported-pre-release` |
| Fedora target lane | `scaffolded` | `contract-tested` | `active` | `unsupported-pre-release` |
| Ubuntu target lane | `scaffolded` | `not-run` | `active` | `unsupported-pre-release` |
| Windows 11 x64 target lane | `planned` | `not-run` | `active` | `unsupported-pre-release` |
| Apple Silicon macOS lane | `scaffolded` | `not-run` | `blocked` | `unsupported-pre-release` |
| Gemma 4 E4B candidate | `scaffolded` | `native-tested` | `rejected` | `unsupported-pre-release` |
| Gemma 4 12B Unified candidate | `scaffolded` | `native-tested` | `rejected` | `unsupported-pre-release` |

`contract-tested` and `native-tested` describe only the exact evidence named by
the current status record. They do not imply integration, packaging, release
verification, platform support, or product acceptance.

## Consequences

- Canonical planning documents distinguish current implementation truth from
  target product behavior.
- The language/build matrix no longer uses a boolean `shipped` field.
- Windows appears in the machine architecture as planned and unsupported.
- Model picker, runtime, and fallback prose cannot describe a rejected candidate
  as enabled.
- A scaffold or isolated passing test cannot promote the integrated product.
- Historical decisions and evidence remain available with their original
  identities and claims.
- Stabilization can narrow execution order without deleting accepted scope.

## Verification

- `python3 scripts/status_model.py` validates the closed vocabulary, legal
  transitions, evidence paths, current product truth, model dispositions,
  platform lanes, scope counts, document markers, and matrix references.
- `python3 scripts/architecture_decision.py` validates the amended component and
  platform matrix, rejects the former `shipped` boolean, and requires Windows to
  remain planned until later implementation evidence promotes it.
- Mutation tests reject unknown states, skipped transitions, missing evidence,
  enabled rejected models, unsupported platform claims, scaffold overclaims,
  changed scope counts, stale document claims, and platform-status drift.
- Documentation validation requires this accepted decision and its status model.
