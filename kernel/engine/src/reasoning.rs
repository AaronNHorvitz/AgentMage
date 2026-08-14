//! Bounded concise reasoning records and deterministic verification gates.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    AssumptionRecord, AssumptionStatus, CONTRACT_SCHEMA_VERSION, ClaimAssertion, ClaimStatus,
    ClarificationQuestion, ClarificationState, ContradictionRecord, EvidenceReference,
    HypothesisRecord, HypothesisStatus, IndependentVerificationRequest,
    IndependentVerificationResult, ProblemFrame, VerificationDisposition,
};

const MAX_TEXT_BYTES: usize = 4_096;
const MAX_ITEMS: usize = 128;
const MAX_EVIDENCE: usize = 256;
const MAX_REVISIONS: u32 = 1_024;

/// Typed reason a reasoning record or transition is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReasoningError {
    /// A record is malformed, oversized, duplicated, or internally inconsistent.
    InvalidRecord,
    /// A referenced record does not exist.
    RecordUnknown,
    /// A lifecycle transition is illegal or lacks required evidence.
    TransitionInvalid,
    /// The bounded revision limit has been reached.
    RevisionLimitReached,
    /// Independent verification does not match its request.
    VerificationInvalid,
}

/// Bounded task reasoning state containing concise records, never private chain of thought.
pub struct ReasoningLedger {
    frame: ProblemFrame,
    assumptions: Vec<AssumptionRecord>,
    hypotheses: Vec<HypothesisRecord>,
    questions: Vec<ClarificationQuestion>,
    revision: u32,
}

impl ReasoningLedger {
    /// Starts one ledger from a valid compact problem frame.
    pub fn new(frame: ProblemFrame) -> Result<Self, ReasoningError> {
        validate_frame(&frame)?;
        Ok(Self {
            frame,
            assumptions: Vec::new(),
            hypotheses: Vec::new(),
            questions: Vec::new(),
            revision: 1,
        })
    }

    /// Returns the exact problem frame.
    #[must_use]
    pub fn frame(&self) -> &ProblemFrame {
        &self.frame
    }

    /// Returns the current monotonic ledger revision.
    #[must_use]
    pub const fn revision(&self) -> u32 {
        self.revision
    }

    /// Returns assumptions in insertion order.
    #[must_use]
    pub fn assumptions(&self) -> &[AssumptionRecord] {
        &self.assumptions
    }

    /// Returns hypotheses in insertion order.
    #[must_use]
    pub fn hypotheses(&self) -> &[HypothesisRecord] {
        &self.hypotheses
    }

    /// Appends one valid, initially unverified assumption.
    pub fn register_assumption(&mut self, record: AssumptionRecord) -> Result<(), ReasoningError> {
        validate_assumption(&record)?;
        if record.status != AssumptionStatus::Unverified
            || !record.evidence.is_empty()
            || self.assumptions.len() >= MAX_ITEMS
            || self
                .assumptions
                .iter()
                .any(|current| current.assumption_id == record.assumption_id)
        {
            return Err(ReasoningError::InvalidRecord);
        }
        self.bump()?;
        self.assumptions.push(record);
        Ok(())
    }

    /// Confirms or rejects one assumption using current evidence.
    pub fn resolve_assumption(
        &mut self,
        assumption_id: &str,
        status: AssumptionStatus,
        evidence: Vec<EvidenceReference>,
    ) -> Result<(), ReasoningError> {
        if status == AssumptionStatus::Unverified
            || evidence.is_empty()
            || evidence.len() > MAX_EVIDENCE
            || self.retained_evidence_count() + evidence.len() > MAX_EVIDENCE
            || !valid_evidence_set(&evidence)
        {
            return Err(ReasoningError::TransitionInvalid);
        }
        let index = self
            .assumptions
            .iter()
            .position(|record| record.assumption_id == assumption_id)
            .ok_or(ReasoningError::RecordUnknown)?;
        if self.assumptions[index].status != AssumptionStatus::Unverified {
            return Err(ReasoningError::TransitionInvalid);
        }
        self.bump()?;
        self.assumptions[index].status = status;
        self.assumptions[index].evidence = evidence;
        Ok(())
    }

