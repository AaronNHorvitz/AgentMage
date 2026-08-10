//! Bounded work-packet validation, state transitions, history, and planning.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, EvidenceReference, Plan, PlanId, PlanState, PlanStep, PlanStepId,
    PlanStepState, StopConditionKind, ValidationIssue, ValidationSeverity, WorkPacket,
    WorkPacketState,
};

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_LIST_ITEMS: usize = 128;
const MAX_EVIDENCE_ITEMS: usize = 256;
const REQUIRED_STOP_CONDITIONS: [StopConditionKind; 7] = [
    StopConditionKind::AcceptanceSatisfied,
    StopConditionKind::UserDecisionRequired,
    StopConditionKind::PolicyDenied,
    StopConditionKind::Error,
    StopConditionKind::Cancelled,
    StopConditionKind::BudgetExhausted,
    StopConditionKind::UncertainResult,
];

/// Error returned when a packet revision cannot enter immutable history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkPacketHistoryError {
    /// The proposed packet failed closed validation.
    InvalidPacket {
        /// Exact ordered validation findings.
        issues: Vec<ValidationIssue>,
    },
    /// The first packet did not use revision one.
    InvalidInitialRevision {
        /// Observed initial revision.
        actual: u32,
    },
    /// A later revision changed the stable packet or task identity.
    IdentityChanged {
        /// Stable field whose identity changed.
        field: &'static str,
    },
    /// A revision was repeated or skipped.
    RevisionOutOfSequence {
        /// Exact next revision required.
        expected: u32,
        /// Observed candidate revision.
        actual: u32,
    },
    /// The revision counter cannot increase further.
    RevisionExhausted,
    /// The proposed lifecycle transition is not legal.
    IllegalTransition {
        /// Typed transition finding.
        issue: Box<ValidationIssue>,
    },
}

/// Append-only in-memory history for one stable work packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkPacketHistory {
    revisions: Vec<WorkPacket>,
}

impl WorkPacketHistory {
    /// Starts a history with one valid revision-one packet.
    pub fn new(initial: WorkPacket) -> Result<Self, WorkPacketHistoryError> {
        if initial.revision != 1 {
            return Err(WorkPacketHistoryError::InvalidInitialRevision {
                actual: initial.revision,
            });
        }
        let issues = validate_packet(&initial);
        if !issues.is_empty() {
            return Err(WorkPacketHistoryError::InvalidPacket { issues });
        }
        Ok(Self {
            revisions: vec![initial],
        })
    }

    /// Returns every immutable revision in ascending order.
    #[must_use]
    pub fn revisions(&self) -> &[WorkPacket] {
        &self.revisions
    }

    /// Returns the current immutable revision.
    #[must_use]
    pub fn current(&self) -> &WorkPacket {
        self.revisions
            .last()
            .expect("history construction always records one revision")
    }

    /// Appends one exact next revision after full validation.
    pub fn append_revision(&mut self, candidate: WorkPacket) -> Result<(), WorkPacketHistoryError> {
        let issues = validate_packet(&candidate);
        if !issues.is_empty() {
            return Err(WorkPacketHistoryError::InvalidPacket { issues });
        }
        let current = self.current();
        if candidate.work_packet_id != current.work_packet_id {
            return Err(WorkPacketHistoryError::IdentityChanged {
                field: "work_packet_id",
            });
        }
        if candidate.task_id != current.task_id {
            return Err(WorkPacketHistoryError::IdentityChanged { field: "task_id" });
        }
        let expected = current
            .revision
            .checked_add(1)
            .ok_or(WorkPacketHistoryError::RevisionExhausted)?;
        if candidate.revision != expected {
            return Err(WorkPacketHistoryError::RevisionOutOfSequence {
                expected,
                actual: candidate.revision,
            });
        }
        validate_transition(current.state, candidate.state).map_err(|issue| {
            WorkPacketHistoryError::IllegalTransition {
                issue: Box::new(issue),
            }
        })?;
        self.revisions.push(candidate);
        Ok(())
    }
}

