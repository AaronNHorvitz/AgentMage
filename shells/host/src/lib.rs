#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Local host composition for the bounded AgentMage product surface.

/// Authority-free composition of structured edits into kernel shadow changes.
pub mod code_change;
/// Bounded model-context composition for shared coding sessions.
pub mod coding_context;
/// Exact non-authoritative approval composition for native coding operations.
pub mod coding_authority;
pub mod coding_changes;
pub mod coding_dispatch;
/// Profile-bound authority planning for native coding calls.
pub mod coding_operation;
pub mod coding_projection;
/// Exact interface-neutral runtime request framing for coding sessions.
pub mod coding_run;
pub mod coding_session;
pub mod coding_tools;

/// One-use reviewed local diagnostic export workflow.
pub mod diagnostic_export;

/// Authority-free knowledge-preview composition into kernel filesystem drafts.
pub mod knowledge_write;

pub mod protocol;

/// Native capability registration for the shared runtime tool dispatcher.
pub mod runtime_tools;

/// Versioned thin-client contracts shared by terminal and headless interfaces.
pub mod headless;

/// Strict terminal argument parsing and bounded human or JSON rendering.
pub mod cli;

#[cfg(target_os = "linux")]
/// Linux owned-worktree binding for native coding operations.
pub mod linux_coding;

#[cfg(target_os = "linux")]
/// Linux approval, authority, and effect composition for the reusable coding runtime.
pub mod linux_coding_runtime;

#[cfg(target_os = "linux")]
pub mod linux_read;

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "shell-host";

#[cfg(test)]
mod runtime_read_tests;
