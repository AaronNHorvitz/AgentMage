#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Security-authoritative AgentMage kernel scaffold.

/// Closed dependency degradation and explicit substitution policy.
pub mod dependency_degradation;

/// Authority-reducing application of advisory classifier output.
pub mod advisory_policy;
pub mod agent_ceiling;
/// Declarative specialist-agent profiles, registry, lint, and synthetic evaluation.
pub mod agent_profiles;
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
/// Digest-first artifact registry observation and promotion admission.
pub mod artifact_promotion;
/// Metadata-only attachment classification and exact workspace path resolution.
pub mod attachment;
/// Sealed classification and denial of descriptive artifacts as authority.
pub mod authority;
/// Kernel-owned authority-transaction ordering and recovery contract.
pub mod authority_transaction;
pub mod autonomy_center;
/// Complete backup-domain manifests and non-mutating migration admission.
pub mod backup_migration;
/// Closed engineering-capability lifecycle and dependency qualification.
pub mod capability_lifecycle;
/// Versioned executable engineering capability admission and deterministic execution.
pub mod capability_registry;
/// Exact child-authority intersection, isolation, ownership, and bounded cancellation.
pub mod child_authority;
/// Provider-neutral continuous-integration observation and effect admission.
pub mod ci_control;
/// Deterministic material-claim proof and truthful final-response construction.
pub mod claim_evidence;
/// Exact command templates, previews, execution permits, and terminal receipts.
pub mod command_runner;
/// Versioned, fail-closed configuration loading and recovery.
pub mod configuration;
pub mod connected_identity;
/// Deterministic bounded context, checked summaries, checkpoints, and drift gates.
pub mod context_management;
/// Separately keyed private conversation archives and controlled lifecycle operations.
pub mod conversation_archive;
/// Encrypted canonical conversation records and immutable turn timelines.
pub mod conversation_library;
/// Interface-invariant authority, isolation, recovery, and privacy assurance.
pub mod cross_interface_assurance;
/// Provider-neutral delivery graph and non-inheriting adapter conformance.
pub mod delivery_graph;
/// Exact offline deployment planning, effect verification, and rollback admission.
pub mod deployment_safety;
/// Kernel-owned redacted local doctor report construction.
pub mod diagnostics;
/// Deterministic document registers, records review, and exact local action previews.
pub mod document_control;
/// Durable attempt checkpoints and deterministic no-replay restart decisions.
pub mod durable_attempt_recovery;
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
pub mod external_effect;
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
/// Exact GitOps observation, bounded synchronization, and controller reconciliation.
pub mod gitops_control;
/// Kernel-only session and operation grant issuance.
pub mod grants;
/// Deterministic local-only manual handoff construction and denial.
pub mod handoff;
/// Evidence-linked incidents, separately granted effects, and duplicate-safe recovery.
pub mod incident_lifecycle;
/// Exact infrastructure plan admission, apply verification, and recovery.
pub mod infrastructure_safety;
/// Hash-bound discovery, reading, and narrowing-only trust for untrusted instructions.
pub mod instruction_provenance;
/// Deterministic resumable jobs, leases, read-only schedules, notifications, and receipts.
pub mod job_scheduler;
/// Exact candidate-tree, signer, manual approval, and signed local commit contracts.
pub mod local_commit;
/// Safe-mode capability closure and content-free maintenance diagnostics.
pub mod maintenance_recovery;
/// Source-preserving Markdown mathematics and offline-preview admission.
pub mod markdown_math;
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
/// Non-enabling composition of exact qualified roles and capabilities.
pub mod pod_composition;
/// Deterministic deny-first capability-grant policy evaluation.
pub mod policy;
/// Deterministic static and typed-fact checks before advisory classification.
pub mod preclassification_policy;
/// Versioned productivity-pack manifests, lifecycle, discovery, and data-flow admission.
pub mod productivity_pack;
/// Deterministic authority-free local workflows for specialist profiles.
pub mod profile_workflows;
/// Cancellation trees and lossless typed failure propagation.
pub mod propagation;
/// Bounded concise reasoning records and deterministic verification gates.
pub mod reasoning;
/// Continuous content reclassification before successive trust boundaries.
pub mod reclassification;
/// Exact release manifests, separately granted changes, and safe compensation.
pub mod release_lifecycle;
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
/// Read-only execution inspection and identity-bound performance qualification.
pub mod runtime_inspector;
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
/// Predeclared, threat-modeled unattended state-change admission without effect authority.
pub mod scheduled_authority;
/// Evidence-backed service catalog identities and inert extension candidates.
pub mod service_catalog;
/// Bounded, configuration-bound session environment and provenance capture.
pub mod session_environment;
/// Atomic source refresh, invalidation, retention, hold, deletion, and collection.
pub mod source_lifecycle;
/// Production text/log source admission, extraction, retrieval, and context accounting.
#[cfg(feature = "source-preparation")]
pub mod source_preparation;
/// Prepared-source delivery through the reusable model-context port.
#[cfg(feature = "source-preparation")]
pub mod source_runtime_context;
/// Strict-local endpoint policy, storage admission, and content-free attempt ledger.
pub mod strict_local;
/// Exact supply-chain evidence and conflict-preserving security findings.
pub mod supply_chain_evidence;
/// Deterministic descriptive task intent, complexity, and risk classification.
pub mod task_classification;
/// Bounded telemetry identity, correlation, aggregation, and authority refusal.
pub mod telemetry_correlation;
/// Deterministic tool-call normalization and single profile-bound repair admission.
pub mod tool_call_repair;
/// Deterministic native-tool preflight, attempt, launch, and verification composition.
pub mod tool_composition;
/// Closed terminal tool-observation assembly and output-artifact binding.
pub mod tool_observation;
/// Exact tool registration, call validation, and pre-grant dispatch denial.
pub mod tooling;
/// Explicit offline update staging and fail-closed supply-chain maintenance.
pub mod update_maintenance;
pub mod validation_result;
/// Trusted validation-template provenance, exact registries, and focused selection.
pub mod validation_template;
/// Exact vendor telemetry identities, bounded queries, and observe-only authority.
pub mod vendor_observability;
/// Exact chunked source-artifact capture and bounded retrieval for Verified Chat.
pub mod verified_artifact;
/// Deterministic context admission and model-delivery coverage receipts.
pub mod verified_context;
/// Dependency-ready orchestration over exact reusable-runtime step attempts.
#[cfg(feature = "verified-workflow-supervisor")]
pub mod verified_workflow_supervisor;
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
