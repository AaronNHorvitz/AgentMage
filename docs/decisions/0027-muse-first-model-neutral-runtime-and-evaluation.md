# Decision 0027: Muse-First Model-Neutral Runtime and Evaluation

| Field | Value |
|---|---|
| Status | Accepted scope and architecture refinement |
| Date | 2026-08-12 |
| Scope | Candidate-neutral local-model construction, Muse-first implementation focus, comprehensive first-party Gemma testing, extensible candidate intake, deterministic gating, repeatability claims, and early model evidence |
| Adds | `AM-MDL-004` through `AM-MDL-007`, `AM-AGT-001`, `AM-VSC-003`, `AT-MODEL-003` through `AT-MODEL-005`, `AT-CLASS-001`, `AT-AGENT-001`, and `AT-VSC-003` |
| Preserves | Every completed task and artifact, every stable requirement, the additions-only baseline, zero-model product truth, the kernel authority boundary, explicit user model selection, Sprint 49 measured-routing gate, Sprint 165 final Muse disposition, and every platform/release blocker |
| Supersedes in product direction | The unimplemented assumptions that one approved Gemma 4 E4B profile is the runtime prerequisite, that the model picker is hard-coded to E4B, or that first-GA success depends on a Gemma candidate passing |
| Does not authorize | Model download, model activation, code implementation, automatic routing, cloud or frontier transfer, arbitrary community-model trust, weakened provenance review, a supported-model claim, or a release claim |

## Context

The original internal-v0.1 model plan was written around one Gemma 4 E4B
candidate, a disabled Gemma 4 12B Unified fallback, and a late Meta Muse Glimmer
candidate gate. Subsequent evidence rejected the evaluated Gemma candidates and
left AgentMage with zero enabled models. The authoritative PRD and status model
record that truth, while the still-incomplete Sprint 13 plan continued to assume
that an approved E4B profile would be loaded. The assumption and the current
evidence can no longer both govern future implementation.

The clean-room research assessment at
[`docs/research/2026-08-12-clean-room-muse-glimmer-agent-harness-assessment.md`](../research/2026-08-12-clean-room-muse-glimmer-agent-harness-assessment.md)
finds that Claude Code is not an end-to-end deterministic system. Its publicly
documented architecture combines a probabilistic language-model loop and
probabilistic semantic classifiers with deterministic permission rules, typed
tool boundaries, sandboxing, budgets, and outcome-oriented verification.
AgentMage can independently implement the public architectural pattern without
using leaked proprietary source.

Meta Muse Glimmer is a promising first-party local coding and tool-use candidate
for the Fedora development workstation, but it has not passed AgentMage model,
runtime, usage-policy, hardware, quality, security, or platform admission. Its
official llama.cpp support is new. It must remain a candidate until exact
evidence supports `PASS`, `BLOCKED`, or `REJECTED`.

Google publishes multiple general, coding, function-calling, safety, embedding,
multimodal, research, specialist, current, and legacy Gemma models. Testing them
through one undifferentiated coding suite would be misleading. Excluding them
from initial comparative evidence would also make the runtime unnecessarily
model-specific. AgentMage needs a candidate-neutral construction with
role-appropriate evaluation.

## Decision

### 1. Construction principle

AgentMage will be constructed as a probabilistic local planner inside a
deterministic effect machine:

1. An admitted local model may produce text, request evidence, propose a plan,
   propose one typed tool call, ask the user, report a blocker, or propose
   completion.
2. Every model output is untrusted and non-authoritative.
3. The kernel validates exact schema, identity, snapshot, policy, operation,
   arguments, scope, budgets, and expected effects before considering authority.
4. Only the kernel can issue and consume an exact, expiring, single-use effect
   authorization.
5. A restricted platform worker performs the authorized operation.
6. A deterministic verifier or separately identified field-truth check decides
   whether required postconditions hold.
7. Model prose, model confidence, a model judge, or a classifier cannot grant
   authority or establish successful completion.

The existing Rust authority transaction, exact-object worker, path, durable
store, receipt, and strict-local work remains the foundation. This decision
changes the unimplemented model and orchestration layers rather than replacing
the kernel.

### 2. Candidate-neutral model boundary

`LocalModelRuntime` remains one provider-neutral inference contract. Model-
family behavior is separated from runtime behavior:

- A runtime adapter owns verified load, unload, health, token count, streaming,
  cancellation, resource reporting, isolation, and zero-network behavior.
