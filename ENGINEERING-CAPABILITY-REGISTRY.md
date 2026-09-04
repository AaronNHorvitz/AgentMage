# AgentMage Engineering Capability Registry

| Field              | Value                                                                                                    |
| ------------------ | -------------------------------------------------------------------------------------------------------- |
| Status             | Normative implementation specification; contract-tested registry scaffold, no enabled product capability |
| Decision           | 0043 as mandatorily superseded by 0045                                                                   |
| Runtime authority  | `ENGINEERING-RUNTIME.md`                                                                                 |
| Agent role catalog | `docs/architecture/planning-review-delivery-agent-profiles.md`                                           |

Current product lifecycle: `scaffolded`.
Current integrated workflow: deterministic fake-model repository-analysis vertical slice (Story 22.5).
The bound evidence is [`vertical-slice-report.json`](artifacts/sprints/sprint-22/story-22.5/vertical-slice-report.json); it does not register or enable a product capability.
Current enabled models: none.
Current supported platforms: none.
Stabilization scope freeze: inactive (Decision 0047) — the complete preserved plan is open under the Decision 0021 execution rule; blocked rows retain exact blockers and empty substitution sets.

The Rust kernel currently validates, seals, registers, resolves, and executes
closed manifests through an injected execution/verifier port and includes one
static Repository Review candidate manifest. This is scaffold evidence only:
the full lifecycle, dependency qualification, migration, quarantine, removal,
initial disabled catalog, production registration, and integrated capability
execution remain open.

## 1. Purpose

The Engineering Capability Registry turns repeatable engineering outcomes into
versioned executable contracts. A capability is not a prompt file, model
persona, plugin permission, or private agent loop. It is a deterministic
workflow definition with closed inputs, outputs, dependencies, authority,
budgets, verification, degradation, fixtures, and lifecycle.

## 2. Authority Boundary

A manifest can declare the maximum authority a capability may request. It
cannot mint a `CapabilityGrant`, resolve a credential, register an undeclared
tool, bypass approval, choose a remote disclosure, or establish completion.
The Rust runtime intersects the manifest with current user, workspace, policy,
provider, platform, and task authority for each invocation.

Models may select among already admitted capabilities when current policy
allows it. Model output cannot install, enable, alter, or widen a capability.

## 3. Manifest Contract

Every manifest is closed, versioned, signed or otherwise admitted according to
its distribution class, and includes:

- Capability identity, version, publisher, lifecycle, and content digest;
- Human-facing title and bounded purpose;
- Input and output schema references and hashes;
- Workflow definition reference and hash;
- Required tools and exact minimum versions;
- Required model roles and qualified capability tiers;
- Required platform and optional dependency classes;
- Maximum requested authority and prohibited authority;
- Data classifications, network classes, and retention behavior;
- Turn, token, time, tool, effect, retry, no-progress, resource, and cost
  budgets;
- Preflight and postcondition verifier references;
- Degradation and terminal-failure rules;
- Fixture, conformance, security, migration, disablement, and removal records;
- Compatibility and supersession relationships.

Input and output schemas reject unknown fields. A capability with missing,
stale, incompatible, or unqualified dependencies remains disabled or blocked.

## 4. Lifecycle

```text
draft -> admitted -> enabled -> degraded -> disabled -> retired
             |          |          |           |
             +--------> quarantined <----------+
```

Only deterministic admission can move a manifest out of draft. Enablement
requires all dependencies and current conformance evidence. Degradation is
visible and dependency-specific. Quarantine revokes invocation immediately.
Retirement preserves provenance and migration records while removing runtime
registration and authority.

No lifecycle state implies platform support or release readiness without the
separate evidence required by `architecture/status-model.json`.

## 5. Workflow Graph

A capability workflow is a directed acyclic graph unless a bounded loop is
explicitly represented by a runtime-owned transition with a finite budget and
no-progress detector. Nodes identify exact input projection, model role if any,
tool, preconditions, side-effect class, authority requirement, postconditions,
retry class, output projection, and terminal behavior.

The runtime, not the model, chooses legal transitions. Tool and model nodes
communicate only through typed runtime records. Capability-specific verification
is in addition to universal runtime verification and cannot weaken it.

## 6. Capability Selection

Selection considers:

