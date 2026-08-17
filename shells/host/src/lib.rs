#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Local host composition for the bounded AgentMage product surface.

/// Authority-free composition of structured edits into kernel shadow changes.
pub mod code_change;
pub mod coding_changes;
pub mod coding_dispatch;
pub mod coding_projection;
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
pub mod linux_read;

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "shell-host";

#[cfg(test)]
mod runtime_read_tests;