- A family codec owns the exact tokenizer, chat template, reasoning controls,
  message boundaries, end tokens, tool protocol, and translation into the
  closed AgentMage proposal contract.
- A model profile binds one exact model revision, artifacts, hashes,
  transformations, tokenizer, template, codec, runtime build, quantization,
  modality set, context profile, decoding profile, platform, hardware envelope,
  evaluation result, policy version, and lifecycle state.
- The kernel and capability packs depend only on the common contracts and never
  branch directly on Muse, Gemma, or another model family.

Adding a model normally adds attributable profile data, an existing or newly
reviewed codec, and evidence. It does not add authority or require a kernel
rewrite.

### 3. Muse-first focus

Muse Glimmer is the primary implementation and deep-evaluation candidate for the
first complete local-model vertical slice. That priority creates no approval or
support claim.

The first Muse slice is:

- exact first-party, text-only artifact;
- native pinned llama.cpp on the Fedora development workstation;
- one active model and one inference slot;
- strict-local and zero egress;
- no workspace handle, tools, grants, credentials, connectors, or raw shell;
- no vision projection or speculative draft model;
- bounded 8k context first, followed only by separately measured 16k and 32k
  profiles;
- synthetic evaluation data before real repository use;
- official quality and diagnostic-repeatability profiles recorded separately.

Vision, speculative decoding, dynamic quantization, Docker Model Runner, larger
contexts, Windows, macOS, connected tools, and autonomous routing are separate
profile or platform gates and inherit no approval from the text-only slice.

### 4. Comprehensive Gemma candidate testing

The initial candidate inventory includes every eligible official first-party
Gemma model discoverable from the pinned Google source catalog at the evaluation
freeze. Each exact artifact receives identity, license/use-term, origin, lineage,
format, runtime, and hardware preflight before execution.

Testing is role-appropriate:

- general, instruction, reasoning, coding, and multimodal generative profiles
  receive applicable repository, planning, coding, tool, evidence, context,
  security, and resource suites;
- function-specialized profiles receive tool-selection and structured-proposal
  suites;
- safety profiles receive advisory-classification suites and can only deny,
  narrow, redact, or escalate;
- embedding profiles receive retrieval, provenance, contamination, invalidation,
  and resource suites;
- translation, medical, research, interpretability, legacy, and other specialist
  profiles receive only applicable role tests and cannot silently become the
  coding planner.

A model that cannot fit or run on a reference machine receives a visible
`BLOCKED-HARDWARE` result for that exact profile. It is not silently omitted and
does not block a compatible profile. Community conversions, merges, fine-tunes,
and mirrors are not first-party artifacts and do not inherit a Google result.

### 5. Extensible candidate intake

The development evaluation inventory is open to other models that satisfy the
project's existing origin, lineage, jurisdiction, license, policy, provenance,
artifact, runtime, security, and hardware eligibility rules. The current non-
Chinese and non-Chinese-derived model rule remains in force.

An eligible additional candidate enters through the same exact profile and test
contracts. It cannot become enabled merely because a compatible runtime can load
it. User-selected arbitrary or provenance-incomplete artifacts remain confined
to the separately planned post-GA Experimental Model Lab unless a later accepted
decision changes that boundary.

### 6. Candidate and product states

Model lifecycle states remain exact and distinct:

- `candidate` identifies an attributable profile awaiting evidence;
- `evaluating` permits only the declared isolated evidence run;
- `approved` permits only the capabilities and platforms proved by current
  evidence;
- `degraded` preserves an explicitly supported limited profile;
- `quarantined` prevents use pending investigation or re-review;
- `rejected` records a non-waivable failure for the exact profile;
- `retired` prevents new use after support or policy withdrawal;
- `BLOCKED-HARDWARE` is an evaluation result, not an approval state.

The current product truth remains zero enabled models. Existing rejected Gemma
records remain immutable historical evidence. A new revision, artifact,
quantization, codec, runtime, decoding, platform, or context profile requires its
own attributable admission and cannot reuse a prior result silently.

### 7. Determinism and repeatability vocabulary

AgentMage distinguishes:

- token repeatability for one pinned model/runtime/hardware tuple;
- deterministic policy decisions over the same typed facts;
- deterministic authority and effect mediation;
- independently verified outcome state;
- reproducible audit evidence.

Temperature zero and top-k one do not prove complete model determinism. Every
serious coding candidate receives two separately named profiles:

1. a **quality profile** based on the first-party recommended generation and
   reasoning settings; and