/// Validates all structural and lifecycle-conditional work-packet fields.
#[must_use]
pub fn validate_packet(packet: &WorkPacket) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if packet.schema_version != CONTRACT_SCHEMA_VERSION {
        issues.push(issue(
            "packet.schema_version.unsupported",
            "schema_version",
            "Work-packet schema version is unsupported",
        ));
    }
    validate_identifier(
        "work_packet_id",
        packet.work_packet_id.as_str(),
        &mut issues,
    );
    validate_identifier("task_id", packet.task_id.as_str(), &mut issues);
    if packet.revision == 0 {
        issues.push(issue(
            "packet.revision.invalid",
            "revision",
            "Work-packet revision must be greater than zero",
        ));
    }
    for (field, value) in [
        ("objective", packet.objective.as_str()),
        ("reason", packet.reason.as_str()),
        ("owner", packet.owner.as_str()),
        ("expected_output", packet.expected_output.as_str()),
        (
            "required_capability_class",
            packet.required_capability_class.as_str(),
        ),
        ("rollback.description", packet.rollback.description.as_str()),
    ] {
        validate_text(field, value, true, &mut issues);
    }
    validate_text_list(
        "acceptance_checks",
        &packet.acceptance_checks,
        true,
        &mut issues,
    );
    validate_text_list("mutable_files", &packet.mutable_files, false, &mut issues);
    validate_text_list(
        "protected_files",
        &packet.protected_files,
        false,
        &mut issues,
    );
    validate_disjoint_file_declarations(packet, &mut issues);
    validate_required_evidence(packet, &mut issues);
    validate_budgets(packet, &mut issues);
    validate_stop_conditions(packet, &mut issues);
    validate_date(
        "last_verification_date",
        &packet.last_verification_date,
        &mut issues,
    );
    if let Some(value) = &packet.next_review {
        validate_date("next_review", value, &mut issues);
    }
    for (field, value) in [
        ("next_action", packet.next_action.as_deref()),
        ("status_reason", packet.status_reason.as_deref()),
        ("disposition", packet.disposition.as_deref()),
    ] {
        if let Some(value) = value {
            validate_text(field, value, true, &mut issues);
        }
    }
    validate_evidence_list(
        "authoritative_evidence",
        &packet.authoritative_evidence,
        &mut issues,
    );
    validate_completion_entries(packet, &mut issues);
    validate_retained_issues(packet, &mut issues);
    validate_state_fields(packet, &mut issues);
    issues
}

/// Validates one work-state transition without changing either packet.
pub fn validate_transition(
    from: WorkPacketState,
    to: WorkPacketState,
) -> Result<(), ValidationIssue> {
    let allowed = if from == to {
        matches!(
            from,
            WorkPacketState::Draft
                | WorkPacketState::Validated
                | WorkPacketState::Planned
                | WorkPacketState::Active
                | WorkPacketState::Blocked
        )
    } else {
        matches!(
            (from, to),
            (WorkPacketState::Draft, WorkPacketState::Validated)
                | (WorkPacketState::Draft, WorkPacketState::Cancelled)
                | (WorkPacketState::Validated, WorkPacketState::Planned)
                | (WorkPacketState::Validated, WorkPacketState::Cancelled)
                | (WorkPacketState::Validated, WorkPacketState::Superseded)
                | (WorkPacketState::Planned, WorkPacketState::Active)
                | (WorkPacketState::Planned, WorkPacketState::Cancelled)
                | (WorkPacketState::Planned, WorkPacketState::Superseded)
                | (WorkPacketState::Active, WorkPacketState::Blocked)
                | (WorkPacketState::Active, WorkPacketState::Completed)
                | (WorkPacketState::Active, WorkPacketState::Cancelled)
                | (WorkPacketState::Active, WorkPacketState::Superseded)
                | (WorkPacketState::Blocked, WorkPacketState::Active)
                | (WorkPacketState::Blocked, WorkPacketState::Completed)
                | (WorkPacketState::Blocked, WorkPacketState::Cancelled)
                | (WorkPacketState::Blocked, WorkPacketState::Superseded)
        )
    };
    if allowed {
        Ok(())
    } else {
        Err(issue(
            "packet.transition.illegal",
            "state",
            "Work-packet state transition is not permitted",
        ))
    }
}

/// Validates that every acceptance check and required evidence class is satisfied.
#[must_use]
pub fn validate_completion(packet: &WorkPacket) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if packet.state != WorkPacketState::Completed {
        issues.push(issue(
            "packet.completion.state",
            "state",
            "Completion evidence is valid only for a completed packet",
        ));
    }
    for check in &packet.acceptance_checks {
        let matches = packet
            .completion_evidence
            .iter()
            .filter(|entry| entry.acceptance_check == *check)
            .collect::<Vec<_>>();
        if matches.len() != 1 || matches[0].evidence.is_empty() {
            issues.push(issue(
                "packet.completion.acceptance_evidence",
                "completion_evidence",
                "Every acceptance check requires one non-empty evidence entry",
            ));
        }
    }
    for required in &packet.required_evidence {
        let present = packet
            .completion_evidence
            .iter()
            .flat_map(|entry| &entry.evidence)
            .any(|evidence| evidence.kind == *required);
        if !present {
            issues.push(issue(
                "packet.completion.required_evidence",
                "completion_evidence",
                "A required evidence class is absent from completion evidence",
            ));
        }
    }
    issues
}

