//! Deterministic, non-authoritative evidence-state assignment.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    AuthorityClass, CONTRACT_SCHEMA_VERSION, DerivedClaimProvenance, DeterministicMethodIdentity,
    EvidenceKind, EvidenceReference, GrantOperation, InferenceRuntimeProvenance,
    InferredClaimProvenance, MaterialClaim, MaterialClaimEvidenceAssignment,
    MaterialClaimEvidenceState, ModelManifestObservation, ModelRunId, ObservedClaimProvenance,
    OperationOutcome, Receipt, TaskId, UnknownBlockedClaimProvenance, UnknownBlockedReason,
    to_canonical_json,
};
use sha2::{Digest, Sha256};

const MAX_ASSIGNMENTS: usize = 128;
const MAX_INPUTS: usize = 128;
const MAX_SOURCES: usize = 128;
const MAX_IDENTIFIER_BYTES: usize = 256;
const MAX_DETAIL_CODE_BYTES: usize = 256;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Typed fail-closed reason an evidence-state assignment was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceStateAssignmentError {
    /// The assignment, claim, identifier, or state-specific payload is malformed.
    InvalidAssignment,
    /// An Observed state lacks a valid authority-bound deterministic observation receipt.
    InvalidObservedProvenance,
    /// A Derived state lacks an approved method or exact Observed inputs.
    InvalidDerivedProvenance,
    /// An Inferred state lacks supporting citations or exact model/runtime identity.
    InvalidInferredProvenance,
    /// An Unknown/Blocked state lacks a closed reason and stable detail code.
    InvalidUnknownBlockedProvenance,
    /// The assignment identity has already reached a terminal state.
    AlreadyAssigned,
    /// A referenced assignment is absent or belongs to another task.
    MissingInput,
}

/// Bounded registry of reviewed deterministic derivation implementations.
pub struct DeterministicMethodRegistry {
    methods: BTreeMap<String, DeterministicMethodIdentity>,
}

impl DeterministicMethodRegistry {
    /// Creates an exact registry and rejects malformed or duplicate methods.
    pub fn new(
        methods: Vec<DeterministicMethodIdentity>,
    ) -> Result<Self, EvidenceStateAssignmentError> {
        if methods.is_empty() || methods.len() > MAX_INPUTS {
            return Err(EvidenceStateAssignmentError::InvalidDerivedProvenance);
        }
        let mut registered = BTreeMap::new();
        for method in methods {
            if !valid_identifier(&method.method_id)
                || !valid_identifier(&method.method_version)
                || !valid_sha256(&method.implementation_sha256)
                || registered
                    .insert(method.method_id.clone(), method)
                    .is_some()
            {
                return Err(EvidenceStateAssignmentError::InvalidDerivedProvenance);
            }
        }
        Ok(Self {
            methods: registered,
        })
    }

    fn admits(&self, method: &DeterministicMethodIdentity) -> bool {
        self.methods
            .get(&method.method_id)
            .is_some_and(|registered| registered == method)
    }
}

/// Task-local kernel validator that assigns each claim at most one evidence state.
pub struct EvidenceStateAssigner {
    task_id: TaskId,
    methods: DeterministicMethodRegistry,
    assignments: BTreeMap<String, MaterialClaimEvidenceAssignment>,
}

impl EvidenceStateAssigner {
    /// Starts an empty bounded assignment ledger for one exact task.
    pub fn new(
        task_id: TaskId,
        methods: DeterministicMethodRegistry,
    ) -> Result<Self, EvidenceStateAssignmentError> {
        if !valid_identifier(task_id.as_str()) {
            return Err(EvidenceStateAssignmentError::InvalidAssignment);
        }
        Ok(Self {
            task_id,
            methods,
            assignments: BTreeMap::new(),
        })
    }

    /// Assigns Observed only from a valid successful Observe receipt and its exact sources.
    pub fn assign_observed(
        &mut self,
        assignment_id: impl Into<String>,
        claim: MaterialClaim,
        receipt: Receipt,
        sources: Vec<EvidenceReference>,
    ) -> Result<&MaterialClaimEvidenceAssignment, EvidenceStateAssignmentError> {
        validate_observed(&claim, &receipt, &sources)?;
        self.insert(
            assignment_id.into(),
            claim,
            MaterialClaimEvidenceState::Observed(Box::new(ObservedClaimProvenance {
                receipt,
                sources,
            })),
        )
    }