2. a **diagnostic-repeatability profile** with exact sampler order, top-k one,
   fixed seed, one slot, no speculation, bounded context, and a complete runtime,
   driver, hardware, prompt, template, tool, and artifact manifest.

Results from the two profiles are never merged. AgentMage may state that an
output repeated under a recorded tuple; it cannot claim cross-release, cross-
driver, cross-device, or universal model determinism without independent proof.

### 8. Classification and gating

AgentMage separates data sensitivity, action risk, and model capability.

1. Versioned deterministic code collects typed facts and applies deny-first
   policy over actor, session, task, autonomy, operation, source, destination,
   path, repository state, credential class, data label, reversibility, network,
   external disclosure, budget, and exact authority.
2. Static secret, path, executable-content, and destination checks run before
   semantic classification where applicable.
3. A learned or model-based classifier is advisory. It may deny, remove a tool,
   narrow scope, require redaction, require isolation, or escalate to the user.
4. A classifier cannot issue or widen a grant, override a deterministic denial,
   select an otherwise prohibited destination, mark completion, or silently
   switch models.
5. Low confidence, truncation, disagreement, unavailable classification, or
   out-of-distribution input produces a narrower boundary, a user decision, or a
   blocked result.
6. Classification repeats for newly read content, tool output, patches, diffs,
   messages, attachments, connector results, summaries, diagnostics, and export
   payloads before their next boundary.

The public ComplianceGate artifact and implementation are not adopted. Its
general classify-before-disclosure pattern may inform tests, but its non-
commercial license, experiment inconsistencies, unsafe deserialization, disabled
TLS verification, and missing product controls prevent dependency use.

### 9. Deterministic agent loop and completion

The single-agent loop uses persisted typed states and named terminal results.
Required terminal results include `SUCCESS`, verified `NO_OP`, `BLOCKED`,
`DECLINED`, `STALLED`, `EXHAUSTED`, `UNCERTAIN`, `CANCELLED`, and `FAILED`.

Only `SUCCESS` and verified `NO_OP` are successful. The kernel enforces turn,
token, context, retry, denial, tool, effect, elapsed-time, resource, and no-
progress ceilings. An interruption revalidates the task, snapshot, policy,
profile, pending authority, and any uncertain effect before resume. A consumed
grant is never replayed.

Every model proposal binds its schema, proposal, session, task, turn, model-run,
context-packet, snapshot, tool-catalog, and correlation identities. Partial,
malformed, duplicate, stale, oversized, unsupported, or ambiguous proposals are
inert. Streaming text never creates incremental tool authority.

### 10. Whole-codebase context

The existing whole-codebase audit design remains authoritative. Model use is
fed by a complete deterministic census, exact structural index, content-
addressed evidence cards, bounded coherent packets, source citations, coverage,
cross-module reconciliation, checkpoints, and dependency invalidation. The
model runtime receives no repository handle.

Muse-first integration must prove the read-only evidence path on synthetic
repositories before bounded synthetic edits, whole-codebase audits, controlled
real-workspace writes, local Git writes, or connected Git operations are
eligible.

### 11. Roadmap placement

Completed sprints and evidence are not rewritten. The first incomplete model
area is reconciled as follows:

- Sprint 12 receives explicit persisted-state, verifier-only completion,
  classifier-authority, no-progress, terminal-state, and decision-aware planning-
  validation coverage.
- Sprint 13 becomes the candidate-neutral runtime, codec, proposal, fake-adapter,
  repeatability-contract, and isolated Muse text-spike gate.
- Sprint 14 remains the separated acquisition/import boundary and gains
  candidate-inventory, exact policy-display, role, hardware, and artifact-state
  coverage.
- Sprint 15 remains diagnostics and manual selection and gains Muse-first plus
  complete eligible first-party Gemma role-matrix evaluation.
- Later read-only, repository-map, evidence, and audit sprints consume only an
  admitted selected profile.
- Sprint 49 remains the first eligible automatic measured-routing gate and still
  cannot use model self-confidence.
- Sprints 163 and 164 remain the signed public catalog and user-facing model
  manager gates.
- Sprint 165 remains the final integrated Muse disposition and trusted-
  operations reconciliation gate.
- Sprint 167 remains the post-GA arbitrary experimental-model boundary.

No sprint is renumbered. New stories and tasks are additions under their owning
future sprints.

### 12. Stable-requirement preservation

The exact text of `AM-MDL-001` through `AM-MDL-003`, `AM-MDL-002`'s explicit-
selection behavior, `AM-VSC-001`, and their existing acceptance tests remains in
the additions-only inventory. Their Gemma-specific assumptions are historical
and cannot define the new product default.

