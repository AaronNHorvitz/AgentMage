//! Explicit exact-profile selection, deterministic-first dispatch, and resource control.

use std::fmt::Write;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ModelProfileId, ModelResourceReport, ModelRole, ModelRuntimeIdentity,
    TaskId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::model_runtime::{
    AdmittedModelProfile, ModelAdmissionCatalog, ModelRuntimeGateError, ModelUsePurpose,
};

const MAX_CODE_BYTES: usize = 128;
const MAX_DETERMINISTIC_OPERATIONS: usize = 64;

/// Terminal outcome of one explicit selection attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelSelectionOutcome {
    /// The exact user-selected profile became active.
    Selected,
    /// The named exact profile was refused without changing the current selection.
    Refused,
}

/// Content-free receipt for one explicit model selection or refusal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelSelectionReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Monotonic sequence within this selector.
    pub sequence: u64,
    /// Exact profile requested by the user.
    pub requested_profile_id: ModelProfileId,
    /// Declared role for this selection.
    pub requested_role: ModelRole,
    /// Terminal selection outcome.
    pub outcome: ModelSelectionOutcome,
    /// Stable content-free result code.
    pub result_code: String,
    /// Exact selected profile after the attempt, when one remains selected.
    pub active_profile_id: Option<ModelProfileId>,
    /// Digest of the receipt fields before this field.
    pub receipt_sha256: String,
}

/// Visible exact model boundary for one local session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SessionModelIdentity {
    /// Exact selected profile.
    pub profile_id: ModelProfileId,
    /// Exact manifest digest.
    pub manifest_sha256: String,
    /// Exact runtime identity.
    pub runtime: ModelRuntimeIdentity,
    /// Exact context-token ceiling.
    pub max_context_tokens: u32,
    /// Exact message ceiling.
    pub max_messages: u32,
    /// Exact output-token ceiling.
    pub max_output_tokens: u32,
    /// Exact tool protocol visible to the model codec.
    pub tool_protocol_version: String,
    /// Whether image input is admitted by this exact profile.
    pub image_input: bool,
}

/// Kernel owner of one explicit selected product profile.
pub struct ManualModelSelector {
    catalog: ModelAdmissionCatalog,
    selected: Option<AdmittedModelProfile>,
    sequence: u64,
}

impl ManualModelSelector {
    /// Creates an empty selector. No profile is selected implicitly.
    #[must_use]
    pub const fn new(catalog: ModelAdmissionCatalog) -> Self {
        Self {
            catalog,
            selected: None,
            sequence: 0,
        }
    }

    /// Selects exactly one user-named product profile for one passed role.
    pub fn select(&mut self, profile_id: ModelProfileId, role: ModelRole) -> ModelSelectionReceipt {
        let result = self
            .catalog
            .admit_by_id(&profile_id, ModelUsePurpose::Product);
        let (outcome, code) = match result {
            Ok(admitted)
                if admitted
                    .exact_profile()
                    .capabilities
                    .iter()
                    .any(|capability| {
                        capability.role == role
                            && capability.state
                                == agentmage_kernel_contracts::ModelCapabilityState::Passed
                    }) =>
            {
                self.selected = Some(admitted);
                (ModelSelectionOutcome::Selected, "model.selection.selected")
            }
            Ok(_) => (
                ModelSelectionOutcome::Refused,
                "model.selection.role-unavailable",
            ),
            Err(error) => (ModelSelectionOutcome::Refused, selection_error_code(error)),
        };
        self.sequence = self.sequence.saturating_add(1);
        self.receipt(profile_id, role, outcome, code)
    }

    /// Refuses every non-user-directed selection without changing current state.
    pub fn refuse_automatic_selection(
        &mut self,
        profile_id: ModelProfileId,
        role: ModelRole,
    ) -> ModelSelectionReceipt {
        self.sequence = self.sequence.saturating_add(1);
        self.receipt(
            profile_id,
            role,
            ModelSelectionOutcome::Refused,
            "model.selection.automatic-prohibited",
        )
    }

    /// Returns the exact visible session boundary, if the user selected one.
    #[must_use]
    pub fn session_identity(&self) -> Option<SessionModelIdentity> {
        let profile = self.selected.as_ref()?.exact_profile();
        Some(SessionModelIdentity {
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            runtime: profile.runtime.clone(),
            max_context_tokens: profile.context.max_context_tokens,
            max_messages: profile.context.max_messages,
            max_output_tokens: profile.decoding.max_output_tokens,
            tool_protocol_version: profile.codec.tool_protocol_version.clone(),
            image_input: profile
                .modalities
                .contains(&agentmage_kernel_contracts::ModelModality::Image),
        })
    }

