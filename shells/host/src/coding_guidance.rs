//! Deterministic execution narrowing from explicitly trusted repository guidance.

use std::collections::BTreeSet;
use std::fmt::Write;

use agentmage_kernel_contracts::{
    RuntimeRunLimits, ToolDefinition, ToolId, WorkspaceId, WorkspaceScopePath,
};
use agentmage_kernel_engine::{
    instruction_provenance::{
        EffectiveGuidance, GuidanceConstraintKind, InstructionEvidenceLedger,
        InstructionTrustDisposition, effective_guidance,
    },
    runtime_coordinator::validate_runtime_run_limits,
    tooling::ToolRegistry,
    validation_template::ValidationTemplateRegistry,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Stable content-free refusal while compiling trusted narrowing guidance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingGuidanceError {
    /// The ledger, derived guidance, decision scope, or decision relationship is invalid.
    SourceDenied,
    /// A constraint target is malformed, unknown, widening, or unsupported.
    ConstraintDenied,
    /// Explicit trusted guidance requires this coding session to stop.
    StopRequested,
}

impl CodingGuidanceError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SourceDenied => "runtime.coding-guidance.source-denied",
            Self::ConstraintDenied => "runtime.coding-guidance.constraint-denied",
            Self::StopRequested => "runtime.coding-guidance.stop-requested",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields)]
struct CodingGuidanceTool {
    tool_id: ToolId,
    tool_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields)]
struct CodingGuidanceApplication {
    decision_id: String,
    scope: WorkspaceScopePath,
    kind: GuidanceConstraintKind,
    target: String,
}

/// Immutable authority-reducing policy derived from explicit user trust decisions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodingGuidancePolicy {
    schema_version: u16,
    ledger_sha256: String,
    effective_guidance_sha256: String,
    applications: Vec<CodingGuidanceApplication>,
    excluded_scopes: Vec<WorkspaceScopePath>,
    disabled_tools: Vec<CodingGuidanceTool>,
    required_validation_ids: Vec<String>,
    confirmation_tools: Vec<CodingGuidanceTool>,
    limits: RuntimeRunLimits,
    policy_sha256: String,
}

impl CodingGuidancePolicy {
    /// Verifies the complete closed narrowing policy and its canonical digest.
    #[must_use]
    pub fn verify(&self) -> bool {
        self.schema_version == 1
            && is_sha256(&self.ledger_sha256)
            && is_sha256(&self.effective_guidance_sha256)
            && self.applications.windows(2).all(|pair| pair[0] < pair[1])
            && self.applications.iter().all(|application| {
                valid_identifier(&application.decision_id)
                    && !application.target.is_empty()
                    && application.target.len() <= 512
                    && !application.target.chars().any(char::is_control)
            })
            && self
                .excluded_scopes
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            && self.disabled_tools.windows(2).all(|pair| pair[0] < pair[1])
            && self.disabled_tools.iter().all(valid_tool)
            && self
                .required_validation_ids
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            && self
                .required_validation_ids
                .iter()
                .all(|identity| valid_identifier(identity))
            && self
                .confirmation_tools
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            && self.confirmation_tools.iter().all(valid_tool)
            && validate_runtime_run_limits(&self.limits).is_ok()
            && is_sha256(&self.policy_sha256)
            && policy_digest(self).is_ok_and(|digest| digest == self.policy_sha256)
    }

    /// Removes every explicitly disabled registration from one newly composed native registry.
    pub fn restrict_registry(&self, registry: &mut ToolRegistry) {
        registry.retain_tools(|definition| !self.disables(definition));
    }

    /// Returns canonical scopes that every operation policy must deny.
    #[must_use]
    pub fn excluded_scopes(&self) -> &[WorkspaceScopePath] {
        &self.excluded_scopes
    }

    /// Returns exact validation identities required before successful completion.
    #[must_use]
    pub fn required_validation_ids(&self) -> &[String] {
        &self.required_validation_ids
    }

    /// Returns the effective run ceilings after every trusted reduction.
    #[must_use]
    pub const fn limits(&self) -> &RuntimeRunLimits {
        &self.limits
    }

    /// Returns the canonical digest of all trusted guidance effects.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    /// Reports whether trusted guidance requires an additional confirmation for one exact tool.
    #[must_use]
    pub fn requires_confirmation(&self, tool_id: &ToolId, tool_version: &str) -> bool {
        self.confirmation_tools
            .binary_search(&CodingGuidanceTool {
                tool_id: tool_id.clone(),
                tool_version: tool_version.to_owned(),
            })
            .is_ok()
    }