    /// Assigns Derived only from a registered method and exact Observed inputs.
    pub fn assign_derived(
        &mut self,
        assignment_id: impl Into<String>,
        claim: MaterialClaim,
        method: DeterministicMethodIdentity,
        observed_input_assignment_ids: Vec<String>,
    ) -> Result<&MaterialClaimEvidenceAssignment, EvidenceStateAssignmentError> {
        if !self.methods.admits(&method)
            || observed_input_assignment_ids.is_empty()
            || observed_input_assignment_ids.len() > MAX_INPUTS
            || has_duplicates(observed_input_assignment_ids.iter().map(String::as_str))
        {
            return Err(EvidenceStateAssignmentError::InvalidDerivedProvenance);
        }
        for input_id in &observed_input_assignment_ids {
            let input = self
                .assignments
                .get(input_id)
                .ok_or(EvidenceStateAssignmentError::MissingInput)?;
            if input.claim.task_id != self.task_id
                || !matches!(
                    input.evidence_state,
                    MaterialClaimEvidenceState::Observed(_)
                )
            {
                return Err(EvidenceStateAssignmentError::InvalidDerivedProvenance);
            }
        }
        self.insert(
            assignment_id.into(),
            claim,
            MaterialClaimEvidenceState::Derived(Box::new(DerivedClaimProvenance {
                method,
                observed_input_assignment_ids,
            })),
        )
    }

    /// Assigns Inferred with supporting citations and one exact model/runtime manifest.
    pub fn assign_inferred(
        &mut self,
        assignment_id: impl Into<String>,
        claim: MaterialClaim,
        citations: Vec<EvidenceReference>,
        model_run_id: ModelRunId,
        manifest: ModelManifestObservation,
        response_sha256: String,
    ) -> Result<&MaterialClaimEvidenceAssignment, EvidenceStateAssignmentError> {
        validate_inferred(&citations, &model_run_id, &manifest, &response_sha256)?;
        self.insert(
            assignment_id.into(),
            claim,
            MaterialClaimEvidenceState::Inferred(Box::new(InferredClaimProvenance {
                citations,
                runtime: InferenceRuntimeProvenance {
                    model_run_id,
                    manifest,
                    response_sha256,
                },
            })),
        )
    }

    /// Assigns Unknown/Blocked with an exact closed reason and content-free detail code.
    pub fn assign_unknown_blocked(
        &mut self,
        assignment_id: impl Into<String>,
        claim: MaterialClaim,
        reason: UnknownBlockedReason,
        detail_code: String,
        evidence: Vec<EvidenceReference>,
    ) -> Result<&MaterialClaimEvidenceAssignment, EvidenceStateAssignmentError> {
        if !valid_detail_code(&detail_code) || !valid_sources(&evidence, true) {
            return Err(EvidenceStateAssignmentError::InvalidUnknownBlockedProvenance);
        }
        self.insert(
            assignment_id.into(),
            claim,
            MaterialClaimEvidenceState::UnknownBlocked(Box::new(UnknownBlockedClaimProvenance {
                reason,
                detail_code,
                evidence,
            })),
        )
    }

    /// Returns one terminal assignment when present.
    #[must_use]
    pub fn assignment(&self, assignment_id: &str) -> Option<&MaterialClaimEvidenceAssignment> {
        self.assignments.get(assignment_id)
    }

    fn insert(
        &mut self,
        assignment_id: String,
        claim: MaterialClaim,
        evidence_state: MaterialClaimEvidenceState,
    ) -> Result<&MaterialClaimEvidenceAssignment, EvidenceStateAssignmentError> {
        if !valid_identifier(&assignment_id)
            || !valid_claim(&claim)
            || claim.task_id != self.task_id
            || self.assignments.len() >= MAX_ASSIGNMENTS
        {
            return Err(EvidenceStateAssignmentError::InvalidAssignment);
        }
        if self.assignments.contains_key(&assignment_id) {
            return Err(EvidenceStateAssignmentError::AlreadyAssigned);
        }
        self.assignments.insert(
            assignment_id.clone(),
            MaterialClaimEvidenceAssignment {
                schema_version: CONTRACT_SCHEMA_VERSION,
                assignment_id: assignment_id.clone(),
                claim,
                evidence_state,
            },
        );
        self.assignments
            .get(&assignment_id)
            .ok_or(EvidenceStateAssignmentError::InvalidAssignment)
    }
}

