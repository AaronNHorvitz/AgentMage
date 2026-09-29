//! Truthful run progress for any attached client (CAP-39).
//!
//! The projection reads a verified runtime event stream and states only what
//! the runtime knows. That covers whether the run has started, is running, is
//! waiting for an approval, or is stopping after a cancellation request. It
//! also covers which terminal state ended it, and whether the runtime's own
//! verifier accepted that end. Independent review and delivery happen outside
//! the runtime, so the projection never reports them as established. A count
//! carries a denominator and percentage only when the caller supplies a
//! declared ceiling; otherwise its scope is reported as unknown.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    AgentStateKind, RuntimeEvent, RuntimeEventKind, RuntimeSafeNextAction,
};
use serde::Serialize;

use crate::runtime_event::{RuntimeEventError, RuntimeEventSequence};

/// Largest event stream projected at once.
pub const MAX_PROGRESS_EVENTS: usize = 65_536;

/// What the run is doing, as far as the runtime can establish it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunActivity {
    /// No event has been recorded; the run is queued or was never admitted.
    NotStarted,
    /// The run is advancing.
    Running,
    /// At least one operation waits for a person's decision.
    WaitingForApproval,
    /// Cancellation was requested and the run has not yet ended.
    CancellationRequested,
    /// A terminal state ended the run.
    Ended,
}

/// Whether a count's denominator is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProgressScope {
    /// The caller supplied the run's declared ceiling.
    DeclaredCeiling,
    /// No denominator is known; no percentage is shown.
    Unknown,
}

/// One named count and, only when known, its denominator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProgressCount {
    /// Stable count name.
    pub name: &'static str,
    /// Completed units observed in the stream.
    pub observed: u64,
    /// Declared ceiling, when supplied.
    pub denominator: Option<u64>,
    /// Whole percentage of the declared ceiling, when one is known.
    pub percent: Option<u64>,
    /// Whether the denominator is known.
    pub scope: ProgressScope,
}

/// Declared run ceilings supplied by the run owner, when known.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProgressCeilings {
    /// Declared turn ceiling.
    pub turns: Option<u64>,
    /// Declared model-call ceiling.
    pub model_calls: Option<u64>,
    /// Declared tool-call ceiling.
    pub tool_calls: Option<u64>,
}

/// Content-free progress projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RunProgress {
    /// Projection schema version.
    pub schema_version: u16,
    /// Exact run, once one event exists.
    pub run_id: Option<String>,
    /// Current activity.
    pub activity: RunActivity,
    /// Terminal state, once the run ended.
    pub terminal_state: Option<AgentStateKind>,
    /// Whether the runtime's own verifier accepted the run's end.
    pub locally_verified: bool,
    /// The recorded safe next action after a non-success end.
    pub safe_next_action: Option<RuntimeSafeNextAction>,
    /// Operations that still wait for a decision.
    pub pending_approvals: u64,
    /// Always false: independent review is outside the runtime.
    pub independently_reviewed_established: bool,
    /// Always false: delivery is outside the runtime.
    pub delivered_established: bool,
    /// Turn, model-call and tool-call counts.
    pub counts: Vec<ProgressCount>,
    /// Verified events projected.
    pub events: u64,
    /// Digest of the last projected event, or zeroes.
    pub last_event_sha256: String,
}