    fn disables(&self, definition: &ToolDefinition) -> bool {
        self.disabled_tools
            .binary_search(&CodingGuidanceTool {
                tool_id: definition.tool_id.clone(),
                tool_version: definition.tool_version.clone(),
            })
            .is_ok()
    }
}

/// Compiles explicit trusted decisions into only deterministic authority-reducing effects.
pub fn compile_coding_guidance(
    workspace_id: &WorkspaceId,
    ledger: &InstructionEvidenceLedger,
    guidance: &EffectiveGuidance,
    registry: &ToolRegistry,
    validations: &ValidationTemplateRegistry,
    limits: &RuntimeRunLimits,
) -> Result<CodingGuidancePolicy, CodingGuidanceError> {
    if effective_guidance(ledger).as_ref() != Ok(guidance)
        || validate_runtime_run_limits(limits).is_err()
    {
        return Err(CodingGuidanceError::SourceDenied);
    }
    let applied_ids = guidance
        .applied_decision_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if applied_ids.len() != guidance.applied_decision_ids.len() {
        return Err(CodingGuidanceError::SourceDenied);
    }

    let mut applications = Vec::new();
    let mut excluded_scopes = Vec::new();
    let mut disabled_tools = Vec::new();
    let mut required_validation_ids = Vec::new();
    let mut confirmation_tools = Vec::new();
    let mut effective_limits = limits.clone();
    for decision in ledger
        .decisions
        .iter()
        .filter(|decision| applied_ids.contains(decision.decision_id.as_str()))
    {
        if decision.disposition != InstructionTrustDisposition::TrustForNarrowing
            || &decision.scope.workspace_id != workspace_id
            || decision.scope.path.workspace_id() != workspace_id
        {
            return Err(CodingGuidanceError::SourceDenied);
        }
        for constraint in &decision.constraints {
            applications.push(CodingGuidanceApplication {
                decision_id: decision.decision_id.clone(),
                scope: decision.scope.path.clone(),
                kind: constraint.kind,
                target: constraint.target.clone(),
            });
            match constraint.kind {
                GuidanceConstraintKind::ExcludeScope => {
                    excluded_scopes.push(decision.scope.path.clone());
                }
                GuidanceConstraintKind::DisableTool => {
                    disabled_tools.push(resolve_tool(registry, &constraint.target)?);
                }
                GuidanceConstraintKind::RequireVerification => {
                    if !valid_identifier(&constraint.target)
                        || !validations
                            .templates
                            .iter()
                            .any(|template| template.validation_id == constraint.target)
                    {
                        return Err(CodingGuidanceError::ConstraintDenied);
                    }
                    required_validation_ids.push(constraint.target.clone());
                }
                GuidanceConstraintKind::RequireUserConfirmation => {
                    confirmation_tools.push(resolve_tool(registry, &constraint.target)?);
                }
                GuidanceConstraintKind::ReduceBudget => {
                    apply_budget_cap(&mut effective_limits, &constraint.target)?;
                }
                GuidanceConstraintKind::Stop => {
                    return Err(CodingGuidanceError::StopRequested);
                }
            }
        }
    }
    if applications.len()
        != ledger
            .decisions
            .iter()
            .filter(|decision| applied_ids.contains(decision.decision_id.as_str()))
            .map(|decision| decision.constraints.len())
            .sum::<usize>()
        || applied_ids.len()
            != ledger
                .decisions
                .iter()
                .filter(|decision| applied_ids.contains(decision.decision_id.as_str()))
                .count()
        || validate_runtime_run_limits(&effective_limits).is_err()
    {
        return Err(CodingGuidanceError::SourceDenied);
    }

    applications.sort();
    applications.dedup();
    excluded_scopes.sort();
    excluded_scopes.dedup();
    disabled_tools.sort();
    disabled_tools.dedup();
    required_validation_ids.sort();
    required_validation_ids.dedup();
    confirmation_tools.sort();
    confirmation_tools.dedup();
    let mut policy = CodingGuidancePolicy {
        schema_version: 1,
        ledger_sha256: ledger.ledger_sha256.clone(),
        effective_guidance_sha256: guidance.guidance_sha256.clone(),
        applications,
        excluded_scopes,
        disabled_tools,
        required_validation_ids,
        confirmation_tools,
        limits: effective_limits,
        policy_sha256: String::new(),
    };
    policy.policy_sha256 = policy_digest(&policy)?;
    policy
        .verify()
        .then_some(policy)
        .ok_or(CodingGuidanceError::ConstraintDenied)
}