/// Produces a deterministic descriptive plan for one validated packet revision.
///
/// The returned plan carries no authority. A prior plan may adapt only the same task and
/// stable plan identity, and its revision increases exactly once.
pub fn adapt_packet_to_plan(
    packet: &WorkPacket,
    plan_id: PlanId,
    prior: Option<&Plan>,
) -> Result<Plan, Vec<ValidationIssue>> {
    let mut issues = validate_packet(packet);
    validate_identifier("plan_id", plan_id.as_str(), &mut issues);
    if !matches!(
        packet.state,
        WorkPacketState::Validated
            | WorkPacketState::Planned
            | WorkPacketState::Active
            | WorkPacketState::Blocked
    ) {
        issues.push(issue(
            "packet.plan.state",
            "state",
            "Work-packet state cannot be adapted into a plan",
        ));
    }
    let revision = match prior {
        None => 1,
        Some(plan) => {
            if plan.schema_version != CONTRACT_SCHEMA_VERSION {
                issues.push(issue(
                    "packet.plan.schema_version.unsupported",
                    "schema_version",
                    "Prior plan schema version is unsupported",
                ));
            }
            if plan.task_id != packet.task_id {
                issues.push(issue(
                    "packet.plan.task_mismatch",
                    "task_id",
                    "Prior plan belongs to another task",
                ));
            }
            if plan.plan_id != plan_id {
                issues.push(issue(
                    "packet.plan.identity_mismatch",
                    "plan_id",
                    "Plan adaptation cannot change plan identity",
                ));
            }
            if plan.work_packet_revision >= packet.revision {
                issues.push(issue(
                    "packet.plan.packet_revision_stale",
                    "revision",
                    "Plan adaptation requires a newer work-packet revision",
                ));
            }
            match plan.revision.checked_add(1) {
                Some(revision) => revision,
                None => {
                    issues.push(issue(
                        "packet.plan.revision_exhausted",
                        "revision",
                        "Plan revision cannot increase further",
                    ));
                    0
                }
            }
        }
    };
    if packet
        .plan_id
        .as_ref()
        .is_some_and(|packet_plan_id| packet_plan_id != &plan_id)
    {
        issues.push(issue(
            "packet.plan.packet_identity_mismatch",
            "plan_id",
            "Packet and adapted plan identities do not match",
        ));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut previous = None;
    let steps = packet
        .acceptance_checks
        .iter()
        .enumerate()
        .map(|(ordinal, check)| {
            let step_id = PlanStepId::from_raw(format!("{}:step:{ordinal}", plan_id.as_str()));
            let depends_on = previous.replace(step_id.clone()).into_iter().collect();
            let state = prior
                .and_then(|plan| plan.steps.get(ordinal))
                .filter(|step| {
                    step.description == *check && step.expected_evidence == packet.required_evidence
                })
                .map_or(PlanStepState::Proposed, |step| step.state);
            PlanStep {
                plan_step_id: step_id,
                ordinal: u32::try_from(ordinal).expect("packet item bound fits u32"),
                description: check.clone(),
                depends_on,
                expected_evidence: packet.required_evidence.clone(),
                state,
            }
        })
        .collect();
    Ok(Plan {
        schema_version: CONTRACT_SCHEMA_VERSION,
        plan_id,
        task_id: packet.task_id.clone(),
        work_packet_revision: packet.revision,
        revision,
        steps,
        state: PlanState::Proposed,
    })
}

fn validate_state_fields(packet: &WorkPacket, issues: &mut Vec<ValidationIssue>) {
    match packet.state {
        WorkPacketState::Draft | WorkPacketState::Validated if packet.plan_id.is_some() => {
            issues.push(issue(
                "packet.plan.unexpected",
                "plan_id",
                "Draft and validated work cannot already carry a plan identity",
            ));
        }
        WorkPacketState::Planned
        | WorkPacketState::Active
        | WorkPacketState::Blocked
        | WorkPacketState::Completed
            if packet.plan_id.is_none() =>
        {
            issues.push(issue(
                "packet.plan.required",
                "plan_id",
                "Current work-packet state requires a plan identity",
            ));
        }
        _ => {}
    }
    if let Some(plan_id) = &packet.plan_id {
        validate_identifier("plan_id", plan_id.as_str(), issues);
    }
    if let Some(superseding_work) = &packet.superseding_work {
        validate_identifier("superseding_work", superseding_work.as_str(), issues);
    }
    let reason_required = matches!(
        packet.state,
        WorkPacketState::Blocked | WorkPacketState::Cancelled | WorkPacketState::Superseded
    );
    if reason_required && packet.status_reason.is_none() {
        issues.push(issue(
            "packet.status_reason.required",
            "status_reason",
            "Current work-packet state requires a status reason",
        ));
    }
    let disposition_required = matches!(
        packet.state,
        WorkPacketState::Completed | WorkPacketState::Cancelled | WorkPacketState::Superseded
    );
    if disposition_required && packet.disposition.is_none() {
        issues.push(issue(
            "packet.disposition.required",
            "disposition",
            "Terminal work-packet state requires a disposition",
        ));
    }
    if packet.state == WorkPacketState::Blocked
        && packet.next_action.is_none()
        && packet.next_review.is_none()
    {
        issues.push(issue(
            "packet.blocked.follow_up",
            "next_action",
            "Blocked work requires a next action or next review date",
        ));
    }
    if packet.state == WorkPacketState::Superseded {
        if packet.superseding_work.is_none() {
            issues.push(issue(
                "packet.superseding_work.required",
                "superseding_work",
                "Superseded work requires a replacement packet identity",
            ));
        } else if packet.superseding_work.as_ref() == Some(&packet.work_packet_id) {
            issues.push(issue(
                "packet.superseding_work.self",
                "superseding_work",
                "A packet cannot supersede itself",
            ));
        }
    } else if packet.superseding_work.is_some() {
        issues.push(issue(
            "packet.superseding_work.unexpected",
            "superseding_work",
            "Only superseded work may identify a replacement packet",
        ));
    }
    if packet.state == WorkPacketState::Completed {
        issues.extend(validate_completion(packet));
    } else if !packet.completion_evidence.is_empty() {
        issues.push(issue(
            "packet.completion_evidence.unexpected",
            "completion_evidence",
            "Only completed work may carry completion evidence",
        ));
    }
    if packet.state != WorkPacketState::Draft && !packet.validation_issues.is_empty() {
        issues.push(issue(
            "packet.validation_issues.unresolved",
            "validation_issues",
            "Only draft work may retain unresolved validation findings",
        ));
    }
}

fn validate_required_evidence(packet: &WorkPacket, issues: &mut Vec<ValidationIssue>) {
    if packet.required_evidence.is_empty() {
        issues.push(issue(
            "packet.required_evidence.empty",
            "required_evidence",
            "At least one evidence class is required",
        ));
    }
    if packet.required_evidence.len() > MAX_LIST_ITEMS {
        issues.push(issue(
            "packet.required_evidence.too_many",
            "required_evidence",
            "Required evidence exceeds the item limit",
        ));
    }
    for (index, kind) in packet.required_evidence.iter().enumerate() {
        if packet.required_evidence[..index].contains(kind) {
            issues.push(issue(
                "packet.required_evidence.duplicate",
                "required_evidence",
                "Required evidence classes must be unique",
            ));
        }
    }
}

fn validate_budgets(packet: &WorkPacket, issues: &mut Vec<ValidationIssue>) {
    if packet.budgets.is_empty() {
        issues.push(issue(
            "packet.budgets.empty",
            "budgets",
            "At least one resource budget is required",
        ));
    }
    if packet.budgets.len() > MAX_LIST_ITEMS {
        issues.push(issue(
            "packet.budgets.too_many",
            "budgets",
            "Resource budgets exceed the item limit",
        ));
    }
    for (index, budget) in packet.budgets.iter().enumerate() {
        if budget.limit == 0 {
            issues.push(issue(
                "packet.budget.zero",
                "budgets",
                "Resource budget limits must be greater than zero",
            ));
        }
        if packet.budgets[..index]
            .iter()
            .any(|prior| prior.resource == budget.resource)
        {
            issues.push(issue(
                "packet.budget.duplicate",
                "budgets",
                "Each resource may have only one budget",
            ));
        }
    }
}

fn validate_stop_conditions(packet: &WorkPacket, issues: &mut Vec<ValidationIssue>) {
    if packet.stop_conditions.is_empty() {
        issues.push(issue(
            "packet.stop_conditions.empty",
            "stop_conditions",
            "At least one stop condition is required",
        ));
    }
    if packet.stop_conditions.len() > MAX_LIST_ITEMS {
        issues.push(issue(
            "packet.stop_conditions.too_many",
            "stop_conditions",
            "Stop conditions exceed the item limit",
        ));
    }
    for (index, condition) in packet.stop_conditions.iter().enumerate() {
        validate_text(
            "stop_conditions.description",
            &condition.description,
            true,
            issues,
        );
        if packet.stop_conditions[..index]
            .iter()
            .any(|prior| prior.kind == condition.kind)
        {
            issues.push(issue(
                "packet.stop_condition.duplicate",
                "stop_conditions",
                "Each stop-condition class may appear only once",
            ));
        }
    }
    for required in REQUIRED_STOP_CONDITIONS {
        if !packet
            .stop_conditions
            .iter()
            .any(|condition| condition.kind == required)
        {
            issues.push(issue(
                "packet.stop_condition.required",
                "stop_conditions",
                "A mandatory safety stop condition is absent",
            ));
        }
    }
}

fn validate_completion_entries(packet: &WorkPacket, issues: &mut Vec<ValidationIssue>) {
    if packet.completion_evidence.len() > MAX_LIST_ITEMS {
        issues.push(issue(
            "packet.completion_evidence.too_many",
            "completion_evidence",
            "Completion-evidence entries exceed the item limit",
        ));
    }
    for (index, entry) in packet.completion_evidence.iter().enumerate() {
        validate_text(
            "completion_evidence.acceptance_check",
            &entry.acceptance_check,
            true,
            issues,
        );
        if !packet.acceptance_checks.contains(&entry.acceptance_check) {
            issues.push(issue(
                "packet.completion_evidence.unknown_check",
                "completion_evidence",
                "Completion evidence names an unknown acceptance check",
            ));
        }
        if packet.completion_evidence[..index]
            .iter()
            .any(|prior| prior.acceptance_check == entry.acceptance_check)
        {
            issues.push(issue(
                "packet.completion_evidence.duplicate_check",
                "completion_evidence",
                "Each acceptance check may have only one completion-evidence entry",
            ));
        }
        validate_evidence_list("completion_evidence.evidence", &entry.evidence, issues);
    }
}

fn validate_retained_issues(packet: &WorkPacket, issues: &mut Vec<ValidationIssue>) {
    if packet.validation_issues.len() > MAX_LIST_ITEMS {
        issues.push(issue(
            "packet.validation_issues.too_many",
            "validation_issues",
            "Retained validation findings exceed the item limit",
        ));
    }
    for retained in &packet.validation_issues {
        validate_identifier("validation_issues.code", &retained.code, issues);
        validate_text("validation_issues.message", &retained.message, true, issues);
        if retained.field_path.is_empty() || retained.field_path.len() > 16 {
            issues.push(issue(
                "packet.validation_issue.path.invalid",
                "validation_issues.field_path",
                "Validation-issue field path is empty or exceeds the segment limit",
            ));
        }
        for segment in &retained.field_path {
            validate_identifier("validation_issues.field_path", segment, issues);
        }
    }
}

fn validate_evidence_list(
    field: &str,
    evidence: &[EvidenceReference],
    issues: &mut Vec<ValidationIssue>,
) {
    if evidence.len() > MAX_EVIDENCE_ITEMS {
        issues.push(issue(
            "packet.evidence.too_many",
            field,
            "Evidence exceeds the item limit",
        ));
    }
    for (index, item) in evidence.iter().enumerate() {
        if item.schema_version != CONTRACT_SCHEMA_VERSION {
            issues.push(issue(
                "packet.evidence.schema_version.unsupported",
                field,
                "Evidence schema version is unsupported",
            ));
        }
        validate_identifier("evidence.evidence_id", item.evidence_id.as_str(), issues);
        validate_text("evidence.source_id", &item.source_id, true, issues);
        validate_text("evidence.object_id", &item.object_id, true, issues);
        if let Some(fragment) = &item.fragment {
            validate_text("evidence.fragment", fragment, true, issues);
        }
        if let Some(revision) = &item.observed_revision {
            validate_text("evidence.observed_revision", revision, true, issues);
        }
        if !is_sha256(&item.content_sha256) {
            issues.push(issue(
                "packet.evidence.hash.invalid",
                field,
                "Evidence content hash must be lowercase SHA-256",
            ));
        }
        if evidence[..index]
            .iter()
            .any(|prior| prior.evidence_id == item.evidence_id)
        {
            issues.push(issue(
                "packet.evidence.duplicate",
                field,
                "Evidence identities must be unique within one list",
            ));
        }
    }
}

fn validate_disjoint_file_declarations(packet: &WorkPacket, issues: &mut Vec<ValidationIssue>) {
    let mutable = packet
        .mutable_files
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if packet
        .protected_files
        .iter()
        .any(|path| mutable.contains(path.as_str()))
    {
        issues.push(issue(
            "packet.files.overlap",
            "protected_files",
            "Mutable and protected file declarations must not overlap",
        ));
    }
}

fn validate_identifier(field: &str, value: &str, issues: &mut Vec<ValidationIssue>) {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        issues.push(issue(
            "packet.identifier.invalid",
            field,
            "Identifier is empty, oversized, or contains unsupported characters",
        ));
    }
}