    /// Appends one valid, initially open hypothesis.
    pub fn register_hypothesis(&mut self, record: HypothesisRecord) -> Result<(), ReasoningError> {
        validate_hypothesis(&record)?;
        if record.status != HypothesisStatus::Open
            || !record.evidence_for.is_empty()
            || !record.evidence_against.is_empty()
            || self.hypotheses.len() >= MAX_ITEMS
            || self
                .hypotheses
                .iter()
                .any(|current| current.hypothesis_id == record.hypothesis_id)
        {
            return Err(ReasoningError::InvalidRecord);
        }
        self.bump()?;
        self.hypotheses.push(record);
        Ok(())
    }

    /// Resolves one open hypothesis from discriminating evidence.
    pub fn resolve_hypothesis(
        &mut self,
        hypothesis_id: &str,
        status: HypothesisStatus,
        evidence_for: Vec<EvidenceReference>,
        evidence_against: Vec<EvidenceReference>,
    ) -> Result<(), ReasoningError> {
        if status == HypothesisStatus::Open
            || evidence_for.len() + evidence_against.len() > MAX_EVIDENCE
            || self.retained_evidence_count() + evidence_for.len() + evidence_against.len()
                > MAX_EVIDENCE
            || !valid_evidence_set(&evidence_for)
            || !valid_evidence_set(&evidence_against)
            || (status == HypothesisStatus::Supported && evidence_for.is_empty())
            || (status == HypothesisStatus::Rejected && evidence_against.is_empty())
            || (status == HypothesisStatus::Inconclusive
                && evidence_for.is_empty()
                && evidence_against.is_empty())
        {
            return Err(ReasoningError::TransitionInvalid);
        }
        let index = self
            .hypotheses
            .iter()
            .position(|record| record.hypothesis_id == hypothesis_id)
            .ok_or(ReasoningError::RecordUnknown)?;
        if self.hypotheses[index].status != HypothesisStatus::Open {
            return Err(ReasoningError::TransitionInvalid);
        }
        self.bump()?;
        self.hypotheses[index].status = status;
        self.hypotheses[index].evidence_for = evidence_for;
        self.hypotheses[index].evidence_against = evidence_against;
        Ok(())
    }

    /// Adds one material clarification question.
    pub fn register_question(
        &mut self,
        question: ClarificationQuestion,
    ) -> Result<(), ReasoningError> {
        validate_question(&question)?;
        if self.questions.len() >= MAX_ITEMS
            || question.answer.is_some()
            || self
                .questions
                .iter()
                .any(|current| current.question_id == question.question_id)
        {
            return Err(ReasoningError::InvalidRecord);
        }
        self.bump()?;
        self.questions.push(question);
        Ok(())
    }

    /// Records one explicit answer without inferring user intent.
    pub fn answer_question(
        &mut self,
        question_id: &str,
        answer: String,
    ) -> Result<(), ReasoningError> {
        if !valid_text(&answer) {
            return Err(ReasoningError::InvalidRecord);
        }
        let index = self
            .questions
            .iter()
            .position(|record| record.question_id == question_id)
            .ok_or(ReasoningError::RecordUnknown)?;
        if self.questions[index].answer.is_some() {
            return Err(ReasoningError::TransitionInvalid);
        }
        self.bump()?;
        self.questions[index].answer = Some(answer);
        Ok(())
    }

    /// Returns whether any material question still needs the user.
    #[must_use]
    pub fn clarification_state(&self) -> ClarificationState {
        if self
            .questions
            .iter()
            .any(|question| question.answer.is_none())
        {
            ClarificationState::UserDecisionRequired
        } else {
            ClarificationState::Ready
        }
    }