The replacement product direction is expressed through appended
`AM-MDL-004` through `AM-MDL-007`, `AM-AGT-001`, and `AM-VSC-003`, with appended
acceptance tests and dependencies. Existing requirements may gain dependencies
or acceptance coverage but lose none. Current rejected Gemma evidence is
preserved and cannot be relabeled as a pass.

## Required Verification

- Verify the model-neutral runtime with deterministic fake Muse, Gemma,
  malformed, delayed, cancelled, crashed, resource-exhausted, replayed, and
  false-completion adapters before loading real artifacts.
- Verify exact codec behavior for message boundaries, end tokens, reasoning
  controls, streaming fragments, tool calls, unknown fields, duplicate IDs,
  oversized values, invalid UTF-8, trailing data, and unsupported protocol
  versions.
- Inventory every eligible official first-party Gemma model at a pinned source
  freeze, assign one or more explicit roles, run applicable preflight/tests, and
  retain `BLOCKED-HARDWARE`, `BLOCKED`, `REJECTED`, and negative results.
- Run Muse and comparable Gemma generative profiles under the same AgentMage
  harness, fixtures, tools, context policy, budgets, graders, and platform where
  comparable; label every changed tuple.
- Run repeated stochastic trials and report pass-at-one, pass-at-k, pass-to-the-
  k, variance, confidence intervals, invalid-tool rate, false-completion rate,
  latency, memory, and user-intervention rate.
- Prove deterministic policy and grant results are invariant to model output,
  packet order, classifier output, model confidence, and selected candidate.
- Prove learned classification can only reduce authority or escalate and that
  unavailable, truncated, uncertain, or conflicting classification fails toward
  a narrower boundary.
- Prove every successful terminal result resolves to current postcondition
  evidence and that errors, exhaustion, uncertainty, cancellation, model prose,
  and model-judge output cannot become success.
- Prove the model process receives no workspace, tools, grants, credentials,
  network, broad environment, or unbounded resources.
- Prove model, tokenizer, template, codec, runtime, decoding, context, draft,
  vision, driver, hardware, policy, or test changes invalidate comparison and
  trigger the required re-review.
- Prove planning validation derives the accepted 241-record baseline from
  reconciled decision and registry truth while continuing to reject deletion,
  mutation, duplication, renumbering, stale current-direction assertions, and
  unsupported additions.

## Consequences

- AgentMage remains a universal model-neutral harness while development can
  concentrate deeply on Muse Glimmer.
- All eligible first-party Gemma models become visible, attributable test
  candidates without pretending that every model is a coding planner or fits
  every machine.
- Other eligible candidates can be evaluated without changing the kernel.
- The near-term documentation and test surface grows, but the first real model
  integration becomes an evidence-producing vertical slice rather than a hidden
  product dependency.
- Model-specific complexity moves to attributable codecs and profiles. Runtime,
  authority, context, worker, verifier, and evidence contracts remain common.
- Absolute determinism claims narrow, while deterministic authority, outcome,
  and audit guarantees become more explicit and testable.
- Automatic routing remains later work. Initial operation remains deterministic-
  first, manually selected, one-large-model-at-a-time, and fail-closed.
- No model is enabled, downloaded, supported, or released by this decision.

## Documentation-Only Verification Boundary

The pre-decision validation implementation in `scripts/status_model.py` and its
traceability regressions in `tests/test_traceability_report.py` and
`tests/test_clean_traceability.py` fix the stable-requirement count at 229; the
clean-traceability regression also fixes the normative-statement count at 26.
Decision 0027 raises the accepted additive counts to 241 requirements and 30
normative mappings, but this approved reconciliation explicitly prohibits code
changes. The documentation and generated planning artifacts therefore record
the new counts truthfully while the aggregate documentation validator remains
expected to fail on its two stale 229-count assertions and the current-state
and clean-traceability test gates remain expected to fail on their stale count
assertions until Story 12.3 implements and tests the decision-aware validator
migration.

This is a visible blocked implementation dependency, not a waiver or a passing
result. No validator is disabled, relaxed, or edited by this documentation-only
decision.

## Approval Record

On 2026-08-12, the user explicitly approved Decision 0027 and the complete
Muse-first, model-neutral documentation, requirements, tasks, sprint, and gate
reconciliation exactly as described. The approval requires preservation of all
completed work and existing requirements, prohibits code changes in this
reconciliation, and authorizes granular documentation commits and pushes until
the decision is fully represented and verified.