fn validate_text(field: &str, value: &str, required: bool, issues: &mut Vec<ValidationIssue>) {
    if (required && value.trim().is_empty()) || value.len() > MAX_TEXT_BYTES {
        issues.push(issue(
            "packet.text.invalid",
            field,
            "Required text is empty or text exceeds the byte limit",
        ));
    }
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        issues.push(issue(
            "packet.text.control_character",
            field,
            "Text contains a prohibited control character",
        ));
    }
    if contains_secret_like_value(value) {
        issues.push(issue(
            "packet.text.secret_like",
            field,
            "Text contains a prohibited secret-like pattern",
        ));
    }
}

fn validate_text_list(
    field: &str,
    values: &[String],
    required: bool,
    issues: &mut Vec<ValidationIssue>,
) {
    if required && values.is_empty() {
        issues.push(issue("packet.list.empty", field, "Required list is empty"));
    }
    if values.len() > MAX_LIST_ITEMS {
        issues.push(issue(
            "packet.list.too_many",
            field,
            "List exceeds the item limit",
        ));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_text(field, value, true, issues);
        if !seen.insert(value.as_str()) {
            issues.push(issue(
                "packet.list.duplicate",
                field,
                "List values must be unique",
            ));
        }
    }
}

fn validate_date(field: &str, value: &str, issues: &mut Vec<ValidationIssue>) {
    if !is_iso_date(value) {
        issues.push(issue(
            "packet.date.invalid",
            field,
            "Date must be a valid ISO 8601 calendar date",
        ));
    }
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u16>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let maximum = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=maximum).contains(&day)
}