    /// Builds a verifier input that cannot carry first-pass assumptions or hypotheses.
    pub fn verification_request(
        &self,
        proposed_result: String,
        evidence: Vec<EvidenceReference>,
    ) -> Result<IndependentVerificationRequest, ReasoningError> {
        if !valid_text(&proposed_result)
            || evidence.len() > MAX_EVIDENCE
            || !valid_evidence_set(&evidence)
        {
            return Err(ReasoningError::VerificationInvalid);
        }
        Ok(IndependentVerificationRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: self.frame.task_id.clone(),
            proposed_result,
            acceptance_checks: self.frame.acceptance_checks.clone(),
            evidence,
        })
    }

    fn bump(&mut self) -> Result<(), ReasoningError> {
        self.revision = self
            .revision
            .checked_add(1)
            .filter(|revision| *revision <= MAX_REVISIONS)
            .ok_or(ReasoningError::RevisionLimitReached)?;
        Ok(())
    }

    fn retained_evidence_count(&self) -> usize {
        self.assumptions
            .iter()
            .map(|record| record.evidence.len())
            .sum::<usize>()
            + self
                .hypotheses
                .iter()
                .map(|record| record.evidence_for.len() + record.evidence_against.len())
                .sum::<usize>()
    }
}

/// Finds exact conflicting values for the same typed claim key.
pub fn find_contradictions(
    assertions: &[ClaimAssertion],
) -> Result<Vec<ContradictionRecord>, ReasoningError> {
    if assertions.len() > MAX_ITEMS
        || assertions
            .iter()
            .map(|assertion| assertion.evidence.len())
            .sum::<usize>()
            > MAX_EVIDENCE
        || assertions
            .iter()
            .any(|assertion| !valid_assertion(assertion))
    {
        return Err(ReasoningError::InvalidRecord);
    }
    let mut contradictions = Vec::new();
    for (left_index, left) in assertions.iter().enumerate() {
        for (right_index, right) in assertions.iter().enumerate().skip(left_index + 1) {
            if left.claim_key == right.claim_key && left.value != right.value {
                contradictions.push(ContradictionRecord {
                    claim_key: left.claim_key.clone(),
                    left_index: u32::try_from(left_index).expect("assertion bound fits u32"),
                    right_index: u32::try_from(right_index).expect("assertion bound fits u32"),
                });
            }
        }
    }
    Ok(contradictions)
}

/// Validates one independently produced disposition against its exact request.
pub fn complete_independent_verification(
    request: &IndependentVerificationRequest,
    disposition: VerificationDisposition,
    verified_checks: Vec<String>,
    findings: Vec<String>,
    evidence: Vec<EvidenceReference>,
) -> Result<IndependentVerificationResult, ReasoningError> {
    validate_verification_request(request)?;
    if verified_checks.len() > MAX_ITEMS
        || findings.len() > MAX_ITEMS
        || evidence.len() > MAX_EVIDENCE
        || verified_checks.iter().any(|value| !valid_text(value))
        || findings.iter().any(|value| !valid_text(value))
        || !valid_evidence_set(&evidence)
        || verified_checks
            .iter()
            .any(|check| !request.acceptance_checks.contains(check))
        || has_duplicates(&verified_checks)
        || (disposition == VerificationDisposition::Pass
            && (verified_checks != request.acceptance_checks
                || !findings.is_empty()
                || evidence.is_empty()))
        || (disposition != VerificationDisposition::Pass && findings.is_empty())
    {
        return Err(ReasoningError::VerificationInvalid);
    }
    Ok(IndependentVerificationResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        task_id: request.task_id.clone(),
        disposition,
        verified_checks,
        findings,
        evidence,
    })
}

fn validate_frame(frame: &ProblemFrame) -> Result<(), ReasoningError> {
    let collections = [
        &frame.constraints,
        &frame.unknowns,
        &frame.acceptance_checks,
        &frame.risks,
    ];
    if frame.schema_version != CONTRACT_SCHEMA_VERSION
        || frame.task_id.as_str().is_empty()
        || !valid_text(&frame.objective)
        || frame.acceptance_checks.is_empty()
        || frame.known_facts.len() > MAX_ITEMS
        || frame
            .known_facts
            .iter()
            .map(|fact| fact.evidence.len())
            .sum::<usize>()
            > MAX_EVIDENCE
        || collections.iter().any(|values| {
            values.len() > MAX_ITEMS
                || values.iter().any(|value| !valid_text(value))
                || has_duplicates(values)
        })
        || frame.known_facts.iter().any(|fact| {
            !valid_key(&fact.claim_key)
                || !valid_text(&fact.statement)
                || fact.evidence.len() > MAX_EVIDENCE
                || !valid_evidence_set(&fact.evidence)
                || (matches!(
                    fact.status,
                    ClaimStatus::Known | ClaimStatus::DirectlyObserved
                ) && fact.evidence.is_empty())
        })
    {
        return Err(ReasoningError::InvalidRecord);
    }
    Ok(())
}

