#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Local host composition for the bounded AgentMage product surface.

/// Authority-free composition of structured edits into kernel shadow changes.
pub mod code_change;

/// One-use reviewed local diagnostic export workflow.
pub mod diagnostic_export;

/// Authority-free knowledge-preview composition into kernel filesystem drafts.
pub mod knowledge_write;

pub mod protocol;

/// Versioned thin-client contracts shared by terminal and headless interfaces.
pub mod headless;

#[cfg(target_os = "linux")]
pub mod linux_read;

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "shell-host";