fn contains_secret_like_value(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    [
        "password=",
        "password:",
        "api_key",
        "api-key",
        "secret=",
        "secret:",
        "token=",
        "token:",
        "-----begin private key-----",
    ]
    .iter()
    .any(|pattern| lowered.contains(pattern))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn issue(code: &str, field: &str, message: &str) -> ValidationIssue {
    ValidationIssue {
        code: code.to_owned(),
        severity: ValidationSeverity::Error,
        field_path: field.split('.').map(str::to_owned).collect(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        WorkPacketHistory, WorkPacketHistoryError, adapt_packet_to_plan, validate_completion,
        validate_packet, validate_transition,
    };
    use agentmage_kernel_contracts::{
        BudgetLimit, BudgetResource, CONTRACT_SCHEMA_VERSION, CompletionEvidence, DataSensitivity,
        EvidenceId, EvidenceKind, EvidenceReference, PlanId, RollbackPlan, StopCondition,
        StopConditionKind, TaskId, ValidationIssue, ValidationSeverity, WorkPacket, WorkPacketId,
        WorkPacketState,
    };

    const STATES: [WorkPacketState; 8] = [
        WorkPacketState::Draft,
        WorkPacketState::Validated,
        WorkPacketState::Planned,
        WorkPacketState::Active,
        WorkPacketState::Blocked,
        WorkPacketState::Completed,
        WorkPacketState::Cancelled,
        WorkPacketState::Superseded,
    ];

    fn evidence(identity: &str, kind: EvidenceKind) -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(identity),
            kind,
            source_id: "synthetic-corpus-v1".to_owned(),
            object_id: identity.to_owned(),
            fragment: None,
            content_sha256: "a".repeat(64),
            observed_revision: Some("fixture-v1".to_owned()),
        }
    }

    fn packet(state: WorkPacketState) -> WorkPacket {
        let plan_id = matches!(
            state,
            WorkPacketState::Planned
                | WorkPacketState::Active
                | WorkPacketState::Blocked
                | WorkPacketState::Completed
        )
        .then(|| PlanId::from_raw("plan-0001"));
        let completion_evidence = if state == WorkPacketState::Completed {
            vec![
                CompletionEvidence {
                    acceptance_check: "Observe fixture".to_owned(),
                    evidence: vec![evidence("evidence-observation", EvidenceKind::Observation)],
                },
                CompletionEvidence {
                    acceptance_check: "Validate result".to_owned(),
                    evidence: vec![evidence("evidence-validation", EvidenceKind::Validation)],
                },
            ]
        } else {
            Vec::new()
        };
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("packet-0001"),
            task_id: TaskId::from_raw("task-0001"),
            revision: 1,
            objective: "Inspect one synthetic fixture".to_owned(),
            reason: "Verify deterministic work-packet behavior".to_owned(),
            owner: "fixture-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: vec!["fixtures/input.txt".to_owned()],
            expected_output: "A bounded observation with validation".to_owned(),
            acceptance_checks: vec!["Observe fixture".to_owned(), "Validate result".to_owned()],
            required_evidence: vec![EvidenceKind::Observation, EvidenceKind::Validation],
            required_capability_class: "read-only".to_owned(),
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 2,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 2,
                },
            ],
            stop_conditions: vec![
                StopCondition {
                    kind: StopConditionKind::AcceptanceSatisfied,
                    description: "Stop when both checks have evidence".to_owned(),
                },
                StopCondition {
                    kind: StopConditionKind::UserDecisionRequired,
                    description: "Stop before any unapproved action".to_owned(),
                },
                StopCondition {
                    kind: StopConditionKind::PolicyDenied,
                    description: "Stop on policy denial".to_owned(),
                },
                StopCondition {
                    kind: StopConditionKind::Error,
                    description: "Stop on a typed error".to_owned(),
                },
                StopCondition {
                    kind: StopConditionKind::Cancelled,
                    description: "Stop when cancellation is requested".to_owned(),
                },
                StopCondition {
                    kind: StopConditionKind::BudgetExhausted,
                    description: "Stop before exceeding a resource budget".to_owned(),
                },
                StopCondition {
                    kind: StopConditionKind::UncertainResult,
                    description: "Stop when an effect cannot be established".to_owned(),
                },
            ],
            rollback: RollbackPlan {
                reversible: true,
                description: "No state change is permitted".to_owned(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-08-10".to_owned(),
            next_action: (state == WorkPacketState::Blocked)
                .then(|| "Request a new user decision".to_owned()),
            next_review: None,
            status_reason: matches!(
                state,
                WorkPacketState::Blocked | WorkPacketState::Cancelled | WorkPacketState::Superseded
            )
            .then(|| "Synthetic state reason".to_owned()),
            disposition: matches!(
                state,
                WorkPacketState::Completed
                    | WorkPacketState::Cancelled
                    | WorkPacketState::Superseded
            )
            .then(|| "Synthetic disposition".to_owned()),
            completion_evidence,
            superseding_work: (state == WorkPacketState::Superseded)
                .then(|| WorkPacketId::from_raw("packet-0002")),
            validation_issues: Vec::new(),
            plan_id,
            state,
        }
    }

    #[test]
    fn every_lifecycle_state_has_one_valid_closed_fixture() {
        for state in STATES {
            assert_eq!(validate_packet(&packet(state)), [], "state: {state:?}");
        }
    }

    #[test]
    fn field_validation_is_bounded_deterministic_and_redacted() {
        let mut candidate = packet(WorkPacketState::Validated);
        candidate.objective.clear();
        candidate.owner = "token=synthetic-secret".to_owned();
        candidate.last_verification_date = "2026-02-30".to_owned();
        candidate.mutable_files = vec!["fixtures/input.txt".to_owned()];
        candidate.budgets[0].limit = 0;
        candidate.validation_issues = vec![ValidationIssue {
            code: "fixture.issue".to_owned(),
            severity: ValidationSeverity::Error,
            field_path: vec!["objective".to_owned()],
            message: "token=synthetic-retained-secret".to_owned(),
        }];
        let first = validate_packet(&candidate);
        let second = validate_packet(&candidate);
        assert_eq!(first, second);
        let codes = first
            .iter()
            .map(|finding| finding.code.as_str())
            .collect::<Vec<_>>();
        for expected in [
            "packet.text.invalid",
            "packet.text.secret_like",
            "packet.files.overlap",
            "packet.budget.zero",
            "packet.date.invalid",
            "packet.validation_issues.unresolved",
        ] {
            assert!(codes.contains(&expected), "missing {expected}");
        }
        let rendered = format!("{first:?}");
        assert!(!rendered.contains("synthetic-secret"));
        assert!(!rendered.contains("synthetic-retained-secret"));
    }

    #[test]
    fn complete_transition_matrix_matches_the_closed_state_machine() {
        let allowed = [
            (WorkPacketState::Draft, WorkPacketState::Draft),
            (WorkPacketState::Draft, WorkPacketState::Validated),
            (WorkPacketState::Draft, WorkPacketState::Cancelled),
            (WorkPacketState::Validated, WorkPacketState::Validated),
            (WorkPacketState::Validated, WorkPacketState::Planned),
            (WorkPacketState::Validated, WorkPacketState::Cancelled),
            (WorkPacketState::Validated, WorkPacketState::Superseded),
            (WorkPacketState::Planned, WorkPacketState::Planned),
            (WorkPacketState::Planned, WorkPacketState::Active),
            (WorkPacketState::Planned, WorkPacketState::Cancelled),
            (WorkPacketState::Planned, WorkPacketState::Superseded),
            (WorkPacketState::Active, WorkPacketState::Active),
            (WorkPacketState::Active, WorkPacketState::Blocked),
            (WorkPacketState::Active, WorkPacketState::Completed),
            (WorkPacketState::Active, WorkPacketState::Cancelled),
            (WorkPacketState::Active, WorkPacketState::Superseded),
            (WorkPacketState::Blocked, WorkPacketState::Blocked),
            (WorkPacketState::Blocked, WorkPacketState::Active),
            (WorkPacketState::Blocked, WorkPacketState::Completed),
            (WorkPacketState::Blocked, WorkPacketState::Cancelled),
            (WorkPacketState::Blocked, WorkPacketState::Superseded),
        ];
        let mut observed = 0;
        for from in STATES {
            for to in STATES {
                let expected = allowed.contains(&(from, to));
                assert_eq!(
                    validate_transition(from, to).is_ok(),
                    expected,
                    "{from:?} -> {to:?}"
                );
                observed += 1;
            }
        }
        assert_eq!(observed, 64);
    }

    #[test]
    fn history_appends_exact_revisions_without_overwrite() {
        let initial = packet(WorkPacketState::Draft);
        let mut history = WorkPacketHistory::new(initial.clone()).expect("valid history");
        let mut validated = packet(WorkPacketState::Validated);
        validated.revision = 2;
        history
            .append_revision(validated.clone())
            .expect("legal next revision");
        assert_eq!(history.revisions(), [initial, validated.clone()]);

        let mut skipped = packet(WorkPacketState::Planned);
        skipped.revision = 4;
        assert_eq!(
            history.append_revision(skipped),
            Err(WorkPacketHistoryError::RevisionOutOfSequence {
                expected: 3,
                actual: 4,
            })
        );
        let mut changed_identity = validated;
        changed_identity.revision = 3;
        changed_identity.work_packet_id = WorkPacketId::from_raw("packet-elsewhere");
        assert_eq!(
            history.append_revision(changed_identity),
            Err(WorkPacketHistoryError::IdentityChanged {
                field: "work_packet_id"
            })
        );
        assert_eq!(history.revisions().len(), 2);
    }

    #[test]
    fn history_rejects_every_revision_failure_without_mutation() {
        let mut wrong_initial = packet(WorkPacketState::Draft);
        wrong_initial.revision = 2;
        assert_eq!(
            WorkPacketHistory::new(wrong_initial),
            Err(WorkPacketHistoryError::InvalidInitialRevision { actual: 2 })
        );

        let mut invalid_initial = packet(WorkPacketState::Draft);
        invalid_initial.objective.clear();
        assert!(matches!(
            WorkPacketHistory::new(invalid_initial),
            Err(WorkPacketHistoryError::InvalidPacket { .. })
        ));

        let initial = packet(WorkPacketState::Draft);
        let mut history = WorkPacketHistory::new(initial.clone()).expect("valid history");

        let mut invalid_candidate = packet(WorkPacketState::Validated);
        invalid_candidate.revision = 2;
        invalid_candidate.objective.clear();
        assert!(matches!(
            history.append_revision(invalid_candidate),
            Err(WorkPacketHistoryError::InvalidPacket { .. })
        ));

        let mut wrong_task = packet(WorkPacketState::Validated);
        wrong_task.revision = 2;
        wrong_task.task_id = TaskId::from_raw("task-elsewhere");
        assert_eq!(
            history.append_revision(wrong_task),
            Err(WorkPacketHistoryError::IdentityChanged { field: "task_id" })
        );

        let mut repeated = packet(WorkPacketState::Validated);
        repeated.revision = 1;
        assert_eq!(
            history.append_revision(repeated),
            Err(WorkPacketHistoryError::RevisionOutOfSequence {
                expected: 2,
                actual: 1,
            })
        );

        let mut illegal = packet(WorkPacketState::Active);
        illegal.revision = 2;
        let Err(WorkPacketHistoryError::IllegalTransition { issue }) =
            history.append_revision(illegal)
        else {
            panic!("draft-to-active transition must fail");
        };
        assert_eq!(issue.code, "packet.transition.illegal");
        assert_eq!(history.revisions(), [initial]);

        let mut exhausted_packet = packet(WorkPacketState::Active);
        exhausted_packet.revision = u32::MAX;
        let mut exhausted = WorkPacketHistory {
            revisions: vec![exhausted_packet],
        };
        let mut candidate = packet(WorkPacketState::Blocked);
        candidate.revision = u32::MAX;
        assert_eq!(
            exhausted.append_revision(candidate),
            Err(WorkPacketHistoryError::RevisionExhausted)
        );
        assert_eq!(exhausted.revisions().len(), 1);
    }

    #[test]
    fn plan_adaptation_is_deterministic_sequential_and_revision_bound() {
        let candidate = packet(WorkPacketState::Validated);
        let plan_id = PlanId::from_raw("plan-0001");
        let first = adapt_packet_to_plan(&candidate, plan_id.clone(), None).expect("first plan");
        let repeated = adapt_packet_to_plan(&candidate, plan_id.clone(), None).expect("same plan");
        assert_eq!(first, repeated);
        assert_eq!(first.work_packet_revision, candidate.revision);
        assert_eq!(first.steps.len(), 2);
        assert!(first.steps[0].depends_on.is_empty());
        assert_eq!(
            first.steps[1].depends_on,
            [first.steps[0].plan_step_id.clone()]
        );

        let mut adapted_packet = candidate;
        adapted_packet.revision = 2;
        adapted_packet
            .acceptance_checks
            .push("Retain receipt".to_owned());
        let mut completed_first = first.clone();
        completed_first.steps[0].state = agentmage_kernel_contracts::PlanStepState::Completed;
        let adapted = adapt_packet_to_plan(&adapted_packet, plan_id, Some(&completed_first))
            .expect("adapted plan");
        assert_eq!(adapted.revision, 2);
        assert_eq!(adapted.work_packet_revision, 2);
        assert_eq!(adapted.steps.len(), 3);
        assert_eq!(
            adapted.steps[0].state,
            agentmage_kernel_contracts::PlanStepState::Completed
        );
        assert_eq!(
            adapted.steps[2].state,
            agentmage_kernel_contracts::PlanStepState::Proposed
        );

        let issues = adapt_packet_to_plan(
            &adapted_packet,
            PlanId::from_raw("plan-0001"),
            Some(&adapted),
        )
        .expect_err("same packet revision must not be relabeled as an adaptation");
        assert!(
            issues
                .iter()
                .any(|finding| finding.code == "packet.plan.packet_revision_stale")
        );
    }

    #[test]
    fn completion_requires_each_check_and_required_evidence_class() {
        let complete = packet(WorkPacketState::Completed);
        assert_eq!(validate_completion(&complete), []);

        let mut missing_check = complete.clone();
        missing_check.completion_evidence.pop();
        let issues = validate_completion(&missing_check);
        assert!(
            issues
                .iter()
                .any(|finding| { finding.code == "packet.completion.acceptance_evidence" })
        );
        assert!(
            issues
                .iter()
                .any(|finding| { finding.code == "packet.completion.required_evidence" })
        );

        let mut wrong_state = complete;
        wrong_state.state = WorkPacketState::Active;
        assert!(
            validate_completion(&wrong_state)
                .iter()
                .any(|finding| finding.code == "packet.completion.state")
        );

        let mut duplicate_claim = packet(WorkPacketState::Completed);
        duplicate_claim
            .completion_evidence
            .push(duplicate_claim.completion_evidence[0].clone());
        let duplicate_issues = validate_packet(&duplicate_claim);
        assert!(
            duplicate_issues
                .iter()
                .any(|finding| finding.code == "packet.completion_evidence.duplicate_check")
        );
        assert!(
            validate_completion(&duplicate_claim)
                .iter()
                .any(|finding| finding.code == "packet.completion.acceptance_evidence")
        );

        let mut unknown_claim = packet(WorkPacketState::Completed);
        unknown_claim.completion_evidence[0].acceptance_check = "Unknown check".to_owned();
        assert!(
            validate_packet(&unknown_claim)
                .iter()
                .any(|finding| finding.code == "packet.completion_evidence.unknown_check")
        );

        let mut empty_claim = packet(WorkPacketState::Completed);
        empty_claim.completion_evidence[0].evidence.clear();
        let empty_issues = validate_completion(&empty_claim);
        assert!(
            empty_issues
                .iter()
                .any(|finding| finding.code == "packet.completion.acceptance_evidence")
        );
        assert!(
            empty_issues
                .iter()
                .any(|finding| finding.code == "packet.completion.required_evidence")
        );
    }
}
