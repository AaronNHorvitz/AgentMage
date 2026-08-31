#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Security-authoritative AgentMage kernel scaffold.

/// Authority-reducing application of advisory classifier output.
pub mod advisory_policy;
pub mod agent_ceiling;
/// One-active-step plan history, progress, interruption, and response validation.
pub mod agent_progress;
pub mod agent_proposal;
pub mod agent_restart;
/// Bounded single-agent observe, plan, action-proposal, and review control loop.
pub mod agent_runtime;
pub mod agent_state;
pub mod agent_verifier;
/// Deterministic non-authoritative approval-display construction.
pub mod approval;
/// Metadata-only attachment classification and exact workspace path resolution.
pub mod attachment;
/// Sealed classification and denial of descriptive artifacts as authority.
pub mod authority;
/// Kernel-owned authority-transaction ordering and recovery contract.
pub mod authority_transaction;
/// Versioned executable engineering capability admission and deterministic execution.
pub mod capability_registry;
/// Deterministic material-claim proof and truthful final-response construction.
pub mod claim_evidence;
/// Exact command templates, previews, execution permits, and terminal receipts.
pub mod command_runner;
/// Versioned, fail-closed configuration loading and recovery.
pub mod configuration;
/// Deterministic bounded context, checked summaries, checkpoints, and drift gates.
pub mod context_management;
/// Separately keyed private conversation archives and controlled lifecycle operations.
pub mod conversation_archive;
/// Encrypted canonical conversation records and immutable turn timelines.
pub mod conversation_library;
/// Kernel-owned redacted local doctor report construction.
pub mod diagnostics;
/// Deterministic document registers, records review, and exact local action previews.
pub mod document_control;
pub mod engineering_execution;
/// Kernel-enforced narrowing-only ceilings for Verified Chat operating modes.
pub mod engineering_mode;
/// SQLCipher-backed Engineering Runtime session, event, and source-artifact persistence.
pub mod engineering_persistence;
/// Exact non-authoritative approvals of durable Verified Chat Plan artifacts.
pub mod engineering_plan;
/// Semantic validation and sealing helpers for canonical Engineering Runtime records.
pub mod engineering_records;
/// Exact-preview redacted evidence bundles derived from canonical conversation state.
pub mod evidence_bundle;
/// Fresh citation resolution, complete answer ledgers, and keyed receipt integrity.
pub mod evidence_reconciliation;
/// Deterministic assignment of explicit evidence states to material claims.
pub mod evidence_state;
pub mod evidence_store;
/// Deterministic executive-assistant views, ranking, drafting, and local triage.
pub mod executive_assistant;
/// Authority-free controlled filesystem plans, structured patches, and exact previews.
pub mod filesystem_control;
/// Non-executing frontier-result import, quarantine, and local revalidation.
pub mod frontier_import;
/// Measured frontier recommendation and local-only disclosure packet composition.
pub mod frontier_recommendation;
/// v0.5 manual-frontier release compositions and controlled local export.
pub mod frontier_release;
/// Candidate-neutral disabled gateway tuple identity and admission.
pub mod gateway_candidate_identity;
/// Versioned codec capability validation and deterministic no-silent-fallback routing.
pub mod gateway_routing;
/// Kernel-only session and operation grant issuance.
pub mod grants;
/// Deterministic local-only manual handoff construction and denial.
pub mod handoff;
/// Hash-bound discovery, reading, and narrowing-only trust for untrusted instructions.
pub mod instruction_provenance;
/// Exact candidate-tree, signer, manual approval, and signed local commit contracts.
pub mod local_commit;
/// Stateful read-only MCP request mediation, response validation, and receipts.
pub mod mcp_gateway;
/// Read-only MCP manifest admission, identity binding, and optional tool adaptation.
pub mod mcp_registry;
/// Authority-free meeting preparation, source-preserving minutes, and continuity.
pub mod meeting_continuity;
/// Exact non-activating evaluation gate for alternate local model runtimes.
pub mod model_adapter_evaluation;
pub mod model_codec;
/// Candidate-neutral native-picker projection and stale-selection refusal.
pub mod model_discovery;
/// Deterministic local and remote model endpoint validation, routing, and protocol encoding.
pub mod model_gateway;
/// Checked model-specific context allocation and authority-invariant orchestration profiles.
pub mod model_orchestration_profile;
pub mod model_response;
/// Deterministic measured local routing over exact profile and role evidence.
pub mod model_routing;
/// Candidate-neutral exact-profile admission and local runtime orchestration.
pub mod model_runtime;
/// Explicit exact-profile selection, deterministic-first dispatch, and resource control.
pub mod model_selection;
/// Dependency-aware multi-agent scheduling, leases, review, correction, and integration.
pub mod multi_agent;
/// Encrypted canonical operational state and crash recovery.
pub mod operational_store;
/// Deterministic pre-persistence classification, minimization, and receipts.
pub mod persistence;
/// Persistent Engineering Runtime session supervision and ordered event replay.
pub mod persistent_supervisor;
/// Fail-closed platform capability detection and adapter activation.
pub mod platform_startup;
/// Deterministic deny-first capability-grant policy evaluation.
pub mod policy;
/// Deterministic static and typed-fact checks before advisory classification.
pub mod preclassification_policy;
/// Cancellation trees and lossless typed failure propagation.
pub mod propagation;
/// Bounded concise reasoning records and deterministic verification gates.
pub mod reasoning;
/// Continuous content reclassification before successive trust boundaries.
pub mod reclassification;
/// Encrypted derivative repository-map cache lifecycle.
pub mod repository_cache;
/// Typed, read-only Git inspection plans and nonforgeable execution permits.
pub mod repository_inspection;
/// Content-minimized repository state, exact Git plans, and owned-worktree lifecycle.
pub mod repository_safety;
/// Deterministic compilation of all prerequisites for one genuinely fresh attempt.
pub mod retry_admission;
/// Integrity-protected review packets and authority-free logical commit plans.
pub mod review_packet;
/// Stateful resource budgets and explicit sticky stop conditions.
pub mod run_control;
/// Hash-bound evidence assignment required by successful rendered runtime answers.
pub mod runtime_answer;
/// Closed runtime artifact manifests, references, and resume bindings.
pub mod runtime_artifact;
/// Reusable runtime request admission, outcome verification, and coordinator composition.
pub mod runtime_coordinator;
/// Hash-chained runtime events and bounded ordered client publication.
pub mod runtime_event;
/// Request-bound runtime resource ceilings and content-free accounting.
pub mod runtime_hardening;
/// Bounded durable runtime-event batching over the canonical encrypted store.
pub mod runtime_journal;
/// Cross-domain runtime lifecycle planning without aggregated authority.
pub mod runtime_lifecycle;
/// Interface-independent reusable runtime coordinator.
pub mod runtime_loop;
/// Optional bounded transcript, metric, and diagnostic projections.
#[cfg(feature = "runtime-projections")]
pub mod runtime_projection;
/// Deterministic, non-authoritative selection of one safe action after runtime interruption.
pub mod runtime_recovery;
/// Bounded, configuration-bound session environment and provenance capture.
pub mod session_environment;
/// Atomic source refresh, invalidation, retention, hold, deletion, and collection.
pub mod source_lifecycle;
/// Strict-local endpoint policy, storage admission, and content-free attempt ledger.
pub mod strict_local;
/// Deterministic descriptive task intent, complexity, and risk classification.
pub mod task_classification;
/// Deterministic tool-call normalization and single profile-bound repair admission.
pub mod tool_call_repair;
/// Deterministic native-tool preflight, attempt, launch, and verification composition.
pub mod tool_composition;
/// Closed terminal tool-observation assembly and output-artifact binding.
pub mod tool_observation;
/// Exact tool registration, call validation, and pre-grant dispatch denial.
pub mod tooling;
pub mod validation_result;
/// Trusted validation-template provenance, exact registries, and focused selection.
pub mod validation_template;
/// Exact chunked source-artifact capture and bounded retrieval for Verified Chat.
pub mod verified_artifact;
/// Deterministic context admission and model-delivery coverage receipts.
pub mod verified_context;
/// Bounded work-packet validation, revision history, and plan adaptation.
pub mod work_packet;
/// Narrowing-only authority intersection for future workflow callers.
pub mod workflow_authority;
/// Immutable multi-dimensional workflow-supervision policy and checked budget accounting.
pub mod workflow_budget;
/// Fail-closed admission of immutable workflow graphs and their exact step policies.
pub mod workflow_definition;
/// Atomic issuance and replay denial for workflow execution identities.
pub mod workflow_identity;
/// Complete state fingerprints and policy-bound repeated no-progress detection.
pub mod workflow_progress;
/// Fail-closed reconciliation for durable workflow checkpoints and restart observations.
pub mod workflow_resume;
/// Verifier-owned construction of the closed workflow terminal outcome family.
pub mod workflow_terminal;
/// Non-secret terminal reasons and inert safe next actions for workflow supervision.
pub mod workflow_termination;
/// Deterministic evaluation of exact workflow evidence without completion authority.
pub mod workflow_verifier;
/// Exact-preimage shadow changes, review previews, and bounded write grants.
pub mod write_approval;
/// Content-free write checkpoints, privacy gates, recovery decisions, and audit receipts.
pub mod write_recovery;
/// Grant-consuming atomic write coordination, receipts, restoration, and rollback proposals.
pub mod write_transaction;

#[cfg(test)]
mod s012_it01;
#[cfg(test)]
mod s012_rt01;
#[cfg(test)]
mod s012_ut02;
#[cfg(test)]
mod test_target;

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "kernel-engine";

/// Returns the identity of the contract package used by the kernel.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}

#[cfg(test)]
mod tests {
    use super::{COMPONENT_ID, contract_component_id};

    #[test]
    fn dependency_points_toward_contracts() {
        assert_eq!(COMPONENT_ID, "kernel-engine");
        assert_eq!(contract_component_id(), "kernel-contracts");
    }
}