fn validate_assumption(record: &AssumptionRecord) -> Result<(), ReasoningError> {
    if !valid_key(&record.assumption_id)
        || !valid_text(&record.statement)
        || !valid_text(&record.rationale)
        || !valid_text(&record.verification_method)
        || record.evidence.len() > MAX_EVIDENCE
        || !valid_evidence_set(&record.evidence)
    {
        return Err(ReasoningError::InvalidRecord);
    }
    Ok(())
}

fn validate_hypothesis(record: &HypothesisRecord) -> Result<(), ReasoningError> {
    if !valid_key(&record.hypothesis_id)
        || !valid_text(&record.statement)
        || record.discriminating_tests.is_empty()
        || record.discriminating_tests.len() > MAX_ITEMS
        || record
            .discriminating_tests
            .iter()
            .any(|test| !valid_text(test))
        || has_duplicates(&record.discriminating_tests)
        || record.evidence_for.len() + record.evidence_against.len() > MAX_EVIDENCE
        || !valid_evidence_set(&record.evidence_for)
        || !valid_evidence_set(&record.evidence_against)
    {
        return Err(ReasoningError::InvalidRecord);
    }
    Ok(())
}

fn validate_question(question: &ClarificationQuestion) -> Result<(), ReasoningError> {
    let unique = question.impacts.iter().copied().collect::<BTreeSet<_>>();
    if !valid_key(&question.question_id)
        || !valid_text(&question.question)
        || question.impacts.is_empty()
        || unique.len() != question.impacts.len()
        || question
            .answer
            .as_ref()
            .is_some_and(|answer| !valid_text(answer))
    {
        return Err(ReasoningError::InvalidRecord);
    }
    Ok(())
}

fn validate_verification_request(
    request: &IndependentVerificationRequest,
) -> Result<(), ReasoningError> {
    if request.schema_version != CONTRACT_SCHEMA_VERSION
        || request.task_id.as_str().is_empty()
        || !valid_text(&request.proposed_result)
        || request.acceptance_checks.is_empty()
        || request.acceptance_checks.len() > MAX_ITEMS
        || request
            .acceptance_checks
            .iter()
            .any(|check| !valid_text(check))
        || has_duplicates(&request.acceptance_checks)
        || request.evidence.len() > MAX_EVIDENCE
        || !valid_evidence_set(&request.evidence)
    {
        return Err(ReasoningError::VerificationInvalid);
    }
    Ok(())
}

fn valid_assertion(assertion: &ClaimAssertion) -> bool {
    valid_key(&assertion.claim_key)
        && valid_text(&assertion.value)
        && assertion.evidence.len() <= MAX_EVIDENCE
        && valid_evidence_set(&assertion.evidence)
        && (!matches!(
            assertion.status,
            ClaimStatus::Known | ClaimStatus::DirectlyObserved
        ) || !assertion.evidence.is_empty())
}