fn validate_observed(
    claim: &MaterialClaim,
    receipt: &Receipt,
    sources: &[EvidenceReference],
) -> Result<(), EvidenceStateAssignmentError> {
    if receipt.schema_version != CONTRACT_SCHEMA_VERSION
        || receipt.task_id != claim.task_id
        || receipt.outcome != OperationOutcome::Succeeded
        || receipt.tool_call_id.is_none()
        || receipt.error.is_some()
        || !valid_receipt_identities(receipt)
        || receipt.operation.authority_class() != AuthorityClass::Observe
        || !matches!(
            receipt.operation.operation(),
            GrantOperation::WorkspaceRead | GrantOperation::DatabaseRead
        )
        || !valid_sha256(&receipt.operation_sha256)
        || !valid_sha256(&receipt.previous_receipt_sha256)
        || !valid_sha256(&receipt.receipt_sha256)
        || receipt.operation_sha256 != operation_digest(receipt)
        || receipt.receipt_sha256 != receipt_digest(receipt)
        || receipt.evidence != sources
        || !valid_sources(sources, false)
        || sources.iter().any(|source| {
            source.kind != EvidenceKind::Observation
                || source.object_id != claim.subject_id
                || source.observed_revision.as_deref() != Some(claim.expected_revision.as_str())
        })
    {
        return Err(EvidenceStateAssignmentError::InvalidObservedProvenance);
    }
    Ok(())
}

fn valid_receipt_identities(receipt: &Receipt) -> bool {
    receipt.sequence > 0
        && valid_identifier(receipt.receipt_id.as_str())
        && valid_identifier(receipt.correlation_id.as_str())
        && valid_identifier(receipt.authority_transaction_id.as_str())
        && valid_identifier(receipt.operation_attempt_id.as_str())
        && valid_identifier(receipt.approval_id.as_str())
        && valid_identifier(receipt.grant_id.as_str())
        && valid_identifier(receipt.session_id.as_str())
        && valid_identifier(receipt.task_id.as_str())
        && valid_identifier(receipt.action_id.as_str())
        && receipt
            .tool_call_id
            .as_ref()
            .is_some_and(|identity| valid_identifier(identity.as_str()))
        && !receipt.occurred_at.trim().is_empty()
        && receipt.occurred_at.len() <= 64
}

fn validate_inferred(
    citations: &[EvidenceReference],
    model_run_id: &ModelRunId,
    manifest: &ModelManifestObservation,
    response_sha256: &str,
) -> Result<(), EvidenceStateAssignmentError> {
    let runtime = &manifest.runtime;
    if !valid_sources(citations, false)
        || !valid_identifier(model_run_id.as_str())
        || !valid_identifier(manifest.profile_id.as_str())
        || !valid_sha256(&manifest.manifest_sha256)
        || !valid_sha256(&manifest.artifact_sha256)
        || !valid_sha256(&manifest.tokenizer_sha256)
        || !valid_sha256(&manifest.template_sha256)
        || !valid_sha256(&manifest.codec_sha256)
        || !valid_identifier(runtime.adapter_id.as_str())
        || runtime.contract_version == 0
        || !valid_identifier(&runtime.runtime_build)
        || !valid_sha256(&runtime.runtime_sha256)
        || !valid_sha256(response_sha256)
    {
        return Err(EvidenceStateAssignmentError::InvalidInferredProvenance);
    }
    Ok(())
}

fn valid_claim(claim: &MaterialClaim) -> bool {
    claim.schema_version == CONTRACT_SCHEMA_VERSION
        && valid_identifier(&claim.claim_id)
        && valid_identifier(claim.task_id.as_str())
        && !claim.statement.trim().is_empty()
        && claim.statement.len() <= 4_096
        && valid_identifier(&claim.subject_id)
        && valid_identifier(&claim.expected_revision)
}

