#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Local host composition for the bounded AgentMage product surface.

/// Deterministic in-memory ingestion over exact captured artifact bytes.
pub mod artifact_ingestion;

/// Authority-free composition of structured edits into kernel shadow changes.
pub mod code_change;
/// Exact non-authoritative approval composition for native coding operations.
pub mod coding_authority;
pub mod coding_changes;
/// Thin presentation and approval clients for the shared coding runtime.
pub mod coding_client;
/// Bounded model-context composition for shared coding sessions.
pub mod coding_context;
pub mod coding_dispatch;
/// Deterministic narrowing of coding profiles from explicitly trusted guidance.
pub mod coding_guidance;
/// Interface-neutral composition for the shared local coding runtime.
pub mod coding_harness;
/// Profile-bound authority planning for native coding calls.
pub mod coding_operation;
/// Verified descriptive planning identity for bounded coding sessions.
pub mod coding_plan;
pub mod coding_projection;
/// Exact interface-neutral runtime request framing for coding sessions.
pub mod coding_run;
pub mod coding_session;
pub mod coding_tools;
/// Deterministic completion verification for bounded coding sessions.
pub mod coding_verifier;

/// One-use reviewed local diagnostic export workflow.
pub mod diagnostic_export;
/// Authority-free product composition for local document-control workflows.
pub mod document_control_coordinator;
/// Qualified local/private/managed Engineering model gateway composition.
pub mod engineering_gateway;
/// Product-qualified exact-profile model bridge for Verified Chat.
pub mod engineering_model;
/// Durable session, event, and exact artifact RPC composition.
pub mod engineering_runtime;
pub mod engineering_team;
/// Authority-free product composition for local executive-assistant projections.
pub mod executive_coordinator;
/// Closed independent activation for implemented and unavailable runtime features.
pub mod feature_activation;
/// Fail-closed beta, rollback, and emergency-disable rollout for foundational features.
pub mod foundational_rollout;
/// Bounded local-only coordination for measured frontier recommendation packets.
pub mod frontier_coordinator;

/// Durable, authority-free routing of imported frontier proposals into native local flows.
pub mod frontier_import_coordinator;

/// Durable manual-frontier composition across recommendation, export, and local import.
pub mod frontier_release_coordinator;

/// Verified read-only knowledge workflows over the shared thin-client contract.
pub mod knowledge_workflow_runtime;
/// Authority-free knowledge-preview composition into kernel filesystem drafts.
pub mod knowledge_write;
/// Authority-free composition for Markdown artifact and exact-edit workflows.
pub mod markdown_artifact_coordinator;
/// Authority-free product composition for source-preserving meeting workflows.
pub mod meeting_coordinator;

#[cfg(feature = "native-chat")]
/// Native Chat registry and adapter over the caller-neutral runtime transport.
pub mod native_chat_runtime;

/// Caller-neutral transport contract shared by authenticated local clients.
pub mod runtime_transport;

pub mod protocol;

/// Native capability registration for the shared runtime tool dispatcher.
pub mod runtime_tools;
/// Production prepared-source adapter for the common artifact dispatcher.
#[cfg(feature = "source-artifacts")]
pub mod source_artifact_runtime;
#[cfg(feature = "workflow-caller")]
/// Narrow child-assignment adapter over the shared workflow caller runtime.
pub mod workflow_assignment;
#[cfg(feature = "workflow-caller")]
/// Caller-neutral, narrowing-only workflow attachment to the shared coding runtime.
pub mod workflow_caller;

/// Host composition from the verified workflow supervisor into the existing coding coordinator.
#[cfg(feature = "workflow-supervisor")]
pub mod workflow_supervisor;

/// Authority-free product composition for bounded Word artifact workflows.
#[cfg(feature = "source-artifacts")]
pub mod word_artifact_coordinator;
/// Runtime-owned DOCX preparation and common native artifact-tool projection.
#[cfg(feature = "source-artifacts")]
pub mod word_source_artifact;

/// Versioned thin-client contracts shared by terminal and headless interfaces.
pub mod headless;

#[cfg(feature = "interactive-cli")]
/// Strict terminal argument parsing and bounded human or JSON rendering.
pub mod cli;
#[cfg(feature = "interactive-cli")]
/// Verified interactive CLI driver over the shared caller-neutral runtime transport.
pub mod cli_runtime;

#[cfg(target_os = "linux")]
/// Linux owned-worktree binding for native coding operations.
pub mod linux_coding;

#[cfg(target_os = "linux")]
/// Linux approval, authority, and effect composition for the reusable coding runtime.
pub mod linux_coding_runtime;

#[cfg(target_os = "linux")]
/// Held-object Linux projection into the pure repository-map capability.
pub mod linux_repository_map;

#[cfg(target_os = "linux")]
/// Held-object Linux citation resolution for evidence reconciliation.
pub mod linux_evidence_reconciliation;

#[cfg(target_os = "linux")]
pub mod linux_read;

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "shell-host";

#[cfg(all(
    test,
    feature = "interactive-cli",
    feature = "native-chat",
    feature = "workflow-caller",
    feature = "source-artifacts",
    feature = "workflow-supervisor"
))]
mod runtime_parity_tests;
#[cfg(all(test, feature = "source-artifacts", feature = "workflow-supervisor"))]
mod runtime_read_tests;