/// Projects a verified stream. Every event must extend one valid chain.
pub fn project_run_progress(
    events: &[RuntimeEvent],
    ceilings: ProgressCeilings,
) -> Result<RunProgress, RuntimeEventError> {
    if events.len() > MAX_PROGRESS_EVENTS {
        return Err(RuntimeEventError::OrderingMismatch);
    }
    let mut sequence = RuntimeEventSequence::new();
    let mut turns = 0_u64;
    let mut model_calls = 0_u64;
    let mut tool_calls = 0_u64;
    let mut pending = BTreeSet::new();
    let mut cancellation_requested = false;
    let mut terminal_state = None;
    let mut safe_next_action = None;
    for event in events {
        sequence.push(event)?;
        match &event.kind {
            RuntimeEventKind::TurnStarted => turns += 1,
            RuntimeEventKind::ModelRequested { .. } => model_calls += 1,
            RuntimeEventKind::ToolRequested { .. } => tool_calls += 1,
            RuntimeEventKind::PermissionRequested { approval_id, .. } => {
                pending.insert(approval_id.as_str().to_owned());
            }
            RuntimeEventKind::PermissionDecided { approval_id, .. } => {
                pending.remove(approval_id.as_str());
            }
            RuntimeEventKind::CancellationRequested { .. } => cancellation_requested = true,
            // The owner stopped the run's work; no decision is awaited any more.
            RuntimeEventKind::CancellationObserved { .. } => pending.clear(),
            RuntimeEventKind::TerminalDiagnostic {
                safe_next_action: action,
                ..
            } => safe_next_action = Some(*action),
            RuntimeEventKind::RunTerminal { state, .. } => terminal_state = Some(*state),
            _ => {}
        }
    }
    let activity = if events.is_empty() {
        RunActivity::NotStarted
    } else if terminal_state.is_some() {
        RunActivity::Ended
    } else if cancellation_requested {
        // A person who asked to stop is not asked for a decision again.
        RunActivity::CancellationRequested
    } else if !pending.is_empty() {
        RunActivity::WaitingForApproval
    } else {
        RunActivity::Running
    };
    Ok(RunProgress {
        schema_version: 1,
        run_id: events.first().map(|event| event.run_id.as_str().to_owned()),
        activity,
        terminal_state,
        locally_verified: terminal_state.is_some_and(AgentStateKind::is_success),
        safe_next_action,
        pending_approvals: if terminal_state.is_some() || cancellation_requested {
            0
        } else {
            pending.len() as u64
        },
        independently_reviewed_established: false,
        delivered_established: false,
        counts: vec![
            count("turns", turns, ceilings.turns),
            count("model_calls", model_calls, ceilings.model_calls),
            count("tool_calls", tool_calls, ceilings.tool_calls),
        ],
        events: sequence.event_count(),
        last_event_sha256: sequence.last_event_sha256().to_owned(),
    })
}

fn count(name: &'static str, observed: u64, ceiling: Option<u64>) -> ProgressCount {
    let denominator = ceiling.filter(|value| *value > 0);
    ProgressCount {
        name,
        observed,
        denominator,
        percent: denominator.map(|value| observed.saturating_mul(100) / value),
        scope: if denominator.is_some() {
            ProgressScope::DeclaredCeiling
        } else {
            ProgressScope::Unknown
        },
    }
}

/// Bounded text for a person.
#[must_use]
pub fn render_run_progress(progress: &RunProgress) -> String {
    let mut output = String::new();
    let state = match (progress.activity, progress.terminal_state) {
        (RunActivity::NotStarted, _) => "not started; queued or never admitted".to_owned(),
        (RunActivity::Running, _) => "running".to_owned(),
        (RunActivity::WaitingForApproval, _) => format!(
            "waiting for your decision on {} operation(s)",
            progress.pending_approvals
        ),
        (RunActivity::CancellationRequested, _) => {
            "cancellation requested; the run has not ended yet".to_owned()
        }
        (RunActivity::Ended, Some(state)) if state.is_success() => {
            "ended; verified locally by the runtime's verifier".to_owned()
        }
        (RunActivity::Ended, Some(state)) => {
            format!("ended {}; not verified", terminal_label(state))
        }
        (RunActivity::Ended, None) => "ended".to_owned(),
    };
    let _ = write!(output, "run: {state}");
    if let Some(action) = progress.safe_next_action
        && !progress.locally_verified
    {
        let _ = write!(output, "; recorded next step {action:?}");
    }
    output.push('\n');
    for count in &progress.counts {
        let _ = match (count.denominator, count.percent) {
            (Some(denominator), Some(percent)) => writeln!(
                output,
                "{}: {} of {denominator} declared ({percent}%)",
                count.name, count.observed
            ),
            _ => writeln!(output, "{}: {} (total unknown)", count.name, count.observed),
        };
    }
    output.push_str(
        "independent review: not established by the runtime\ndelivery: not established by the runtime\n",
    );
    output
}

