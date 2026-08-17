# Shared Runtime, MCP, and Workflow Local Results

## Scope

This local source campaign covers Story 50.2 caller authority and lifecycle,
the dependency-safe Sprint 95 shared-runtime attachment, and the implemented
Sprint 80-81 MCP contract, manifest, identity, registry, and gateway slices.

## Commands

```text
cargo test -p agentmage-kernel-contracts --lib
cargo test -p agentmage-kernel-engine mcp_ --lib
cargo test -p agentmage-kernel-engine workflow_authority --lib
cargo test -p agentmage-host --lib
cargo clippy -p agentmage-kernel-engine --lib -- -D warnings
cargo clippy -p agentmage-host --lib -- -D warnings
cargo test --workspace --lib
cargo clippy --workspace --lib -- -D warnings
npm run docs:check
```

## Implemented Evidence

| Area | Source | Local evidence |
|---|---|---|
| Workflow authority | `kernel/engine/src/workflow_authority.rs` | Seven required layers, narrowing-only intersection, malformed-layer and aggregation rejection |
| Caller lifecycle | `shells/host/src/workflow_caller.rs` | Sealed submission, ordered events, explicit approval resume, terminal evidence and artifacts |
| Interactive CLI client | `shells/host/src/cli_runtime.rs` | Same host-framed runtime port as native Chat; independent request, stream, artifact, approval, cancellation, outcome, and release verification |
| Three-client parity | `shells/host/src/runtime_parity_tests.rs` | Identical read-only and controlled-write requests across native Chat, interactive CLI, and workflow caller; exact event, artifact, receipt, checkpoint, evidence, and outcome equality |
| Authority attack matrix | `shells/host/src/runtime_parity_tests.rs` | Eight narrower-node broadening attempts rejected before one instrumented coordinator advance |
| Confusion and privacy matrix | Runtime event, journal, projection, artifact, recovery, and CLI verifier modules | Eight required confusion classes reject deterministically without false completion, cross-session disclosure, replay, or unverified presentation |
| Real runtime attachment | `shells/host/src/linux_coding_runtime.rs` | Deterministic Linux fake-model no-op and denial paths through the production coordinator composition |
| Child attachment | `shells/host/src/workflow_assignment.rs` | Bound ancestry/dependencies/depth and authority-free pending-review result |
| MCP contracts | `kernel/contracts/src/mcp.rs` | Closed transport, manifest, request, response, cancellation, disconnect, error, and receipt schemas |
| MCP identity | `kernel/engine/src/mcp_registry.rs` | Manifest sealing, exact identity revalidation, expiry, non-shadowing optional common-registry adaptation |
| MCP mediation | `kernel/engine/src/mcp_gateway.rs` | One-use requests, bounded untrusted responses, lifecycle receipts, verified cancellation and disconnect cleanup |

## Confusion and Privacy Matrix

| Required attack class | Exact local evidence | Fail-closed result |
|---|---|---|
| Transcript confusion | `story_50_2_verified_runtime_exchange_projects_valid_non_authoritative_turns`; `story_50_2_transcript_projection_obeys_opt_in_safe_mode_sensitivity_and_budget` | Altered source text, forged authority, broken source links, disabled collection, safe mode, restricted data, and over-budget projection are rejected |
| Journal confusion | `ephemeral_events_and_failed_batches_leave_no_false_history`; `retained_event_tampering_blocks_restart`; `retained_event_projection_tampering_blocks_restart` | Invalid persistence and failed batches create no false history; retained-byte or indexed-field tampering blocks restart |
| Metrics confusion | `story_50_2_metrics_and_diagnostics_are_content_free_event_bound_and_bounded`; `story_50_2_collection_overage_and_binding_drift_are_non_mutating` | Malformed, replayed, over-budget, or foreign-session telemetry is rejected without extending collected state |
| Artifact-reference confusion | `manifest_digest_or_reference_drift_fails_closed`; `story_50_2_artifact_pages_are_owner_bound_bounded_and_exact`; `publication_deduplicates_without_broadening_owner_or_reference_state` | Reference drift, invalid paging, and deduplication cannot broaden ownership or reveal bytes |
| Cross-session access | `story_50_2_artifact_pages_are_owner_bound_bounded_and_exact`; `story_50_2_collection_overage_and_binding_drift_are_non_mutating`; `story_50_2_confusion_and_interface_bypasses_never_present_or_complete` | Foreign artifact reads and re-signed foreign-session records are denied without disclosure or presentation |
| Stale checkpoints | `story_50_2_missing_stale_or_invalid_durable_evidence_enters_safe_mode`; `continuation_digest_binds_cursor_and_collected_state`; `failed_resume_binding_rolls_back_the_checkpoint_and_requires_reopen` | Stale, missing, altered, or partly committed resume evidence cannot resume; replay remains disabled |
| Duplicate and malformed terminal events | `story_21_2_sequence_rejects_reorder_replay_binding_and_post_terminal_events`; `story_50_2_confusion_and_interface_bypasses_never_present_or_complete` | Duplicate events and a validly re-signed terminal state inconsistent with its outcome are rejected before presentation or completion |
| Interface-specific bypass | `request_event_artifact_and_outcome_substitution_fail_before_success`; `story_50_2_confusion_and_interface_bypasses_never_present_or_complete` | Forged request, run, event, artifact, outcome, or terminal bindings produce no client-visible events and no success result |

## Truthful Disposition

Focused local tests, the workspace library suite, and strict workspace Clippy
pass. The complete host library reports 121 passing tests and three ignored
tests that require the Fedora or Ubuntu systemd user session, Bubblewrap, or
built worker manifests. Those ignores are not converted into passes.

Authenticated CLI stdin/history and installed transport, a production runtime
factory, live MCP transport, installed real-runtime client parity,
reference-hardware campaigns, Sprints 89-94, complete Sprint 95
coordination, supported-platform evidence, independent review, and deferred
manual fuzzing remain open. No sprint or release gate is closed by this report.