    /// Returns the exact selected admitted profile for runtime composition.
    #[must_use]
    pub const fn selected(&self) -> Option<&AdmittedModelProfile> {
        self.selected.as_ref()
    }

    fn receipt(
        &self,
        requested_profile_id: ModelProfileId,
        requested_role: ModelRole,
        outcome: ModelSelectionOutcome,
        result_code: &str,
    ) -> ModelSelectionReceipt {
        let unsigned = UnsignedSelectionReceipt {
            schema_version: CONTRACT_SCHEMA_VERSION,
            sequence: self.sequence,
            requested_profile_id: &requested_profile_id,
            requested_role,
            outcome,
            result_code,
            active_profile_id: self.selected.as_ref().map(AdmittedModelProfile::profile_id),
        };
        let bytes = serde_json::to_vec(&unsigned).expect("selection receipt is serializable");
        ModelSelectionReceipt {
            schema_version: CONTRACT_SCHEMA_VERSION,
            sequence: self.sequence,
            requested_profile_id,
            requested_role,
            outcome,
            result_code: result_code.to_owned(),
            active_profile_id: self
                .selected
                .as_ref()
                .map(|profile| profile.profile_id().clone()),
            receipt_sha256: sha256_hex(&bytes),
        }
    }
}

#[derive(Serialize)]
struct UnsignedSelectionReceipt<'a> {
    schema_version: u16,
    sequence: u64,
    requested_profile_id: &'a ModelProfileId,
    requested_role: ModelRole,
    outcome: ModelSelectionOutcome,
    result_code: &'a str,
    active_profile_id: Option<&'a ModelProfileId>,
}

/// One deterministic operation considered before model inference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DeterministicOperationRecord {
    /// Stable operation code.
    pub operation_code: String,
    /// Whether this operation applies to the exact task.
    pub applicable: bool,
    /// Whether it was attempted.
    pub attempted: bool,
    /// Whether it resolved the task without inference.
    pub satisfied: bool,
}

/// Deterministic-first dispatch decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchDecision {
    /// Deterministic work satisfied the task and inference is unnecessary.
    DeterministicComplete,
    /// Every applicable deterministic operation ran and inference may be proposed.
    ModelEligible,
    /// An applicable deterministic operation was not attempted.
    Blocked,
}

/// Content-free record of deterministic-first task dispatch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DispatchReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Stable deterministic task class.
    pub task_class_code: String,
    /// Ordered deterministic operations.
    pub deterministic_operations: Vec<DeterministicOperationRecord>,
    /// Exact selected model, if inference is eligible.
    pub selected_profile_id: Option<ModelProfileId>,
    /// Dispatch decision.
    pub decision: DispatchDecision,
    /// Stable validation outcome.
    pub validation_code: String,
    /// Content-free logical dispatch latency.
    pub latency_ms: u64,
    /// Whether the deterministic acceptance check was satisfied.
    pub acceptance_satisfied: bool,
    /// Digest of the receipt fields before this field.
    pub receipt_sha256: String,
}

/// Computes one deterministic-first decision without invoking or switching a model.
pub fn dispatch_deterministic_first(
    task_id: TaskId,
    task_class_code: String,
    operations: Vec<DeterministicOperationRecord>,
    selected_profile_id: Option<ModelProfileId>,
    latency_ms: u64,
) -> Result<DispatchReceipt, &'static str> {
    if !valid_code(&task_class_code)
        || operations.len() > MAX_DETERMINISTIC_OPERATIONS
        || operations.iter().any(|operation| {
            !valid_code(&operation.operation_code)
                || (!operation.attempted && operation.satisfied)
                || (!operation.applicable && operation.satisfied)
        })
    {
        return Err("model.dispatch.input-invalid");
    }
    let blocked = operations
        .iter()
        .any(|operation| operation.applicable && !operation.attempted);
    let acceptance_satisfied = operations
        .iter()
        .any(|operation| operation.applicable && operation.satisfied);
    let (decision, validation_code) = if blocked {
        (
            DispatchDecision::Blocked,
            "model.dispatch.deterministic-required",
        )
    } else if acceptance_satisfied {
        (
            DispatchDecision::DeterministicComplete,
            "model.dispatch.deterministic-complete",
        )
    } else if selected_profile_id.is_some() {
        (
            DispatchDecision::ModelEligible,
            "model.dispatch.model-eligible",
        )
    } else {
        (
            DispatchDecision::Blocked,
            "model.dispatch.selection-required",
        )
    };
    let mut receipt = DispatchReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        task_id,
        task_class_code,
        deterministic_operations: operations,
        selected_profile_id,
        decision,
        validation_code: validation_code.to_owned(),
        latency_ms,
        acceptance_satisfied,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = receipt_digest(&receipt);
    Ok(receipt)
}

