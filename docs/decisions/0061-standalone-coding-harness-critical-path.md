# Decision 0061: Standalone Coding Harness as the Immediate Critical Path

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-21 |
| Authority | Decision 0054 and the owner's explicit 2026-09-21 stop-and-replan instruction |
| Scope | Standalone AgentMage coding workflow, OpenCode architectural reference, delivery order and executable acceptance |
| Supersedes | Conflicting work-selection order in Decisions 0048, 0052 and 0053 for this workstream only; the sequencing ambiguity in Implementation Plan Section 12.1 |
| Preserves | Existing requirement and sprint identities, completed component evidence, security boundaries, license, independent review and release gates |

## Owner Direction

The owner requested that the AgentMage development agent be stopped and that
OpenCode's framework and the existing AgentMage design be integrated into the
documents and architecture. The owner identified independent coding capability
as a central AgentMage responsibility, with correctness taking priority over
elapsed development time. This records architecture and planning authorization,
not a claim that the implementation or its acceptance tests have been completed.

The prior `agentmage-claude` worker was stopped before this revision. Its four
polling shells were terminated and its tmux session closed. The operator stop
marker remains at `~/.local/share/agentmage-run/STOP-CLAUDE`. Other project workers
were not stopped. This decision does not restart an agent or override that marker.

## Diagnosis

At AgentMage revision `6f90f81cbeaa930674a2f98e8ee27bfd8814a59b`,
`shells/host/src/bin/agentmage.rs` unconditionally constructs
`ThinClientError::TransportFailed` for operational commands. No connection is
attempted. The ordinary host entry point does not assemble a coding service;
the package-verified bootstrap ends at `platform_activation_required`.
`NativeChatRuntimeFactory` has replay implementations in tests but no installed
real-model factory at this boundary. The reusable coordinator, native tools,
policy, verifier and CLI driver are valuable component work, not an executable
coding product.

The real-model fixture in Story 48.2 depends on a Sprint 49 admission bundle,
while Sprint 49's broad gate depends on Sprint 48. The exact admission work must
be delivered before the coding acceptance test without pretending either whole
sprint has passed. Likewise, durable session features cannot be prerequisites
for the earlier explicitly ephemeral milestone.

## Decision

1. AgentMage owns a complete single-agent coding workflow. CodingMage, USTE,
   AgentMagik, OpenCode, a cloud account, MCP and multi-agent orchestration are
   not prerequisites for its first working Linux coding session.
2. Adopt the connected session, prompt/tool feedback, event, approval,
   cancellation, diff-inspection and continuation patterns examined in OpenCode
   revision `e059ac5918f3e2c798de029b9df4cede617466ed`. Adapt those patterns into
   AgentMage's existing Rust runtime and thin-client contracts. Do not embed or
   launch OpenCode's execution engine, introduce a second agent loop, or claim
   OpenCode protocol/binary compatibility. This is architectural integration,
   not vendoring. Any later source reuse needs separate exact-file provenance,
   dependency, notices and license review; this decision changes no license.
3. Keep the kernel's exact grants, held-target enforcement, confined platform
   workers, canonical stores, context accounting and verifier-only completion.
   An OpenCode-style approval UI is not a substitute for these controls.
4. Make Tasks 48.2.4 through 48.2.6 the immediate delivery workstream. Preserve
   their exact prerequisite rows and select only dependency-ready work. The
   remaining-plan selector prioritizes these additions and their explicitly
   recorded prerequisites over unrelated desktop, enterprise and multi-agent
   expansion. Do not erase external blockers to obtain a runnable queue.
   New rows record their complete entry dependencies on the numbered row itself;
   task parents additionally depend on their own incomplete children. Trailing
   story/release commentary is not a task-entry dependency and cannot make a
   task depend on its own story's closure. Existing roadmap parsing is retained.
5. `M-HARNESS-MVP` requires the real CLI and host, authenticated local transport,
   a production-capable factory, live bounded events/control, protected
   approvals, one exact qualified local coding profile, real tools, failed-test
   correction and verifier-backed results. The new tasks strengthen the existing
   milestone; they do not replace its stale, adversarial or exclusion fixtures.
6. Task 48.2.4 owns delivery of the exact model-admission prerequisite together
   with the existing model/platform owners. It does not wait for the broad
   Sprint 49 routing/release gate and does not close that gate. Every admission
   check still applies. A document-QA demo model, synthetic model, downloaded
   file, or successful direct model-server response is not coding qualification.
7. Define `M-HARNESS-DAILY` in Task 50.2.4 as the next internal quality threshold:
   installed-session continuity, source-backed context management, inspectable
   outputs, conflict-aware recovery, bounded session preauthorization, setup
   diagnostics, soak evidence and independent review. It is not a new sprint or
   a release. Do not stop the product plan at a one-task demonstration.
   Use exact implemented storage/context contracts as entry dependencies and
   deliver the remaining Linux coding integration with those existing owners.
   Do not require whole-story platform/release closure before implementing the
   integration that contributes to it. All applicable G1/G2 recording,
   reopening, privacy and fault cases remain required; no second storage or
   continuity implementation is authorized.
8. Development activation is explicit work, not a bypass. Task 48.2.4.1 records
   the exact launch/trust/resource/model prerequisites and chooses an existing
   approved path or proposes a separately accepted development-only profile.
   No test key, unsigned package, arbitrary endpoint or demo-only exception may
   enter production activation. This decision grants no production signing,
   model activation, credential, publication or release authority.
9. Keep existing checkbox meanings and historical evidence. New implementation
   tasks start unchecked. Record component, executable-fixture, real-model,
   native-platform and independent-review evidence separately using the existing
   lifecycle/verification model. No status promotion follows from this document.
10. Batch implementation before regenerating affected evidence. Never weaken
    hashes, tests, thresholds or the task graph to obtain a pass. Planning checks
   prove planning consistency, not coding execution.

The existing schema-evolution and parser-placement records retain every control
and implementation-truth field. Their documentary authority digests are renewed
for the amended `ENGINEERING-RUNTIME.md` and `RUNTIME-BOUNDARIES.md`; the current
contract boundary/evidence bundle is regenerated through its existing commands.
This is whole-file binding renewal, not an exemption or historical evidence rewrite.

## Architecture and Acceptance

The detailed ownership, protocol changes, asynchronous execution, effect safety,
context, milestones, source comparison and acceptance matrix are specified in
[Standalone Coding Harness](../architecture/standalone-coding-harness.md).
PRD Section 40 and Implementation Plan Section 15 place this within the product.
The live restart instruction is
[the implementation handoff](../verification/coding-harness-replan-2026-09-21.md).

The decisive acceptance path launches the actual binaries in a disposable
repository, explores code, proposes and applies an authorized patch, runs a
targeted test, observes an actual failure, corrects it within budget, and emits
a verified report. Denial, cancellation, stale approvals, forged progress,
overflow and uncertain effects produce truthful non-success and no unauthorized
or delayed mutation. Human staged, unstaged and untracked work remains intact.

## Reference Boundary

OpenCode is a workflow reference, not an authority or security dependency.
Its pinned [security policy](https://github.com/anomalyco/opencode/blob/e059ac5918f3e2c798de029b9df4cede617466ed/SECURITY.md)
explicitly distinguishes permission prompts from sandboxing. Keep AgentMage's
confinement and admission controls. Do not import third-party project
instructions, run upstream installers, or treat upstream tests as AgentMage
acceptance evidence.
