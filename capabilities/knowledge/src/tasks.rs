//! Deterministic task projection, views, duplicates, and evidence-bound transition previews.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::EvidenceReference;
use sha2::{Digest, Sha256};

use crate::{
    KnowledgeError, KnowledgeField, KnowledgeLink, KnowledgeLinkKind, KnowledgeRecord,
    KnowledgeRecordId, KnowledgeRecordKind, validate_record,
};

const MAX_TASKS: usize = 100_000;

/// Closed task lifecycle status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeTaskStatus {
    /// Work is available but has not started.
    Open,
    /// Work is actively in progress.
    InProgress,
    /// Work cannot continue until a visible blocker is resolved.
    Blocked,
    /// Work was explicitly postponed until a visible date or decision.
    Deferred,
    /// Work reached its evidence-backed acceptance boundary.
    Completed,
    /// Work was explicitly cancelled with evidence.
    Cancelled,
}

/// Closed user-visible task priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeTaskPriority {
    /// No urgency beyond ordinary queue ordering.
    Low,
    /// Normal priority.
    Normal,
    /// Important work that should precede normal work.
    High,
    /// Critical work requiring immediate visibility.
    Critical,
}

/// Fully parsed source-backed canonical task projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeTask {
    /// Stable canonical task identity.
    pub task_id: KnowledgeRecordId,
    /// User-visible task title.
    pub title: String,
    /// Exact owner identity or bounded owner label.
    pub owner: String,
    /// Optional project identity or bounded project label.
    pub project: Option<String>,
    /// Current closed status.
    pub status: KnowledgeTaskStatus,
    /// Current priority.
    pub priority: KnowledgeTaskPriority,
    /// Visible blocker required for blocked tasks.
    pub blocker: Option<String>,
    /// Visible next action required for actionable tasks.
    pub next_action: Option<String>,
    /// Optional deferral boundary required for deferred tasks.
    pub deferred_until: Option<String>,
    /// Stable task dependencies drawn from canonical `depends_on` links.
    pub dependencies: Vec<KnowledgeRecordId>,
    /// Other canonical source links supporting task context.
    pub source_links: Vec<KnowledgeLink>,
    /// Content-addressed source and transition evidence.
    pub evidence: Vec<EvidenceReference>,
    /// Digest of the complete canonical source record.
    pub canonical_record_sha256: String,
}

/// Deterministic reason two task records may represent the same work.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KnowledgeTaskDuplicate {
    /// Lexically first task identity.
    pub first_task_id: KnowledgeRecordId,
    /// Lexically second task identity.
    pub second_task_id: KnowledgeRecordId,
    /// Stable duplicate reason code.
    pub reason: String,
}

/// Closed derived task view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeTaskViewKind {
    /// All nonterminal tasks not currently deferred.
    Active,
    /// Tasks whose current state is blocked.
    Blocked,
    /// Tasks with a visible next action.
    NextActions,
    /// Tasks explicitly deferred.
    Deferred,
    /// Terminal completed and cancelled tasks.
    Terminal,
}

/// Deterministic non-authoritative task view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeTaskView {
    /// Selected derived view.
    pub kind: KnowledgeTaskViewKind,
    /// Ordered source-backed tasks.
    pub tasks: Vec<KnowledgeTask>,
    /// Complete duplicate warnings over the input set.
    pub duplicates: Vec<KnowledgeTaskDuplicate>,
    /// Digest of the ordered task and duplicate projection.
    pub view_sha256: String,
    /// Fixed false marker: this derived view is not canonical authority.
    pub canonical: bool,
}

/// Exact no-write proposal for one evidence-backed canonical task transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeTaskTransitionPreview {
    /// Stable canonical task identity.
    pub task_id: KnowledgeRecordId,
    /// Current status observed from canonical Markdown.
    pub from_status: KnowledgeTaskStatus,
    /// Proposed next status.
    pub to_status: KnowledgeTaskStatus,
    /// Digest of the exact current canonical record.
    pub expected_record_sha256: String,
    /// Complete proposed canonical record.
    pub proposed_record: KnowledgeRecord,
    /// Digest of the complete proposed canonical record.
    pub proposed_record_sha256: String,
    /// Evidence identities newly introduced for this transition.
    pub transition_evidence_sha256: Vec<String>,
    /// Digest binding source, proposed state, and evidence.
    pub preview_sha256: String,
    /// Fixed false marker: v0.2 has no task-transition apply operation.
    pub applied: bool,
}