/// Exact ceilings for one inference slot and its worker process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelResourceLimits {
    /// Resident-memory ceiling.
    pub resident_memory_bytes: u64,
    /// Accelerator-memory ceiling.
    pub accelerator_memory_bytes: u64,
    /// CPU-time ceiling.
    pub cpu_time_ms: u64,
    /// GPU-time ceiling.
    pub gpu_time_ms: u64,
    /// Context-token ceiling.
    pub context_tokens: u32,
    /// Output-token ceiling.
    pub output_tokens: u32,
    /// Concurrent inference-slot ceiling; v0.1 requires exactly one.
    pub inference_slots: u16,
    /// Scratch-disk ceiling.
    pub disk_bytes: u64,
    /// Descendant-process ceiling.
    pub process_count: u32,
}

/// One content-free observation spanning runtime and operating-system collectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelResourceObservation {
    /// Resident memory.
    pub resident_memory_bytes: u64,
    /// Accelerator memory.
    pub accelerator_memory_bytes: u64,
    /// CPU time.
    pub cpu_time_ms: u64,
    /// GPU time.
    pub gpu_time_ms: u64,
    /// Input context tokens.
    pub context_tokens: u32,
    /// Output tokens.
    pub output_tokens: u32,
    /// Active inference slots.
    pub inference_slots: u16,
    /// Scratch disk bytes.
    pub disk_bytes: u64,
    /// Worker plus descendant process count.
    pub process_count: u32,
}

impl ModelResourceObservation {
    /// Combines one validated runtime report with separately collected OS-only counters.
    #[must_use]
    pub const fn from_runtime(
        report: &ModelResourceReport,
        cpu_time_ms: u64,
        gpu_time_ms: u64,
        inference_slots: u16,
        disk_bytes: u64,
        process_count: u32,
    ) -> Self {
        Self {
            resident_memory_bytes: report.resident_memory_bytes,
            accelerator_memory_bytes: report.accelerator_memory_bytes,
            cpu_time_ms,
            gpu_time_ms,
            context_tokens: report.input_tokens,
            output_tokens: report.output_tokens,
            inference_slots,
            disk_bytes,
            process_count,
        }
    }
}

/// Stable resource whose exact ceiling was exceeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelResourceKind {
    /// Resident memory.
    ResidentMemory,
    /// Accelerator memory.
    AcceleratorMemory,
    /// CPU time.
    CpuTime,
    /// GPU time.
    GpuTime,
    /// Context tokens.
    Context,
    /// Output tokens.
    Output,
    /// Concurrent inference slots.
    Concurrency,
    /// Scratch disk.
    Disk,
    /// Worker process tree.
    Process,
}

/// Result of one resource observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceDecision {
    /// The affected operation may continue.
    Continue,
    /// The affected operation must stop and clean up.
    Stop {
        /// First exact ceiling exceeded by the operation.
        resource: ModelResourceKind,
    },
}

/// Sticky fail-closed resource governor for one active model operation.
pub struct ModelResourceGovernor {
    limits: ModelResourceLimits,
    stopped: Option<ModelResourceKind>,
}