fn valid_sources(sources: &[EvidenceReference], may_be_empty: bool) -> bool {
    if (!may_be_empty && sources.is_empty()) || sources.len() > MAX_SOURCES {
        return false;
    }
    let mut identities = BTreeSet::new();
    sources.iter().all(|source| {
        source.schema_version == CONTRACT_SCHEMA_VERSION
            && valid_identifier(source.evidence_id.as_str())
            && !source.source_id.trim().is_empty()
            && source.source_id.len() <= 4_096
            && !source.object_id.trim().is_empty()
            && source.object_id.len() <= 4_096
            && source
                .fragment
                .as_ref()
                .is_none_or(|value| !value.trim().is_empty() && value.len() <= 4_096)
            && valid_sha256(&source.content_sha256)
            && source
                .observed_revision
                .as_ref()
                .is_none_or(|value| !value.trim().is_empty() && value.len() <= 256)
            && identities.insert(source.evidence_id.as_str())
    })
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_detail_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DETAIL_CODE_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn has_duplicates<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let values = values.collect::<Vec<_>>();
    values.iter().copied().collect::<BTreeSet<_>>().len() != values.len()
}

fn operation_digest(receipt: &Receipt) -> String {
    serde_json::to_vec(&receipt.operation)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_default()
}

fn receipt_digest(receipt: &Receipt) -> String {
    let mut candidate = receipt.clone();
    candidate.receipt_sha256 = ZERO_SHA256.to_owned();
    to_canonical_json(&candidate)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_default()
}