fn valid_evidence_set(evidence: &[EvidenceReference]) -> bool {
    let unique = evidence
        .iter()
        .map(|item| item.evidence_id.as_str())
        .collect::<BTreeSet<_>>();
    unique.len() == evidence.len()
        && evidence.iter().all(|item| {
            item.schema_version == CONTRACT_SCHEMA_VERSION
                && !item.evidence_id.as_str().is_empty()
                && valid_text(&item.source_id)
                && valid_text(&item.object_id)
                && item
                    .fragment
                    .as_ref()
                    .is_none_or(|fragment| valid_text(fragment))
                && item
                    .observed_revision
                    .as_ref()
                    .is_none_or(|revision| valid_text(revision))
                && item.content_sha256.len() == 64
                && item
                    .content_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
}

fn valid_key(value: &str) -> bool {
    valid_text(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT_BYTES
}

fn has_duplicates(values: &[String]) -> bool {
    let unique = values.iter().collect::<BTreeSet<_>>();
    unique.len() != values.len()
}

#[cfg(test)]
mod tests {
    use super::{
        ReasoningError, ReasoningLedger, complete_independent_verification, find_contradictions,
    };
    use agentmage_kernel_contracts::{
        AssumptionRecord, AssumptionRisk, AssumptionStatus, AuthorityClass,
        CONTRACT_SCHEMA_VERSION, ClaimAssertion, ClaimStatus, ClarificationImpact,
        ClarificationQuestion, ClarificationState, EvidenceId, EvidenceKind, EvidenceReference,
        HypothesisRecord, HypothesisStatus, ProblemFact, ProblemFrame, TaskId,
        VerificationDisposition,
    };

    fn evidence(id: &str) -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(id),
            kind: EvidenceKind::Validation,
            source_id: "synthetic".to_owned(),
            object_id: id.to_owned(),
            fragment: None,
            content_sha256: "a".repeat(64),
            observed_revision: Some("fixture-v1".to_owned()),
        }
    }

    fn frame() -> ProblemFrame {
        ProblemFrame {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: TaskId::from_raw("task-reasoning"),
            objective: "Verify a bounded result".to_owned(),
            known_facts: vec![ProblemFact {
                claim_key: "workspace.state".to_owned(),
                statement: "The fixture is clean".to_owned(),
                status: ClaimStatus::DirectlyObserved,
                evidence: vec![evidence("fact")],
            }],
            constraints: vec!["Remain read only".to_owned()],
            unknowns: vec!["Whether the candidate passes".to_owned()],
            acceptance_checks: vec!["Check output".to_owned(), "Check limits".to_owned()],
            risks: vec!["False completion".to_owned()],
            authority_boundary: AuthorityClass::Observe,
        }
    }

    #[test]
    fn frame_assumption_and_hypothesis_lifecycles_are_bounded() {
        let mut ledger = ReasoningLedger::new(frame()).expect("valid frame");
        assert_eq!(
            crate::authority::reject_as_authority(ledger.frame()).artifact_kind,
            crate::authority::DescriptiveArtifactKind::ReasoningRecord
        );
        ledger
            .register_assumption(AssumptionRecord {
                assumption_id: "assumption-1".to_owned(),
                statement: "The fixture is representative".to_owned(),
                rationale: "A bounded test needs one fixture".to_owned(),
                risk: AssumptionRisk::Moderate,
                verification_method: "Compare another fixture".to_owned(),
                status: AssumptionStatus::Unverified,
                evidence: Vec::new(),
            })
            .expect("assumption");
        ledger
            .resolve_assumption(
                "assumption-1",
                AssumptionStatus::Confirmed,
                vec![evidence("a")],
            )
            .expect("resolve assumption");
        ledger
            .register_hypothesis(HypothesisRecord {
                hypothesis_id: "hypothesis-1".to_owned(),
                statement: "The output is deterministic".to_owned(),
                evidence_for: Vec::new(),
                evidence_against: Vec::new(),
                discriminating_tests: vec!["Repeat with reordered input".to_owned()],
                status: HypothesisStatus::Open,
            })
            .expect("hypothesis");
        ledger
            .resolve_hypothesis(
                "hypothesis-1",
                HypothesisStatus::Supported,
                vec![evidence("h")],
                Vec::new(),
            )
            .expect("resolve hypothesis");
        assert_eq!(ledger.revision(), 5);
        assert_eq!(ledger.assumptions()[0].status, AssumptionStatus::Confirmed);
        assert_eq!(ledger.hypotheses()[0].status, HypothesisStatus::Supported);
    }

    #[test]
    fn malformed_duplicate_and_unsupported_transitions_fail_without_revision_change() {
        let mut ledger = ReasoningLedger::new(frame()).expect("valid frame");
        let record = AssumptionRecord {
            assumption_id: "assumption-1".to_owned(),
            statement: "An assumption".to_owned(),
            rationale: "Needed for test".to_owned(),
            risk: AssumptionRisk::Low,
            verification_method: "Inspect evidence".to_owned(),
            status: AssumptionStatus::Unverified,
            evidence: Vec::new(),
        };
        ledger.register_assumption(record.clone()).expect("first");
        assert_eq!(
            ledger.register_assumption(record),
            Err(ReasoningError::InvalidRecord)
        );
        assert_eq!(
            ledger.resolve_assumption("assumption-1", AssumptionStatus::Confirmed, Vec::new()),
            Err(ReasoningError::TransitionInvalid)
        );
        assert_eq!(ledger.revision(), 2);
    }

    #[test]
    fn exact_claim_keys_expose_conflicts_without_language_inference() {
        let assertions = vec![
            ClaimAssertion {
                claim_key: "branch.head".to_owned(),
                value: "abc".to_owned(),
                status: ClaimStatus::Known,
                evidence: vec![evidence("left")],
            },
            ClaimAssertion {
                claim_key: "branch.head".to_owned(),
                value: "def".to_owned(),
                status: ClaimStatus::DirectlyObserved,
                evidence: vec![evidence("right")],
            },
            ClaimAssertion {
                claim_key: "workspace.state".to_owned(),
                value: "clean".to_owned(),
                status: ClaimStatus::Inferred,
                evidence: Vec::new(),
            },
        ];
        let conflicts = find_contradictions(&assertions).expect("valid assertions");
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].claim_key, "branch.head");
        assert_eq!((conflicts[0].left_index, conflicts[0].right_index), (0, 1));
    }

    #[test]
    fn material_question_blocks_until_explicitly_answered() {
        let mut ledger = ReasoningLedger::new(frame()).expect("valid frame");
        assert_eq!(
            ledger.register_question(ClarificationQuestion {
                question_id: "question-preanswered".to_owned(),
                question: "Which workspace is in scope?".to_owned(),
                impacts: vec![ClarificationImpact::Scope],
                answer: Some("A candidate answer".to_owned()),
            }),
            Err(ReasoningError::InvalidRecord)
        );
        ledger
            .register_question(ClarificationQuestion {
                question_id: "question-1".to_owned(),
                question: "Which workspace is in scope?".to_owned(),
                impacts: vec![ClarificationImpact::Scope, ClarificationImpact::Authority],
                answer: None,
            })
            .expect("question");
        assert_eq!(
            ledger.clarification_state(),
            ClarificationState::UserDecisionRequired
        );
        ledger
            .answer_question("question-1", "Only the synthetic fixture".to_owned())
            .expect("answer");
        assert_eq!(ledger.clarification_state(), ClarificationState::Ready);
    }

    #[test]
    fn independent_verification_requires_exact_checks_evidence_and_truthful_disposition() {
        let ledger = ReasoningLedger::new(frame()).expect("valid frame");
        let request = ledger
            .verification_request("Candidate result".to_owned(), vec![evidence("input")])
            .expect("request");
        let pass = complete_independent_verification(
            &request,
            VerificationDisposition::Pass,
            request.acceptance_checks.clone(),
            Vec::new(),
            vec![evidence("verification")],
        )
        .expect("pass");
        assert_eq!(pass.disposition, VerificationDisposition::Pass);
        assert_eq!(
            complete_independent_verification(
                &request,
                VerificationDisposition::Pass,
                vec!["Check output".to_owned()],
                Vec::new(),
                vec![evidence("partial")],
            ),
            Err(ReasoningError::VerificationInvalid)
        );
        let serialized = serde_json::to_value(request).expect("serialize request");
        let object = serialized.as_object().expect("request object");
        assert!(!object.contains_key("assumptions"));
        assert!(!object.contains_key("hypotheses"));
        assert!(!object.contains_key("first_pass_conclusions"));
    }
}