impl ModelResourceGovernor {
    /// Creates a v0.1 governor only for nonzero limits and one inference slot.
    pub fn new(limits: ModelResourceLimits) -> Result<Self, &'static str> {
        if limits.resident_memory_bytes == 0
            || limits.accelerator_memory_bytes == 0
            || limits.cpu_time_ms == 0
            || limits.gpu_time_ms == 0
            || limits.context_tokens == 0
            || limits.output_tokens == 0
            || limits.inference_slots != 1
            || limits.disk_bytes == 0
            || limits.process_count == 0
        {
            return Err("model.resource.limits-invalid");
        }
        Ok(Self {
            limits,
            stopped: None,
        })
    }

    /// Evaluates all ceilings in fixed order and retains the first terminal resource.
    pub fn observe(&mut self, observation: ModelResourceObservation) -> ResourceDecision {
        if let Some(resource) = self.stopped {
            return ResourceDecision::Stop { resource };
        }
        let exceeded = [
            (
                observation.resident_memory_bytes > self.limits.resident_memory_bytes,
                ModelResourceKind::ResidentMemory,
            ),
            (
                observation.accelerator_memory_bytes > self.limits.accelerator_memory_bytes,
                ModelResourceKind::AcceleratorMemory,
            ),
            (
                observation.cpu_time_ms > self.limits.cpu_time_ms,
                ModelResourceKind::CpuTime,
            ),
            (
                observation.gpu_time_ms > self.limits.gpu_time_ms,
                ModelResourceKind::GpuTime,
            ),
            (
                observation.context_tokens > self.limits.context_tokens,
                ModelResourceKind::Context,
            ),
            (
                observation.output_tokens > self.limits.output_tokens,
                ModelResourceKind::Output,
            ),
            (
                observation.inference_slots > self.limits.inference_slots,
                ModelResourceKind::Concurrency,
            ),
            (
                observation.disk_bytes > self.limits.disk_bytes,
                ModelResourceKind::Disk,
            ),
            (
                observation.process_count > self.limits.process_count,
                ModelResourceKind::Process,
            ),
        ]
        .into_iter()
        .find_map(|(exceeded, resource)| exceeded.then_some(resource));
        if let Some(resource) = exceeded {
            self.stopped = Some(resource);
            ResourceDecision::Stop { resource }
        } else {
            ResourceDecision::Continue
        }
    }
}

fn selection_error_code(error: ModelRuntimeGateError) -> &'static str {
    match error {
        ModelRuntimeGateError::ProfileNotRegistered => "model.selection.unavailable",
        ModelRuntimeGateError::ProfileChanged => "model.selection.profile-changed",
        ModelRuntimeGateError::ProfileUnavailable => "model.selection.not-admitted",
        ModelRuntimeGateError::AutomaticFallbackProhibited => {
            "model.selection.automatic-prohibited"
        }
        _ => "model.selection.invalid",
    }
}