fn sha256_hex(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{
        DeterministicMethodRegistry, EvidenceStateAssigner, EvidenceStateAssignmentError,
        receipt_digest, sha256_hex,
    };
    use agentmage_kernel_contracts::{
        ActionId, ApprovalId, AuthorityTransactionId, CONTRACT_SCHEMA_VERSION, CorrelationId,
        DeterministicMethodIdentity, EvidenceId, EvidenceKind, EvidenceReference, GrantId,
        GrantOperation, MaterialClaim, MaterialClaimEvidenceState, MaterialClaimEvidenceStateKind,
        MaterialClaimKind, ModelAdapterId, ModelManifestObservation, ModelProfileId, ModelRunId,
        ModelRuntimeIdentity, ModelRuntimeKind, OperationAttemptId, OperationBinding,
        OperationOutcome, PlatformArchitecture, PlatformFamily, Receipt, ReceiptId, SessionId,
        TaskId, ToolCallId, UnknownBlockedReason, from_json,
    };

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn method() -> DeterministicMethodIdentity {
        DeterministicMethodIdentity {
            method_id: "method.sum".to_owned(),
            method_version: "1.0.0".to_owned(),
            implementation_sha256: SHA.to_owned(),
        }
    }

    fn assigner() -> EvidenceStateAssigner {
        EvidenceStateAssigner::new(
            TaskId::from_raw("task-evidence"),
            DeterministicMethodRegistry::new(vec![method()]).unwrap(),
        )
        .unwrap()
    }

    fn claim(id: &str, subject: &str) -> MaterialClaim {
        MaterialClaim {
            schema_version: CONTRACT_SCHEMA_VERSION,
            claim_id: id.to_owned(),
            task_id: TaskId::from_raw("task-evidence"),
            kind: MaterialClaimKind::Read,
            statement: format!("Evidence statement {id}"),
            subject_id: subject.to_owned(),
            expected_revision: "revision-1".to_owned(),
            prerequisite_claim_ids: Vec::new(),
        }
    }

    fn source(id: &str, subject: &str) -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(id),
            kind: EvidenceKind::Observation,
            source_id: "workspace/src/lib.rs".to_owned(),
            object_id: subject.to_owned(),
            fragment: Some("bytes:0-8".to_owned()),
            content_sha256: SHA.to_owned(),
            observed_revision: Some("revision-1".to_owned()),
        }
    }

    fn receipt(source: EvidenceReference) -> Receipt {
        let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
        let mut receipt = Receipt {
            schema_version: CONTRACT_SCHEMA_VERSION,
            receipt_id: ReceiptId::from_raw("receipt-observed"),
            sequence: 1,
            correlation_id: CorrelationId::from_raw("correlation-observed"),
            authority_transaction_id: AuthorityTransactionId::from_raw("transaction-observed"),
            operation_attempt_id: OperationAttemptId::from_raw("attempt-observed"),
            approval_id: ApprovalId::from_raw("approval-observed"),
            grant_id: GrantId::from_raw("grant-observed"),
            session_id: SessionId::from_raw("session-evidence"),
            task_id: TaskId::from_raw("task-evidence"),
            action_id: ActionId::from_raw("action-observed"),
            tool_call_id: Some(ToolCallId::from_raw("tool-call-observed")),
            operation,
            outcome: OperationOutcome::Succeeded,
            operation_sha256: sha256_hex(&serde_json::to_vec(&operation).unwrap()),
            evidence: vec![source],
            error: None,
            previous_receipt_sha256: "0".repeat(64),
            receipt_sha256: "0".repeat(64),
            occurred_at: "1970-01-01T00:00:01Z".to_owned(),
        };
        receipt.receipt_sha256 = receipt_digest(&receipt);
        receipt
    }

    fn manifest() -> ModelManifestObservation {
        ModelManifestObservation {
            profile_id: ModelProfileId::from_raw("profile-exact"),
            manifest_sha256: SHA.to_owned(),
            artifact_sha256: SHA.to_owned(),
            tokenizer_sha256: SHA.to_owned(),
            template_sha256: SHA.to_owned(),
            codec_sha256: SHA.to_owned(),
            runtime: ModelRuntimeIdentity {
                adapter_id: ModelAdapterId::from_raw("adapter-exact"),
                kind: ModelRuntimeKind::NativeLlamaCpp,
                contract_version: 1,
                runtime_build: "runtime-1.0.0".to_owned(),
                runtime_sha256: SHA.to_owned(),
                platform: PlatformFamily::Fedora,
                architecture: PlatformArchitecture::X86_64,
            },
        }
    }

    #[test]
    fn exactly_four_state_kinds_are_constructed_with_required_provenance() {
        let mut assigner = assigner();
        let observed_source = source("evidence-observed", "subject-observed");
        let observed = assigner
            .assign_observed(
                "assignment-observed",
                claim("claim-observed", "subject-observed"),
                receipt(observed_source.clone()),
                vec![observed_source.clone()],
            )
            .unwrap()
            .clone();
        let derived = assigner
            .assign_derived(
                "assignment-derived",
                claim("claim-derived", "subject-derived"),
                method(),
                vec!["assignment-observed".to_owned()],
            )
            .unwrap()
            .clone();
        let inferred = assigner
            .assign_inferred(
                "assignment-inferred",
                claim("claim-inferred", "subject-inferred"),
                vec![observed_source],
                ModelRunId::from_raw("run-exact"),
                manifest(),
                SHA.to_owned(),
            )
            .unwrap()
            .clone();
        let unknown = assigner
            .assign_unknown_blocked(
                "assignment-unknown",
                claim("claim-unknown", "subject-unknown"),
                UnknownBlockedReason::Unavailable,
                "evidence.source.unavailable".to_owned(),
                Vec::new(),
            )
            .unwrap()
            .clone();

        assert_eq!(
            [observed, derived, inferred, unknown].map(|item| item.evidence_state.kind()),
            [
                MaterialClaimEvidenceStateKind::Observed,
                MaterialClaimEvidenceStateKind::Derived,
                MaterialClaimEvidenceStateKind::Inferred,
                MaterialClaimEvidenceStateKind::UnknownBlocked,
            ]
        );
    }

    #[test]
    fn observed_rejects_every_non_success_non_observe_missing_tool_and_receipt_tampering() {
        let source = source("evidence-observed", "subject-observed");
        for mutate in 0..8 {
            let mut candidate = receipt(source.clone());
            match mutate {
                0 => candidate.outcome = OperationOutcome::Denied,
                1 => candidate.outcome = OperationOutcome::Failed,
                2 => candidate.outcome = OperationOutcome::Cancelled,
                3 => candidate.outcome = OperationOutcome::TimedOut,
                4 => candidate.outcome = OperationOutcome::Uncertain,
                5 => {
                    candidate.operation = OperationBinding::new(GrantOperation::CommandExecute);
                    candidate.operation_sha256 =
                        sha256_hex(&serde_json::to_vec(&candidate.operation).unwrap());
                }
                6 => candidate.tool_call_id = None,
                _ => {
                    candidate.receipt_sha256 = "b".repeat(64);
                    let mut assigner = assigner();
                    assert!(
                        assigner
                            .assign_observed(
                                "assignment-observed",
                                claim("claim-observed", "subject-observed"),
                                candidate,
                                vec![source.clone()],
                            )
                            .is_err()
                    );
                    continue;
                }
            }
            candidate.receipt_sha256 = receipt_digest(&candidate);
            let mut assigner = assigner();
            let result = assigner.assign_observed(
                "assignment-observed",
                claim("claim-observed", "subject-observed"),
                candidate,
                vec![source.clone()],
            );
            assert_eq!(
                result.map(|_| ()),
                Err(EvidenceStateAssignmentError::InvalidObservedProvenance)
            );
        }
    }

    #[test]
    fn observed_requires_exact_receipt_sources_subject_and_revision() {
        let expected = source("evidence-observed", "subject-observed");
        let valid_receipt = receipt(expected.clone());
        for mut sources in [
            Vec::new(),
            vec![source("evidence-other", "subject-observed")],
            vec![source("evidence-observed", "subject-other")],
        ] {
            if let Some(source) = sources.first_mut()
                && source.evidence_id.as_str() == "evidence-observed"
            {
                source.observed_revision = Some("revision-old".to_owned());
            }
            assert!(
                assigner()
                    .assign_observed(
                        "assignment-observed",
                        claim("claim-observed", "subject-observed"),
                        valid_receipt.clone(),
                        sources,
                    )
                    .is_err()
            );
        }
    }

    #[test]
    fn derived_requires_registered_method_and_distinct_observed_inputs() {
        let mut assigner = assigner();
        let observed_source = source("evidence-observed", "subject-observed");
        assigner
            .assign_observed(
                "assignment-observed",
                claim("claim-observed", "subject-observed"),
                receipt(observed_source.clone()),
                vec![observed_source],
            )
            .unwrap();
        let mut changed_method = method();
        changed_method.method_version = "2.0.0".to_owned();
        assert_eq!(
            assigner
                .assign_derived(
                    "assignment-derived-bad",
                    claim("claim-derived-bad", "subject-derived"),
                    changed_method,
                    vec!["assignment-observed".to_owned()],
                )
                .map(|_| ()),
            Err(EvidenceStateAssignmentError::InvalidDerivedProvenance)
        );
        assert_eq!(
            assigner
                .assign_derived(
                    "assignment-derived-missing",
                    claim("claim-derived-missing", "subject-derived"),
                    method(),
                    vec!["assignment-missing".to_owned()],
                )
                .map(|_| ()),
            Err(EvidenceStateAssignmentError::MissingInput)
        );
        assigner
            .assign_unknown_blocked(
                "assignment-unknown",
                claim("claim-unknown", "subject-unknown"),
                UnknownBlockedReason::Failed,
                "tool.failed".to_owned(),
                Vec::new(),
            )
            .unwrap();
        assert_eq!(
            assigner
                .assign_derived(
                    "assignment-derived-unknown",
                    claim("claim-derived-unknown", "subject-derived"),
                    method(),
                    vec!["assignment-unknown".to_owned()],
                )
                .map(|_| ()),
            Err(EvidenceStateAssignmentError::InvalidDerivedProvenance)
        );
    }

    #[test]
    fn inferred_requires_citations_and_exact_model_runtime_manifest() {
        for mutate in 0..4 {
            let mut citations = vec![source("evidence-citation", "subject-source")];
            let mut manifest = manifest();
            let mut response = SHA.to_owned();
            match mutate {
                0 => citations.clear(),
                1 => manifest.manifest_sha256 = "bad".to_owned(),
                2 => manifest.runtime.runtime_build.clear(),
                _ => response = "bad".to_owned(),
            }
            assert_eq!(
                assigner()
                    .assign_inferred(
                        "assignment-inferred",
                        claim("claim-inferred", "subject-inferred"),
                        citations,
                        ModelRunId::from_raw("run-exact"),
                        manifest,
                        response,
                    )
                    .map(|_| ()),
                Err(EvidenceStateAssignmentError::InvalidInferredProvenance)
            );
        }
    }

    #[test]
    fn every_unknown_blocked_reason_is_explicit_and_empty_evidence_is_valid() {
        for (index, reason) in [
            UnknownBlockedReason::Unavailable,
            UnknownBlockedReason::Denied,
            UnknownBlockedReason::Failed,
            UnknownBlockedReason::Conflict,
            UnknownBlockedReason::StaleEvidence,
            UnknownBlockedReason::UnsupportedParsing,
            UnknownBlockedReason::UnverifiableData,
            UnknownBlockedReason::ScopeExcluded,
        ]
        .into_iter()
        .enumerate()
        {
            let mut assigner = assigner();
            let assignment = assigner
                .assign_unknown_blocked(
                    format!("assignment-unknown-{index}"),
                    claim(&format!("claim-unknown-{index}"), "subject-unknown"),
                    reason,
                    format!("evidence.blocked.reason-{index}"),
                    Vec::new(),
                )
                .unwrap();
            assert!(matches!(
                assignment.evidence_state,
                MaterialClaimEvidenceState::UnknownBlocked(_)
            ));
        }
    }

    #[test]
    fn malformed_detail_duplicate_source_and_duplicate_assignment_fail_closed() {
        assert_eq!(
            assigner()
                .assign_unknown_blocked(
                    "assignment-unknown",
                    claim("claim-unknown", "subject-unknown"),
                    UnknownBlockedReason::Denied,
                    "Contains Spaces".to_owned(),
                    Vec::new(),
                )
                .map(|_| ()),
            Err(EvidenceStateAssignmentError::InvalidUnknownBlockedProvenance)
        );
        let mut assigner = assigner();
        assigner
            .assign_unknown_blocked(
                "assignment-unknown",
                claim("claim-unknown", "subject-unknown"),
                UnknownBlockedReason::Denied,
                "policy.denied".to_owned(),
                Vec::new(),
            )
            .unwrap();
        assert_eq!(
            assigner
                .assign_unknown_blocked(
                    "assignment-unknown",
                    claim("claim-other", "subject-other"),
                    UnknownBlockedReason::Failed,
                    "tool.failed".to_owned(),
                    Vec::new(),
                )
                .map(|_| ()),
            Err(EvidenceStateAssignmentError::AlreadyAssigned)
        );

        let duplicate = source("evidence-duplicate", "subject-unknown");
        assert_eq!(
            assigner
                .assign_unknown_blocked(
                    "assignment-duplicate-source",
                    claim("claim-duplicate-source", "subject-unknown"),
                    UnknownBlockedReason::Conflict,
                    "evidence.conflict".to_owned(),
                    vec![duplicate.clone(), duplicate],
                )
                .map(|_| ()),
            Err(EvidenceStateAssignmentError::InvalidUnknownBlockedProvenance)
        );
    }

    #[test]
    fn evidence_assignments_are_descriptive_and_never_grant_authority() {
        let mut assigner = assigner();
        let assignment = assigner
            .assign_unknown_blocked(
                "assignment-unknown",
                claim("claim-unknown", "subject-unknown"),
                UnknownBlockedReason::ScopeExcluded,
                "scope.excluded".to_owned(),
                Vec::new(),
            )
            .unwrap();
        assert_eq!(
            crate::authority::reject_as_authority(assignment).artifact_kind,
            crate::authority::DescriptiveArtifactKind::ClaimRecord
        );
    }

    #[test]
    fn model_confidence_and_unknown_state_fields_cannot_enter_the_wire_contract() {
        let mut assigner = assigner();
        let assignment = assigner
            .assign_unknown_blocked(
                "assignment-unknown",
                claim("claim-unknown", "subject-unknown"),
                UnknownBlockedReason::UnverifiableData,
                "evidence.unverifiable".to_owned(),
                Vec::new(),
            )
            .unwrap();
        let mut candidate = serde_json::to_value(assignment).unwrap();
        candidate["evidence_state"]["model_confidence"] = serde_json::json!(0.999);
        assert!(
            from_json::<agentmage_kernel_contracts::MaterialClaimEvidenceAssignment>(
                &serde_json::to_vec(&candidate).unwrap()
            )
            .is_err()
        );
    }
}