/// Builds one deterministic source-backed task view without changing Markdown.
pub fn build_task_view(
    records: &[KnowledgeRecord],
    kind: KnowledgeTaskViewKind,
) -> Result<KnowledgeTaskView, KnowledgeError> {
    if records.len() > MAX_TASKS {
        return Err(KnowledgeError::InvalidMetadata);
    }
    let mut all_tasks = records
        .iter()
        .filter(|record| record.kind == KnowledgeRecordKind::Task)
        .map(parse_task)
        .collect::<Result<Vec<_>, _>>()?;
    all_tasks.sort_by(task_order);
    let duplicates = detect_task_duplicates(&all_tasks);
    let tasks = all_tasks
        .into_iter()
        .filter(|task| task_in_view(task, kind))
        .collect::<Vec<_>>();
    let view_sha256 = digest_task_view(kind, &tasks, &duplicates)?;
    Ok(KnowledgeTaskView {
        kind,
        tasks,
        duplicates,
        view_sha256,
        canonical: false,
    })
}

/// Constructs an exact evidence-bound transition preview with no apply authority.
pub fn preview_task_transition(
    current: &KnowledgeRecord,
    to_status: KnowledgeTaskStatus,
    transition_evidence: &[EvidenceReference],
    blocker: Option<String>,
    next_action: Option<String>,
    deferred_until: Option<String>,
    updated_at: String,
) -> Result<KnowledgeTaskTransitionPreview, KnowledgeError> {
    let task = parse_task(current)?;
    if transition_evidence.is_empty()
        || transition_evidence.len() > 256
        || !allowed_transition(task.status, to_status)
        || updated_at <= current.updated_at
    {
        return Err(KnowledgeError::InvalidField);
    }
    let current_evidence = current
        .evidence
        .iter()
        .map(|evidence| evidence.content_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let mut transition_hashes = BTreeSet::new();
    for evidence in transition_evidence {
        if !valid_evidence(evidence)
            || current_evidence.contains(evidence.content_sha256.as_str())
            || !transition_hashes.insert(evidence.content_sha256.clone())
        {
            return Err(KnowledgeError::InvalidEvidence);
        }
    }
    if (to_status == KnowledgeTaskStatus::Blocked && blocker.as_deref().is_none_or(str::is_empty))
        || (to_status != KnowledgeTaskStatus::Blocked && blocker.is_some())
        || (to_status == KnowledgeTaskStatus::Deferred
            && deferred_until.as_deref().is_none_or(str::is_empty))
        || (to_status != KnowledgeTaskStatus::Deferred && deferred_until.is_some())
        || matches!(
            to_status,
            KnowledgeTaskStatus::Open | KnowledgeTaskStatus::InProgress
        ) && next_action.as_deref().is_none_or(str::is_empty)
        || matches!(
            to_status,
            KnowledgeTaskStatus::Completed | KnowledgeTaskStatus::Cancelled
        ) && next_action.is_some()
    {
        return Err(KnowledgeError::InvalidField);
    }
    let mut proposed = current.clone();
    set_field(&mut proposed.fields, "status", status_wire(to_status));
    set_optional_field(&mut proposed.fields, "blocker", blocker);
    set_optional_field(&mut proposed.fields, "next_action", next_action);
    set_optional_field(&mut proposed.fields, "deferred_until", deferred_until);
    proposed.updated_at = updated_at;
    proposed.evidence.extend_from_slice(transition_evidence);
    proposed.evidence.sort_by(|left, right| {
        left.content_sha256
            .cmp(&right.content_sha256)
            .then_with(|| left.evidence_id.as_str().cmp(right.evidence_id.as_str()))
    });
    validate_record(&proposed)?;
    let current_json = serde_json::to_vec(current).map_err(|_| KnowledgeError::InvalidMetadata)?;
    let proposed_json =
        serde_json::to_vec(&proposed).map_err(|_| KnowledgeError::InvalidMetadata)?;
    let expected_record_sha256 = sha256(&current_json);
    let proposed_record_sha256 = sha256(&proposed_json);
    let transition_evidence_sha256 = transition_hashes.into_iter().collect::<Vec<_>>();
    let preview_sha256 = sha256(
        &serde_json::to_vec(&(
            "agentmage-knowledge-task-transition-v1",
            task.task_id.as_str(),
            status_wire(task.status),
            status_wire(to_status),
            &expected_record_sha256,
            &proposed_record_sha256,
            &transition_evidence_sha256,
        ))
        .map_err(|_| KnowledgeError::InvalidMetadata)?,
    );
    Ok(KnowledgeTaskTransitionPreview {
        task_id: task.task_id,
        from_status: task.status,
        to_status,
        expected_record_sha256,
        proposed_record: proposed,
        proposed_record_sha256,
        transition_evidence_sha256,
        preview_sha256,
        applied: false,
    })
}

fn parse_task(record: &KnowledgeRecord) -> Result<KnowledgeTask, KnowledgeError> {
    validate_record(record)?;
    if record.kind != KnowledgeRecordKind::Task {
        return Err(KnowledgeError::InvalidField);
    }
    let fields = record
        .fields
        .iter()
        .map(|field| (field.name.as_str(), field.value.as_str()))
        .collect::<BTreeMap<_, _>>();
    let status = parse_status(fields["status"])?;
    let blocker = fields.get("blocker").map(|value| (*value).to_owned());
    let next_action = fields.get("next_action").map(|value| (*value).to_owned());
    let deferred_until = fields
        .get("deferred_until")
        .map(|value| (*value).to_owned());
    if (status == KnowledgeTaskStatus::Blocked) != blocker.is_some()
        || (status == KnowledgeTaskStatus::Deferred) != deferred_until.is_some()
        || matches!(
            status,
            KnowledgeTaskStatus::Open | KnowledgeTaskStatus::InProgress
        ) && next_action.is_none()
        || matches!(
            status,
            KnowledgeTaskStatus::Completed | KnowledgeTaskStatus::Cancelled
        ) && next_action.is_some()
    {
        return Err(KnowledgeError::InvalidField);
    }
    let priority = fields
        .get("priority")
        .map_or(Ok(KnowledgeTaskPriority::Normal), |value| {
            parse_priority(value)
        })?;
    let mut dependencies = Vec::new();
    let mut source_links = Vec::new();
    for link in &record.links {
        if link.kind == KnowledgeLinkKind::DependsOn {
            dependencies.push(link.target_id.clone());
        } else {
            source_links.push(link.clone());
        }
    }
    dependencies.sort();
    source_links.sort();
    Ok(KnowledgeTask {
        task_id: record.record_id.clone(),
        title: record.title.clone(),
        owner: fields["owner"].to_owned(),
        project: fields.get("project").map(|value| (*value).to_owned()),
        status,
        priority,
        blocker,
        next_action,
        deferred_until,
        dependencies,
        source_links,
        evidence: record.evidence.clone(),
        canonical_record_sha256: sha256(
            &serde_json::to_vec(record).map_err(|_| KnowledgeError::InvalidMetadata)?,
        ),
    })
}

fn detect_task_duplicates(tasks: &[KnowledgeTask]) -> Vec<KnowledgeTaskDuplicate> {
    let mut candidates: BTreeMap<(String, String, String), &KnowledgeRecordId> = BTreeMap::new();
    let mut duplicates = Vec::new();
    for task in tasks {
        let key = (
            normalize(&task.title),
            normalize(task.project.as_deref().unwrap_or("")),
            normalize(&task.owner),
        );
        if let Some(first) = candidates.insert(key, &task.task_id) {
            duplicates.push(KnowledgeTaskDuplicate {
                first_task_id: first.clone(),
                second_task_id: task.task_id.clone(),
                reason: "same-title-project-owner".to_owned(),
            });
        }
    }
    duplicates.sort();
    duplicates
}

fn task_in_view(task: &KnowledgeTask, kind: KnowledgeTaskViewKind) -> bool {
    match kind {
        KnowledgeTaskViewKind::Active => matches!(
            task.status,
            KnowledgeTaskStatus::Open
                | KnowledgeTaskStatus::InProgress
                | KnowledgeTaskStatus::Blocked
        ),
        KnowledgeTaskViewKind::Blocked => task.status == KnowledgeTaskStatus::Blocked,
        KnowledgeTaskViewKind::NextActions => task.next_action.is_some(),
        KnowledgeTaskViewKind::Deferred => task.status == KnowledgeTaskStatus::Deferred,
        KnowledgeTaskViewKind::Terminal => matches!(
            task.status,
            KnowledgeTaskStatus::Completed | KnowledgeTaskStatus::Cancelled
        ),
    }
}

fn task_order(left: &KnowledgeTask, right: &KnowledgeTask) -> std::cmp::Ordering {
    right
        .priority
        .cmp(&left.priority)
        .then_with(|| left.task_id.cmp(&right.task_id))
}

fn digest_task_view(
    kind: KnowledgeTaskViewKind,
    tasks: &[KnowledgeTask],
    duplicates: &[KnowledgeTaskDuplicate],
) -> Result<String, KnowledgeError> {
    let projection = tasks
        .iter()
        .map(|task| {
            (
                task.task_id.as_str(),
                &task.canonical_record_sha256,
                status_wire(task.status),
                priority_wire(task.priority),
            )
        })
        .collect::<Vec<_>>();
    let duplicate_projection = duplicates
        .iter()
        .map(|duplicate| {
            (
                duplicate.first_task_id.as_str(),
                duplicate.second_task_id.as_str(),
                duplicate.reason.as_str(),
            )
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&(
        "agentmage-knowledge-task-view-v1",
        view_wire(kind),
        projection,
        duplicate_projection,
    ))
    .map(|bytes| sha256(&bytes))
    .map_err(|_| KnowledgeError::InvalidMetadata)
}

fn allowed_transition(from: KnowledgeTaskStatus, to: KnowledgeTaskStatus) -> bool {
    from != to
        && !matches!(
            from,
            KnowledgeTaskStatus::Completed | KnowledgeTaskStatus::Cancelled
        )
        && match to {
            KnowledgeTaskStatus::Open => matches!(
                from,
                KnowledgeTaskStatus::Blocked | KnowledgeTaskStatus::Deferred
            ),
            KnowledgeTaskStatus::InProgress => matches!(
                from,
                KnowledgeTaskStatus::Open
                    | KnowledgeTaskStatus::Blocked
                    | KnowledgeTaskStatus::Deferred
            ),
            KnowledgeTaskStatus::Blocked
            | KnowledgeTaskStatus::Deferred
            | KnowledgeTaskStatus::Completed
            | KnowledgeTaskStatus::Cancelled => true,
        }
}

fn set_field(fields: &mut Vec<KnowledgeField>, name: &str, value: &str) {
    if let Some(field) = fields.iter_mut().find(|field| field.name == name) {
        field.value = value.to_owned();
    } else {
        fields.push(KnowledgeField {
            name: name.to_owned(),
            value: value.to_owned(),
        });
    }
    fields.sort();
}

fn set_optional_field(fields: &mut Vec<KnowledgeField>, name: &str, value: Option<String>) {
    fields.retain(|field| field.name != name);
    if let Some(value) = value {
        fields.push(KnowledgeField {
            name: name.to_owned(),
            value,
        });
    }
    fields.sort();
}

fn valid_evidence(evidence: &EvidenceReference) -> bool {
    evidence.schema_version == agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        && !evidence.evidence_id.as_str().is_empty()
        && !evidence.source_id.is_empty()
        && !evidence.object_id.is_empty()
        && evidence.content_sha256.len() == 64
        && evidence
            .content_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_status(value: &str) -> Result<KnowledgeTaskStatus, KnowledgeError> {
    match value {
        "open" => Ok(KnowledgeTaskStatus::Open),
        "in_progress" => Ok(KnowledgeTaskStatus::InProgress),
        "blocked" => Ok(KnowledgeTaskStatus::Blocked),
        "deferred" => Ok(KnowledgeTaskStatus::Deferred),
        "completed" => Ok(KnowledgeTaskStatus::Completed),
        "cancelled" => Ok(KnowledgeTaskStatus::Cancelled),
        _ => Err(KnowledgeError::InvalidField),
    }
}

fn parse_priority(value: &str) -> Result<KnowledgeTaskPriority, KnowledgeError> {
    match value {
        "low" => Ok(KnowledgeTaskPriority::Low),
        "normal" => Ok(KnowledgeTaskPriority::Normal),
        "high" => Ok(KnowledgeTaskPriority::High),
        "critical" => Ok(KnowledgeTaskPriority::Critical),
        _ => Err(KnowledgeError::InvalidField),
    }
}

const fn status_wire(status: KnowledgeTaskStatus) -> &'static str {
    match status {
        KnowledgeTaskStatus::Open => "open",
        KnowledgeTaskStatus::InProgress => "in_progress",
        KnowledgeTaskStatus::Blocked => "blocked",
        KnowledgeTaskStatus::Deferred => "deferred",
        KnowledgeTaskStatus::Completed => "completed",
        KnowledgeTaskStatus::Cancelled => "cancelled",
    }
}

const fn priority_wire(priority: KnowledgeTaskPriority) -> &'static str {
    match priority {
        KnowledgeTaskPriority::Low => "low",
        KnowledgeTaskPriority::Normal => "normal",
        KnowledgeTaskPriority::High => "high",
        KnowledgeTaskPriority::Critical => "critical",
    }
}

const fn view_wire(kind: KnowledgeTaskViewKind) -> &'static str {
    match kind {
        KnowledgeTaskViewKind::Active => "active",
        KnowledgeTaskViewKind::Blocked => "blocked",
        KnowledgeTaskViewKind::NextActions => "next_actions",
        KnowledgeTaskViewKind::Deferred => "deferred",
        KnowledgeTaskViewKind::Terminal => "terminal",
    }
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn sha256(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a string cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, DataSensitivity, EvidenceId, EvidenceKind,
    };

    use super::*;
    use crate::{KnowledgePrivacy, KnowledgeRetention, KnowledgeRetentionKind};

    fn evidence(index: u8) -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(format!("evidence-task-{index}")),
            kind: EvidenceKind::Observation,
            source_id: "task-source".to_owned(),
            object_id: format!("object-{index}"),
            fragment: Some("record:1".to_owned()),
            content_sha256: format!("{index:x}").repeat(64),
            observed_revision: Some("revision-1".to_owned()),
        }
    }

    fn task(identity: &str, title: &str, status: &str, priority: &str) -> KnowledgeRecord {
        let mut fields = vec![
            KnowledgeField {
                name: "owner".to_owned(),
                value: "Aaron".to_owned(),
            },
            KnowledgeField {
                name: "project".to_owned(),
                value: "AgentMage".to_owned(),
            },
            KnowledgeField {
                name: "status".to_owned(),
                value: status.to_owned(),
            },
            KnowledgeField {
                name: "priority".to_owned(),
                value: priority.to_owned(),
            },
        ];
        match status {
            "open" | "in_progress" => fields.push(KnowledgeField {
                name: "next_action".to_owned(),
                value: "Run the next bounded test".to_owned(),
            }),
            "blocked" => fields.push(KnowledgeField {
                name: "blocker".to_owned(),
                value: "Independent review".to_owned(),
            }),
            "deferred" => fields.push(KnowledgeField {
                name: "deferred_until".to_owned(),
                value: "2026-09-01T00:00:00Z".to_owned(),
            }),
            _ => {}
        }
        fields.sort();
        KnowledgeRecord {
            schema_version: crate::KNOWLEDGE_SCHEMA_VERSION,
            record_id: KnowledgeRecordId::parse(identity).expect("identity"),
            kind: KnowledgeRecordKind::Task,
            title: title.to_owned(),
            privacy: KnowledgePrivacy::Private,
            sensitivity: DataSensitivity::Durable,
            retention: KnowledgeRetention {
                kind: KnowledgeRetentionKind::UntilSupersededOrDeleted,
                expires_at: None,
            },
            created_at: "2026-08-14T00:00:00Z".to_owned(),
            updated_at: "2026-08-14T00:00:00Z".to_owned(),
            last_verified_at: None,
            fields,
            links: Vec::new(),
            tags: vec!["task".to_owned()],
            evidence: Vec::new(),
        }
    }

    #[test]
    fn views_are_priority_ordered_source_backed_and_duplicate_aware() {
        let low = task("knowledge-task-low", "Review plan", "open", "low");
        let critical = task(
            "knowledge-task-critical",
            " review   plan ",
            "blocked",
            "critical",
        );
        let deferred = task(
            "knowledge-task-deferred",
            "Later work",
            "deferred",
            "normal",
        );
        let active = build_task_view(
            &[low.clone(), deferred.clone(), critical.clone()],
            KnowledgeTaskViewKind::Active,
        )
        .expect("active view");
        assert_eq!(active.tasks.len(), 2);
        assert_eq!(active.tasks[0].task_id, critical.record_id);
        assert_eq!(active.duplicates.len(), 1);
        assert!(!active.canonical);
        let deferred_view =
            build_task_view(&[low, deferred, critical], KnowledgeTaskViewKind::Deferred)
                .expect("deferred view");
        assert_eq!(deferred_view.tasks.len(), 1);
        assert!(deferred_view.tasks[0].deferred_until.is_some());
    }

    #[test]
    fn dependencies_and_source_links_remain_stable_canonical_identities() {
        let mut value = task("knowledge-task-links", "Linked task", "open", "normal");
        value.links = vec![
            KnowledgeLink {
                kind: KnowledgeLinkKind::DependsOn,
                target_id: KnowledgeRecordId::parse("knowledge-task-dependency")
                    .expect("dependency"),
            },
            KnowledgeLink {
                kind: KnowledgeLinkKind::FollowUpTo,
                target_id: KnowledgeRecordId::parse("knowledge-meeting-source").expect("source"),
            },
        ];
        let view = build_task_view(&[value], KnowledgeTaskViewKind::Active).expect("view");
        assert_eq!(view.tasks[0].dependencies.len(), 1);
        assert_eq!(view.tasks[0].source_links.len(), 1);
    }

    #[test]
    fn every_transition_requires_new_evidence_and_produces_no_write() {
        let current = task("knowledge-task-transition", "Transition", "open", "high");
        assert_eq!(
            preview_task_transition(
                &current,
                KnowledgeTaskStatus::Completed,
                &[],
                None,
                None,
                None,
                "2026-08-15T00:00:00Z".to_owned(),
            ),
            Err(KnowledgeError::InvalidField)
        );
        let preview = preview_task_transition(
            &current,
            KnowledgeTaskStatus::Completed,
            &[evidence(1)],
            None,
            None,
            None,
            "2026-08-15T00:00:00Z".to_owned(),
        )
        .expect("transition previews");
        assert_eq!(preview.from_status, KnowledgeTaskStatus::Open);
        assert_eq!(preview.to_status, KnowledgeTaskStatus::Completed);
        assert_eq!(preview.transition_evidence_sha256, vec!["1".repeat(64)]);
        assert!(!preview.applied);
        assert_ne!(
            preview.expected_record_sha256,
            preview.proposed_record_sha256
        );
    }

    #[test]
    fn blocker_deferral_next_action_and_terminal_rules_fail_closed() {
        let current = task("knowledge-task-rules", "Rules", "open", "normal");
        let invalid = [
            preview_task_transition(
                &current,
                KnowledgeTaskStatus::Blocked,
                &[evidence(2)],
                None,
                None,
                None,
                "2026-08-15T00:00:00Z".to_owned(),
            ),
            preview_task_transition(
                &current,
                KnowledgeTaskStatus::Deferred,
                &[evidence(3)],
                None,
                None,
                None,
                "2026-08-15T00:00:00Z".to_owned(),
            ),
            preview_task_transition(
                &current,
                KnowledgeTaskStatus::InProgress,
                &[evidence(4)],
                None,
                None,
                None,
                "2026-08-15T00:00:00Z".to_owned(),
            ),
        ];
        assert!(invalid.iter().all(Result::is_err));
        let terminal = task("knowledge-task-terminal", "Terminal", "completed", "normal");
        assert!(
            preview_task_transition(
                &terminal,
                KnowledgeTaskStatus::Open,
                &[evidence(5)],
                None,
                Some("Reopen".to_owned()),
                None,
                "2026-08-15T00:00:00Z".to_owned(),
            )
            .is_err()
        );
    }
}