fn resolve_tool(
    registry: &ToolRegistry,
    target: &str,
) -> Result<CodingGuidanceTool, CodingGuidanceError> {
    let (tool_id, tool_version) = target
        .split_once('@')
        .filter(|(tool_id, tool_version)| {
            valid_identifier(tool_id) && valid_version(tool_version) && !tool_version.contains('@')
        })
        .ok_or(CodingGuidanceError::ConstraintDenied)?;
    let tool_id = ToolId::from_raw(tool_id.to_owned());
    registry
        .get_tool(&tool_id, tool_version)
        .ok_or(CodingGuidanceError::ConstraintDenied)?;
    Ok(CodingGuidanceTool {
        tool_id,
        tool_version: tool_version.to_owned(),
    })
}

fn apply_budget_cap(
    limits: &mut RuntimeRunLimits,
    target: &str,
) -> Result<(), CodingGuidanceError> {
    let (resource, cap) = target
        .split_once('=')
        .filter(|(resource, cap)| !resource.is_empty() && !cap.is_empty() && !cap.contains('='))
        .ok_or(CodingGuidanceError::ConstraintDenied)?;
    let cap = cap
        .parse::<u64>()
        .ok()
        .filter(|cap| *cap > 0)
        .ok_or(CodingGuidanceError::ConstraintDenied)?;
    macro_rules! cap_u32 {
        ($field:ident) => {{
            let cap = u32::try_from(cap).map_err(|_| CodingGuidanceError::ConstraintDenied)?;
            if cap > limits.$field {
                return Err(CodingGuidanceError::ConstraintDenied);
            }
            limits.$field = limits.$field.min(cap);
        }};
    }
    macro_rules! cap_u8 {
        ($field:ident) => {{
            let cap = u8::try_from(cap).map_err(|_| CodingGuidanceError::ConstraintDenied)?;
            if cap > limits.$field {
                return Err(CodingGuidanceError::ConstraintDenied);
            }
            limits.$field = limits.$field.min(cap);
        }};
    }
    macro_rules! cap_u64 {
        ($field:ident) => {{
            if cap > limits.$field {
                return Err(CodingGuidanceError::ConstraintDenied);
            }
            limits.$field = limits.$field.min(cap);
        }};
    }
    match resource {
        "max_turns" => cap_u32!(max_turns),
        "max_model_calls" => cap_u32!(max_model_calls),
        "max_tool_calls" => cap_u32!(max_tool_calls),
        "max_repeated_tool_calls" => cap_u8!(max_repeated_tool_calls),
        "max_tool_call_depth" => cap_u8!(max_tool_call_depth),
        "max_no_progress_turns" => cap_u32!(max_no_progress_turns),
        "max_context_refreshes" => cap_u32!(max_context_refreshes),
        "max_events" => cap_u32!(max_events),
        "max_elapsed_ms" => cap_u64!(max_elapsed_ms),
        "max_output_bytes" => cap_u64!(max_output_bytes),
        _ => return Err(CodingGuidanceError::ConstraintDenied),
    }
    Ok(())
}

fn policy_digest(policy: &CodingGuidancePolicy) -> Result<String, CodingGuidanceError> {
    #[derive(Serialize)]
    struct Material<'a> {
        schema_version: u16,
        ledger_sha256: &'a str,
        effective_guidance_sha256: &'a str,
        applications: &'a [CodingGuidanceApplication],
        excluded_scopes: &'a [WorkspaceScopePath],
        disabled_tools: &'a [CodingGuidanceTool],
        required_validation_ids: &'a [String],
        confirmation_tools: &'a [CodingGuidanceTool],
        limits: &'a RuntimeRunLimits,
    }

    serde_json::to_vec(&Material {
        schema_version: policy.schema_version,
        ledger_sha256: &policy.ledger_sha256,
        effective_guidance_sha256: &policy.effective_guidance_sha256,
        applications: &policy.applications,
        excluded_scopes: &policy.excluded_scopes,
        disabled_tools: &policy.disabled_tools,
        required_validation_ids: &policy.required_validation_ids,
        confirmation_tools: &policy.confirmation_tools,
        limits: &policy.limits,
    })
    .map(|bytes| sha256(&bytes))
    .map_err(|_| CodingGuidanceError::ConstraintDenied)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'.' | b'_' | b':' | b'-'))
        })
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
}