fn receipt_digest(receipt: &DispatchReceipt) -> String {
    let bytes = serde_json::to_vec(&(
        receipt.schema_version,
        &receipt.task_id,
        &receipt.task_class_code,
        &receipt.deterministic_operations,
        &receipt.selected_profile_id,
        receipt.decision,
        &receipt.validation_code,
        receipt.latency_ms,
        receipt.acceptance_satisfied,
    ))
    .expect("dispatch receipt is serializable");
    sha256_hex(&bytes)
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CODE_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn sha256_hex(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ContextBudget, DecodingProfile, ExactModelProfile,
        FamilyCodecIdentity, HardwareEnvelope, ModelAdapterId, ModelArtifact, ModelCapability,
        ModelCapabilityState, ModelCodecId, ModelLifecycleState, ModelManifestId, ModelModality,
        ModelProfileId, ModelRole, ModelRuntimeIdentity, ModelRuntimeKind, PlatformArchitecture,
        PlatformFamily, TaskId,
    };

    use super::{
        DeterministicOperationRecord, DispatchDecision, ManualModelSelector, ModelResourceGovernor,
        ModelResourceKind, ModelResourceLimits, ModelResourceObservation, ModelSelectionOutcome,
        ResourceDecision, dispatch_deterministic_first,
    };
    use crate::model_runtime::ModelAdmissionCatalog;

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn profile(id: &str, lifecycle: ModelLifecycleState, enabled: bool) -> ExactModelProfile {
        ExactModelProfile {
            schema_version: CONTRACT_SCHEMA_VERSION,
            profile_id: ModelProfileId::from_raw(id),
            manifest_id: ModelManifestId::from_raw(format!("manifest-{id}")),
            manifest_sha256: SHA.to_owned(),
            display_name: id.to_owned(),
            family: "fixture".to_owned(),
            publisher_control: "fixture".to_owned(),
            lineage: vec!["fixture".to_owned()],
            license_spdx: "Apache-2.0".to_owned(),
            license_terms_sha256: SHA.to_owned(),
            artifact: ModelArtifact {
                artifact_id: format!("artifact-{id}"),
                publisher: "fixture".to_owned(),
                source_revision: "revision-1".to_owned(),
                format: "fixture".to_owned(),
                bytes: 1,
                sha256: SHA.to_owned(),
            },
            transformations: Vec::new(),
            codec: FamilyCodecIdentity {
                codec_id: ModelCodecId::from_raw(format!("codec-{id}")),
                codec_version: "1".to_owned(),
                codec_sha256: SHA.to_owned(),
                tokenizer: "fixture".to_owned(),
                tokenizer_sha256: SHA.to_owned(),
                template: "fixture".to_owned(),
                template_sha256: SHA.to_owned(),
                tool_protocol_version: "1".to_owned(),
                end_tokens: vec![1],
                reasoning_enabled: false,
            },
            runtime: ModelRuntimeIdentity {
                adapter_id: ModelAdapterId::from_raw(format!("adapter-{id}")),
                kind: ModelRuntimeKind::DeterministicFake,
                contract_version: 1,
                runtime_build: "fixture".to_owned(),
                runtime_sha256: SHA.to_owned(),
                platform: PlatformFamily::Fedora,
                architecture: PlatformArchitecture::X86_64,
            },
            quantization: "fixture".to_owned(),
            modalities: vec![ModelModality::Text],
            context: ContextBudget {
                max_context_tokens: 1024,
                max_input_bytes: 4096,
                max_messages: 16,
                token_counter: "fixture".to_owned(),
                token_counter_sha256: SHA.to_owned(),
            },
            decoding: DecodingProfile {
                profile_id: "fixture".to_owned(),
                sampler_order: vec!["temperature".to_owned()],
                temperature: 0.0,
                top_p: 1.0,
                top_k: 1,
                repeat_penalty: 1.0,
                seed: 1,
                max_output_tokens: 256,
            },
            hardware: vec![HardwareEnvelope {
                platform: PlatformFamily::Fedora,
                architecture: PlatformArchitecture::X86_64,
                minimum_system_memory_bytes: 1,
                minimum_accelerator_memory_bytes: 0,
                accelerator: "cpu".to_owned(),
                driver_constraint: "none".to_owned(),
            }],
            capabilities: vec![ModelCapability {
                role: ModelRole::Dialogue,
                state: ModelCapabilityState::Passed,
                evaluation_profile: Some("fixture-evaluation".to_owned()),
                result_sha256: Some(SHA.to_owned()),
                limitations: Vec::new(),
            }],
            policy_sha256: SHA.to_owned(),
            lifecycle,
            enabled,
            automatic_fallback: false,
        }
    }

    #[test]
    fn manual_selection_preserves_active_profile_across_every_refusal() {
        let approved = profile("approved", ModelLifecycleState::Approved, true);
        let blocked = profile("blocked", ModelLifecycleState::Candidate, false);
        let catalog = ModelAdmissionCatalog::new(vec![approved.clone(), blocked.clone()])
            .expect("valid catalog");
        let mut selector = ManualModelSelector::new(catalog);
        let selected = selector.select(approved.profile_id.clone(), ModelRole::Dialogue);
        assert_eq!(selected.outcome, ModelSelectionOutcome::Selected);
        assert_eq!(selected.sequence, 1);
        assert_eq!(
            selector.session_identity().expect("identity").profile_id,
            approved.profile_id
        );

        let attempts = [
            selector.select(blocked.profile_id, ModelRole::Dialogue),
            selector.select(ModelProfileId::from_raw("missing"), ModelRole::Dialogue),
            selector.select(
                ModelProfileId::from_raw("approved"),
                ModelRole::CodingPlanner,
            ),
            selector.refuse_automatic_selection(
                ModelProfileId::from_raw("automatic"),
                ModelRole::Dialogue,
            ),
        ];
        for receipt in attempts {
            assert_eq!(receipt.outcome, ModelSelectionOutcome::Refused);
            assert_eq!(
                receipt
                    .active_profile_id
                    .as_ref()
                    .map(ModelProfileId::as_str),
                Some("approved")
            );
            assert_eq!(receipt.receipt_sha256.len(), 64);
        }
        assert_eq!(
            selector
                .session_identity()
                .expect("identity")
                .profile_id
                .as_str(),
            "approved"
        );
    }

    #[test]
    fn deterministic_operations_always_precede_model_eligibility() {
        for index in 0..100 {
            let applicable = index % 2 == 0;
            let attempted = index % 4 != 0;
            let receipt = dispatch_deterministic_first(
                TaskId::from_raw(format!("task-{index}")),
                "task.fixture".to_owned(),
                vec![DeterministicOperationRecord {
                    operation_code: "operation.fixture".to_owned(),
                    applicable,
                    attempted,
                    satisfied: false,
                }],
                Some(ModelProfileId::from_raw("approved")),
                1,
            )
            .expect("valid receipt");
            if applicable && !attempted {
                assert_eq!(receipt.decision, DispatchDecision::Blocked);
            } else {
                assert_eq!(receipt.decision, DispatchDecision::ModelEligible);
            }
        }
    }

    #[test]
    fn deterministic_completion_avoids_model_and_invalid_records_fail() {
        let receipt = dispatch_deterministic_first(
            TaskId::from_raw("task-complete"),
            "task.fixture".to_owned(),
            vec![DeterministicOperationRecord {
                operation_code: "operation.fixture".to_owned(),
                applicable: true,
                attempted: true,
                satisfied: true,
            }],
            Some(ModelProfileId::from_raw("approved")),
            1,
        )
        .expect("valid receipt");
        assert_eq!(receipt.decision, DispatchDecision::DeterministicComplete);
        assert!(receipt.acceptance_satisfied);
        assert!(
            dispatch_deterministic_first(
                TaskId::from_raw("task-invalid"),
                "task.fixture".to_owned(),
                vec![DeterministicOperationRecord {
                    operation_code: "operation.fixture".to_owned(),
                    applicable: true,
                    attempted: false,
                    satisfied: true,
                }],
                None,
                0,
            )
            .is_err()
        );
    }

    fn limits() -> ModelResourceLimits {
        ModelResourceLimits {
            resident_memory_bytes: 10,
            accelerator_memory_bytes: 10,
            cpu_time_ms: 10,
            gpu_time_ms: 10,
            context_tokens: 10,
            output_tokens: 10,
            inference_slots: 1,
            disk_bytes: 10,
            process_count: 1,
        }
    }

    fn observation() -> ModelResourceObservation {
        ModelResourceObservation {
            resident_memory_bytes: 10,
            accelerator_memory_bytes: 10,
            cpu_time_ms: 10,
            gpu_time_ms: 10,
            context_tokens: 10,
            output_tokens: 10,
            inference_slots: 1,
            disk_bytes: 10,
            process_count: 1,
        }
    }

    type ResourceMutation = (ModelResourceKind, fn(&mut ModelResourceObservation));

    #[test]
    fn every_resource_limit_is_inclusive_independent_and_sticky() {
        let mutations: [ResourceMutation; 9] = [
            (ModelResourceKind::ResidentMemory, |value| {
                value.resident_memory_bytes = 11
            }),
            (ModelResourceKind::AcceleratorMemory, |value| {
                value.accelerator_memory_bytes = 11
            }),
            (ModelResourceKind::CpuTime, |value| value.cpu_time_ms = 11),
            (ModelResourceKind::GpuTime, |value| value.gpu_time_ms = 11),
            (ModelResourceKind::Context, |value| {
                value.context_tokens = 11
            }),
            (ModelResourceKind::Output, |value| value.output_tokens = 11),
            (ModelResourceKind::Concurrency, |value| {
                value.inference_slots = 2
            }),
            (ModelResourceKind::Disk, |value| value.disk_bytes = 11),
            (ModelResourceKind::Process, |value| value.process_count = 2),
        ];
        for (expected, mutate) in mutations {
            let mut governor = ModelResourceGovernor::new(limits()).expect("valid limits");
            assert_eq!(governor.observe(observation()), ResourceDecision::Continue);
            let mut exceeded = observation();
            mutate(&mut exceeded);
            assert_eq!(
                governor.observe(exceeded),
                ResourceDecision::Stop { resource: expected }
            );
            assert_eq!(
                governor.observe(observation()),
                ResourceDecision::Stop { resource: expected }
            );
        }
    }

    #[test]
    fn one_slot_default_and_all_nonzero_limits_are_mandatory() {
        let mut invalid = limits();
        invalid.inference_slots = 2;
        assert_eq!(
            ModelResourceGovernor::new(invalid).err(),
            Some("model.resource.limits-invalid")
        );
        let mut invalid = limits();
        invalid.disk_bytes = 0;
        assert_eq!(
            ModelResourceGovernor::new(invalid).err(),
            Some("model.resource.limits-invalid")
        );
    }
}