- Exact user intent and requested outcome;
- Input schema compatibility;
- Enabled lifecycle state;
- Available tools, platform, and model role qualifications;
- Data classification and route policy;
- Current authority and approvals;
- Resource and cost budgets;
- Degradation state and declared alternatives.

When no capability is admissible, the runtime reports why and may offer a plan
or read-only explanation. It does not improvise a hidden workflow.

## 7. Initial Catalog Candidates

The initial planned registry composes already accepted AgentMage work:

| Capability              | Primary outcome                                           | Earliest owning work       |
| ----------------------- | --------------------------------------------------------- | -------------------------- |
| Repository inspection   | Cited repository map and bounded source analysis          | Sprints 18-23              |
| Whole-codebase audit    | Resumable evidence-backed comprehensive audit             | Sprints 157-166            |
| Coding change           | Isolated plan, patch, validation, review, and handoff     | Sprints 41-50              |
| Pull-request review     | Independent local and hosted review packet                | Sprints 74, 85-86, 106-107 |
| Issue and bug workflow  | Evidence-backed issue, bug, and task lifecycle            | Sprints 92-95 and 107      |
| Test diagnosis          | Structured test observation and bounded repair plan       | Sprints 41-50              |
| Release readiness       | Traceable build, security, support, and release evidence  | Sprints 96-100 and 103-126 |
| Document workflow       | Source-preserving Word, PDF, sheet, and presentation work | Sprints 54-65              |
| Research with citations | Public-source research through isolated workers           | Sprints 82 and 160         |

Catalog inclusion above is planned identity, not enablement. Each capability
requires its own manifest, fixtures, implementation, conformance, security,
platform, removal, and release evidence.

## 8. Agent Profiles

The `AG-01` through `AG-49` profiles from Decision 0041 are bounded model roles,
not capabilities. A capability may request one or more qualified roles. The
same role can serve multiple capabilities without owning workflow state,
credentials, tools, or provider clients.

Multi-agent workflows combine capabilities and role profiles only after the
single-agent reliability spine passes. Pods receive separate work packets,
grants, leases, budgets, worktrees where applicable, and evidence. Independent
review and serialized integration remain deterministic capabilities rather
than model self-approval.

## 9. Degradation

Each dependency is classified:

- `required`: absence blocks the capability;
- `optional`: absence disables a named enhancement and is shown to the user;
- `substitutable`: an explicitly qualified alternative may be selected under a
  fresh decision and equivalent or stricter policy.

Degradation cannot remove required verification, weaken authority, change data
classification, silently select a remote route, or turn partial work into
success.

## 10. Versioning and Migration

Manifest, input, output, workflow, verifier, tool, model-role, and evidence
versions are independent and hash-bound. An incompatible change creates a new
capability version. Migrations declare source and destination versions,
preconditions, deterministic transformation, rollback or non-reversibility,
and fixtures.

Unknown versions fail closed. A prompt edit, model update, endpoint change,
tool change, or verifier change invalidates affected qualification and cannot
silently alter an enabled capability.

## 11. Conformance and Security

Every capability must test:

- Valid, invalid, missing, extra, oversized, stale, and unsupported inputs;
- Dependency loss, cancellation, timeout, crash, resume, and low resources;
- Prompt injection, authority widening, hidden effects, secret exposure, and
  model attempts to alter workflow or completion;
- Tool denial, malformed observations, uncertain effects, and verifier failure;
- Version migration, disablement, quarantine, removal, and clean strict-local
  restoration;
- Cross-interface parity where the capability is exposed by more than one
  client.

Evidence binds the exact manifest, schemas, workflow, tools, model profiles,
routes, policy, platform, fixtures, and source revision. No prompt-only file can
be represented as a complete capability.

## 12. Machine-Readable Authority

The planned closed manifest schema is
`schemas/engineering-runtime/capability-manifest.schema.json`. Decision,
requirement, test, control, milestone, story, schema, module, and verification
linkage is recorded in
`architecture/engineering-runtime-change-manifest.json`. `TASKS.md` remains the
execution authority and `architecture/status-model.json` remains current-status
authority.

## 13. Roadmap Placement

Capability admission and lifecycle are assigned to Story 95.3. Bounded multi-agent composition is
assigned to Story 95.4 only after the single-agent reliability spine passes. Integrated capability,
route, platform, and recovery qualification is assigned to Story 125.3, and final additive-scope
reconciliation is assigned to Story 126.2. These placements do not enable any candidate manifest or
agent profile.