const fn terminal_label(state: AgentStateKind) -> &'static str {
    match state {
        AgentStateKind::Blocked => "blocked",
        AgentStateKind::Declined => "declined",
        AgentStateKind::Stalled => "stalled without progress",
        AgentStateKind::Exhausted => "at a declared resource or retry ceiling",
        AgentStateKind::Uncertain => "with an effect that cannot be established",
        AgentStateKind::Cancelled => "cancelled",
        AgentStateKind::Failed => "failed",
        _ => "in a non-terminal state",
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ApprovalId, CONTRACT_SCHEMA_VERSION, CancellationId, ContextSensitivity, CorrelationId,
        EndpointClass, GrantOperation, ModelRunId, PolicyId, RuntimeArtifactId, RuntimeEventId,
        RuntimeEventRetention, RuntimeEventRetentionKind, RuntimeOperationId,
        RuntimePayloadReference, RuntimeRunId, RuntimeTurnId, SessionId, TaskId, ToolCallId,
    };

    use super::*;
    use crate::runtime_event::{runtime_event_persistence, seal_runtime_event};

    const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[derive(Default)]
    struct Stream {
        sequence: u64,
        previous: Option<RuntimeEvent>,
    }

    impl Stream {
        fn push(&mut self, kind: RuntimeEventKind, turn: bool, operation: bool) -> RuntimeEvent {
            let payload_reference = matches!(kind, RuntimeEventKind::TerminalDiagnostic { .. })
                .then(|| RuntimePayloadReference {
                    artifact_id: RuntimeArtifactId::from_raw(format!("payload-{}", self.sequence)),
                    sha256: "a".repeat(64),
                    byte_size: 32,
                    media_type: "application/json".to_owned(),
                });
            let event = seal_runtime_event(RuntimeEvent {
                schema_version: CONTRACT_SCHEMA_VERSION,
                event_id: RuntimeEventId::from_raw(format!("event-{}", self.sequence)),
                run_id: RuntimeRunId::from_raw("run-progress"),
                session_id: SessionId::from_raw("session-progress"),
                task_id: TaskId::from_raw("task-progress"),
                turn_id: turn.then(|| RuntimeTurnId::from_raw("turn-1")),
                operation_id: operation.then(|| RuntimeOperationId::from_raw("operation-1")),
                correlation_id: CorrelationId::from_raw("correlation-progress"),
                causation_event_id: self.previous.as_ref().map(|event| event.event_id.clone()),
                sequence: self.sequence,
                occurred_at_epoch_ms: 1_000 + self.sequence,
                sensitivity: ContextSensitivity::Internal,
                retention: RuntimeEventRetention {
                    kind: RuntimeEventRetentionKind::Ephemeral,
                    expires_at_epoch_ms: None,
                },
                persistence: runtime_event_persistence(&kind),
                policy_id: PolicyId::from_raw("policy-progress"),
                payload_reference,
                kind,
                previous_event_sha256: self
                    .previous
                    .as_ref()
                    .map_or_else(|| ZERO.to_owned(), |event| event.event_sha256.clone()),
                event_sha256: ZERO.to_owned(),
            })
            .unwrap();
            self.sequence += 1;
            self.previous = Some(event.clone());
            event
        }
    }

    fn hash(character: char) -> String {
        character.to_string().repeat(64)
    }

    /// A tool turn that waits for approval, then ends with the given terminal.
    fn run(terminal: Option<AgentStateKind>, cancel: bool) -> Vec<RuntimeEvent> {
        let mut stream = Stream::default();
        let call = ToolCallId::from_raw("call-1");
        let mut events = vec![
            stream.push(
                RuntimeEventKind::RunStarted {
                    request_sha256: hash('1'),
                },
                false,
                false,
            ),
            stream.push(RuntimeEventKind::TurnStarted, true, false),
            stream.push(
                RuntimeEventKind::RouteSelected {
                    route_decision_id: "route-1".to_owned(),
                    endpoint_class: EndpointClass::StrictLocal,
                    decision_sha256: hash('2'),
                },
                true,
                false,
            ),
            stream.push(
                RuntimeEventKind::ModelRequested {
                    model_run_id: ModelRunId::from_raw("model-1"),
                    request_sha256: hash('3'),
                },
                true,
                false,
            ),
            stream.push(
                RuntimeEventKind::ModelCompleted {
                    model_run_id: ModelRunId::from_raw("model-1"),
                    result_sha256: hash('4'),
                },
                true,
                false,
            ),
            stream.push(
                RuntimeEventKind::ProposalObserved {
                    proposal_id: "proposal-1".to_owned(),
                    proposal_sha256: hash('5'),
                },
                true,
                false,
            ),
            stream.push(
                RuntimeEventKind::ToolRequested {
                    tool_call_id: call,
                    arguments_sha256: hash('6'),
                },
                true,
                true,
            ),
            stream.push(
                RuntimeEventKind::PermissionRequested {
                    approval_id: ApprovalId::from_raw("approval-1"),
                    operation: GrantOperation::WorkspaceWrite,
                    preview_sha256: hash('7'),
                    expires_at_epoch_ms: 90_000,
                },
                true,
                true,
            ),
        ];
        let Some(state) = terminal else {
            return events;
        };
        events.push(stream.push(
            RuntimeEventKind::PermissionDecided {
                approval_id: ApprovalId::from_raw("approval-1"),
                disposition: agentmage_kernel_contracts::RuntimePermissionDisposition::Deny,
                grant_id: None,
                decision_sha256: hash('8'),
            },
            true,
            true,
        ));
        events.push(stream.push(
            RuntimeEventKind::TurnCompleted {
                outcome_sha256: hash('9'),
            },
            true,
            false,
        ));
        if cancel {
            events.push(stream.push(
                RuntimeEventKind::CancellationRequested {
                    cancellation_id: CancellationId::from_raw("cancel-1"),
                },
                false,
                false,
            ));
            return events;
        }
        if !state.is_success() {
            events.push(stream.push(
                RuntimeEventKind::TerminalDiagnostic {
                    diagnostic_id: "diagnostic-1".to_owned(),
                    diagnostic_sha256: hash('b'),
                    safe_next_action: RuntimeSafeNextAction::InspectEvidence,
                },
                false,
                false,
            ));
        }
        events.push(stream.push(
            RuntimeEventKind::RunTerminal {
                state,
                outcome_sha256: hash('c'),
            },
            false,
            false,
        ));
        events
    }

    #[test]
    fn activity_follows_the_verified_stream_and_never_claims_outside_states() {
        let empty = project_run_progress(&[], ProgressCeilings::default()).unwrap();
        assert_eq!(empty.activity, RunActivity::NotStarted);
        assert!(render_run_progress(&empty).contains("not started"));

        let waiting = project_run_progress(&run(None, false), ProgressCeilings::default()).unwrap();
        assert_eq!(waiting.activity, RunActivity::WaitingForApproval);
        assert_eq!(waiting.pending_approvals, 1);
        let text = render_run_progress(&waiting);
        assert!(
            text.contains("waiting for your decision on 1 operation(s)"),
            "{text}"
        );
        assert!(text.contains("turns: 1 (total unknown)"));
        assert!(!text.contains('%'));

        let stopping = project_run_progress(
            &run(Some(AgentStateKind::Cancelled), true),
            ProgressCeilings::default(),
        )
        .unwrap();
        assert_eq!(stopping.activity, RunActivity::CancellationRequested);
        assert_eq!(stopping.pending_approvals, 0);

        let failed = project_run_progress(
            &run(Some(AgentStateKind::Failed), false),
            ProgressCeilings::default(),
        )
        .unwrap();
        assert_eq!(failed.activity, RunActivity::Ended);
        assert!(!failed.locally_verified);
        assert_eq!(
            failed.safe_next_action,
            Some(RuntimeSafeNextAction::InspectEvidence)
        );
        let text = render_run_progress(&failed);
        assert!(text.contains("ended failed; not verified"), "{text}");
        for progress in [&empty, &waiting, &stopping, &failed] {
            assert!(!progress.independently_reviewed_established);
            assert!(!progress.delivered_established);
            assert!(render_run_progress(progress).contains("independent review: not established"));
        }
    }

    #[test]
    fn only_a_verified_success_is_local_verification_and_percentages_need_a_ceiling() {
        let events = run(Some(AgentStateKind::Success), false);
        let ceilings = ProgressCeilings {
            turns: Some(4),
            model_calls: Some(0),
            tool_calls: None,
        };
        let progress = project_run_progress(&events, ceilings).unwrap();
        assert!(progress.locally_verified);
        assert_eq!(progress.terminal_state, Some(AgentStateKind::Success));
        assert_eq!(progress.counts[0].percent, Some(25));
        assert_eq!(progress.counts[0].scope, ProgressScope::DeclaredCeiling);
        // A zero or absent ceiling is an unknown scope, never a percentage.
        assert_eq!(progress.counts[1].scope, ProgressScope::Unknown);
        assert_eq!(progress.counts[1].percent, None);
        assert_eq!(progress.counts[2].percent, None);
        let text = render_run_progress(&progress);
        assert!(text.contains("verified locally by the runtime's verifier"));
        assert!(text.contains("turns: 1 of 4 declared (25%)"));
        assert!(text.contains("tool_calls: 1 (total unknown)"));
        assert_eq!(progress.events, events.len() as u64);
        assert_eq!(
            progress.last_event_sha256,
            events.last().unwrap().event_sha256
        );
    }

    /// Continues a stream after the given events.
    fn after(events: &[RuntimeEvent]) -> Stream {
        Stream {
            sequence: events.len() as u64,
            previous: events.last().cloned(),
        }
    }

    #[test]
    fn a_cancellation_request_ends_the_wait_for_a_decision() {
        let mut events = run(None, false);
        let mut stream = after(&events);
        let cancellation_id = CancellationId::from_raw("cancel-waiting");
        events.push(stream.push(
            RuntimeEventKind::CancellationRequested {
                cancellation_id: cancellation_id.clone(),
            },
            false,
            false,
        ));
        let requested = project_run_progress(&events, ProgressCeilings::default()).unwrap();
        events.push(stream.push(
            RuntimeEventKind::CancellationObserved { cancellation_id },
            false,
            false,
        ));
        let observed = project_run_progress(&events, ProgressCeilings::default()).unwrap();
        for progress in [&requested, &observed] {
            assert_eq!(progress.activity, RunActivity::CancellationRequested);
            assert_eq!(progress.pending_approvals, 0);
            let text = render_run_progress(progress);
            assert!(
                text.contains("cancellation requested; the run has not ended yet"),
                "{text}"
            );
            assert!(!text.contains("waiting for your decision"), "{text}");
        }
        events.push(stream.push(
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Cancelled,
                outcome_sha256: hash('d'),
            },
            false,
            false,
        ));
        let ended = project_run_progress(&events, ProgressCeilings::default()).unwrap();
        assert_eq!(ended.activity, RunActivity::Ended);
        assert_eq!(ended.terminal_state, Some(AgentStateKind::Cancelled));
        assert!(!ended.locally_verified);
        assert!(render_run_progress(&ended).contains("run: ended cancelled; not verified"));
    }

    #[test]
    fn running_and_every_terminal_state_are_reported_as_the_stream_establishes_them() {
        // A turn whose model call is in flight, with no decision pending.
        let running =
            project_run_progress(&run(None, false)[..4], ProgressCeilings::default()).unwrap();
        assert_eq!(running.activity, RunActivity::Running);
        assert_eq!(running.pending_approvals, 0);
        assert!(render_run_progress(&running).starts_with("run: running\n"));
        for (state, expected, verified) in [
            (
                AgentStateKind::Success,
                "run: ended; verified locally by the runtime's verifier\n",
                true,
            ),
            (
                AgentStateKind::NoOp,
                "run: ended; verified locally by the runtime's verifier\n",
                true,
            ),
            (
                AgentStateKind::Blocked,
                "run: ended blocked; not verified; recorded next step InspectEvidence\n",
                false,
            ),
            (
                AgentStateKind::Declined,
                "run: ended declined; not verified; recorded next step InspectEvidence\n",
                false,
            ),
            (
                AgentStateKind::Stalled,
                "run: ended stalled without progress; not verified; recorded next step InspectEvidence\n",
                false,
            ),
            (
                AgentStateKind::Exhausted,
                "run: ended at a declared resource or retry ceiling; not verified; recorded next step InspectEvidence\n",
                false,
            ),
            (
                AgentStateKind::Uncertain,
                "run: ended with an effect that cannot be established; not verified; recorded next step InspectEvidence\n",
                false,
            ),
            (
                AgentStateKind::Failed,
                "run: ended failed; not verified; recorded next step InspectEvidence\n",
                false,
            ),
        ] {
            let progress =
                project_run_progress(&run(Some(state), false), ProgressCeilings::default())
                    .unwrap();
            assert_eq!(progress.activity, RunActivity::Ended);
            assert_eq!(progress.terminal_state, Some(state));
            assert_eq!(progress.locally_verified, verified, "{state:?}");
            assert_eq!(progress.pending_approvals, 0);
            let text = render_run_progress(&progress);
            assert!(text.starts_with(expected), "{state:?}: {text}");
            assert!(text.ends_with(
                "independent review: not established by the runtime\ndelivery: not established by the runtime\n"
            ));
        }
    }

    #[test]
    fn a_tampered_or_reordered_stream_is_refused() {
        let events = run(Some(AgentStateKind::Failed), false);
        let mut tampered = events.clone();
        tampered[3].sequence += 1;
        let mut reordered = events.clone();
        reordered.swap(2, 3);
        let mut after_terminal = events.clone();
        after_terminal.push(events[1].clone());
        for stream in [tampered, reordered, after_terminal, events[1..].to_vec()] {
            assert!(project_run_progress(&stream, ProgressCeilings::default()).is_err());
        }
    }
}