fn valid_tool(tool: &CodingGuidanceTool) -> bool {
    valid_identifier(tool.tool_id.as_str()) && valid_version(&tool.tool_version)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::BTreeMap;

    use agentmage_kernel_contracts::{ToolId, WorkspacePath, WorkspaceScopePath};
    use agentmage_kernel_engine::instruction_provenance::{
        GuidanceConstraint, InstructionDiscoveryInput, InstructionLocation, InstructionScope,
        InstructionSourceKind, InstructionTrustDisposition, build_instruction_ledger,
        record_instruction_discovery, record_instruction_read, record_instruction_trust_decision,
    };

    use super::*;
    use crate::{
        coding_changes::{CONTROLLED_CHANGE_TOOL_VERSION, STRUCTURED_PATCH_TOOL_ID},
        coding_session::{
            CodingSessionProfile, CodingSessionProfileError, CodingSessionProfileInput,
            tests::input,
        },
    };

    pub(crate) fn profile_input_with(
        constraints: Vec<GuidanceConstraint>,
    ) -> CodingSessionProfileInput {
        let mut profile_input = input();
        let workspace_id = profile_input.write_scope.workspace_id().clone();
        let freshness_sha256 = "b".repeat(64);
        let discovery = record_instruction_discovery(InstructionDiscoveryInput {
            discovery_id: "discovery-coding-guidance".to_owned(),
            source_kind: InstructionSourceKind::RepositoryInstruction,
            location: InstructionLocation {
                workspace_path: Some(
                    WorkspacePath::new(workspace_id.clone(), ["AGENTS.md"])
                        .expect("instruction path"),
                ),
                source_identity_sha256: "c".repeat(64),
                revision_sha256: "d".repeat(64),
                line_start: Some(1),
                line_end: Some(8),
            },
            workspace_manifest_sha256: profile_input.worktree.record_sha256.clone(),
            freshness_sha256: freshness_sha256.clone(),
        })
        .expect("instruction discovery");
        let read = record_instruction_read(
            &discovery,
            &freshness_sha256,
            b"Trusted narrowing fixture only.",
        )
        .expect("instruction read");
        let decision = record_instruction_trust_decision(
            "decision-coding-guidance".to_owned(),
            &read,
            InstructionScope {
                workspace_id: workspace_id.clone(),
                path: WorkspaceScopePath::new(workspace_id, ["src", "generated"])
                    .expect("instruction scope"),
            },
            100,
            InstructionTrustDisposition::TrustForNarrowing,
            constraints,
            "e".repeat(64),
        )
        .expect("trust decision");
        profile_input.instruction_ledger = build_instruction_ledger(
            profile_input.worktree.record_sha256.clone(),
            vec![discovery],
            vec![read.clone()],
            vec![decision],
            &BTreeMap::from([(read.record_sha256, freshness_sha256)]),
        )
        .expect("instruction ledger");
        profile_input
    }

    fn constraint(kind: GuidanceConstraintKind, target: &str) -> GuidanceConstraint {
        GuidanceConstraint {
            kind,
            target: target.to_owned(),
        }
    }

    #[test]
    fn trusted_guidance_only_removes_authority_and_strengthens_completion() {
        let profile = CodingSessionProfile::build(profile_input_with(vec![
            constraint(GuidanceConstraintKind::ExcludeScope, "generated-scope"),
            constraint(
                GuidanceConstraintKind::DisableTool,
                "agentmage.workspace.search-text@1.0.0",
            ),
            constraint(
                GuidanceConstraintKind::RequireVerification,
                "validation-unit",
            ),
            constraint(
                GuidanceConstraintKind::RequireUserConfirmation,
                "agentmage.code.patch-file@1.0.0",
            ),
            constraint(GuidanceConstraintKind::ReduceBudget, "max_tool_calls=4"),
        ]))
        .expect("narrowed profile");

        assert_eq!(profile.visible_tools().len(), 21);
        assert!(
            profile
                .registry()
                .get_tool(
                    &ToolId::from_raw("agentmage.workspace.search-text"),
                    "1.0.0"
                )
                .is_none()
        );
        assert_eq!(profile.limits().max_tool_calls, 4);
        assert_eq!(
            profile.coding_guidance().required_validation_ids(),
            &["validation-unit".to_owned()]
        );
        assert_eq!(profile.coding_guidance().excluded_scopes().len(), 1);
        assert!(profile.coding_guidance().requires_confirmation(
            &ToolId::from_raw(STRUCTURED_PATCH_TOOL_ID),
            CONTROLLED_CHANGE_TOOL_VERSION,
        ));
        assert!(profile.coding_guidance().verify());
    }

    #[test]
    fn stop_guidance_prevents_profile_construction() {
        let result = CodingSessionProfile::build(profile_input_with(vec![constraint(
            GuidanceConstraintKind::Stop,
            "user-requested-stop",
        )]));
        assert_eq!(
            result.expect_err("stop must deny construction"),
            CodingSessionProfileError::InstructionDenied
        );
    }

    #[test]
    fn budget_guidance_cannot_widen_the_profile() {
        let result = CodingSessionProfile::build(profile_input_with(vec![constraint(
            GuidanceConstraintKind::ReduceBudget,
            "max_tool_calls=33",
        )]));
        assert_eq!(
            result.expect_err("widening cap must be denied"),
            CodingSessionProfileError::InstructionDenied
        );
    }
}
