//! The standalone application's bridge to its evidence host (Decision 0150).
//!
//! The bridge launches the sibling host under the standalone evidence
//! activation, authenticates it, and turns the host's verified answers into
//! closed presentation events. It owns no folder, model, tool, grant or
//! completion authority: the host reads the folder and decides every entry,
//! and every question runs through the same verified driver as the
//! development CLI. The window, keyboard and screen-reader presentation is not
//! part of this increment; any presentation speaks only these messages.

use std::cell::RefCell;
use std::path::Path;

use agentmage_kernel_contracts::{
    AgentStateKind, CancellationId, RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEvent,
    RuntimeEventCursor, RuntimeOutput, RuntimeRunId, RuntimeRunRequest, SessionId,
};
use agentmage_kernel_engine::runtime_artifact::{RuntimeArtifactPage, RuntimeArtifactState};
use agentmage_platform_linux::LinuxDevelopmentHostProcess;
use serde::{Deserialize, Serialize};

use crate::cli_runtime::{
    InteractiveCliCancellationPort, InteractiveCliRuntimeError, InteractiveCliRuntimeResult,
    drive_interactive_cli_runtime,
};
use crate::coding_client::{CodingClientError, CodingEventSink, DenyHeadlessApproval};
use crate::runtime_ipc::LinuxRuntimeIpcClient;
use crate::runtime_transport::{
    RuntimePrepareInput, RuntimeRunDeclarations, RuntimeTransportError, RuntimeTransportPort,
    RuntimeTransportStep,
};
use crate::standalone_evidence::{
    EvidenceAnswer, STANDALONE_EVIDENCE_ANSWER_MEDIA_TYPE, STANDALONE_EVIDENCE_PROFILE_ID,
};
use crate::standalone_folder::{
    FolderAdmissionView, FolderAnswer, FolderEntryDisposition, FolderRequest,
};

/// Longest question the bridge sends; the host enforces the same bound.
pub const MAX_SHELL_QUESTION_BYTES: usize = 2 * 1024;
/// How the presentation labels every answer: the fixture is not a model.
pub const FIXTURE_ANSWER_LABEL: &str =
    "Inferred by standalone-evidence-fixture-v1, a term-match fixture, not a language model";

/// The bridge's state as a presentation shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShellState {
    /// The host is being launched and authenticated.
    Starting,
    /// The host is authenticated; folder and question requests are accepted.
    Ready,
    /// The host could not start; only a restart or shutdown is accepted.
    Unavailable {
        /// One stable code.
        code: String,
    },
    /// The host failed or exited while in use; only a restart or shutdown
    /// is accepted, and the earlier folder must be admitted again.
    SafeMode {
        /// One stable code.
        code: String,
    },
}

/// One closed request from a presentation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShellRequest {
    /// Ask the host to admit one folder.
    AdmitFolder {
        /// Strictly increasing request sequence.
        sequence: u64,
        /// Absolute folder path.
        folder: String,
    },
    /// Ask one question about the admitted folder.
    Ask {
        /// Strictly increasing request sequence.
        sequence: u64,
        /// The question.
        question: String,
    },
    /// Cancel the question in progress.
    Cancel {
        /// Strictly increasing request sequence.
        sequence: u64,
    },
    /// Replace a failed or unavailable host with a new one.
    Restart {
        /// Strictly increasing request sequence.
        sequence: u64,
    },
    /// Stop the host and end the bridge.
    Shutdown {
        /// Strictly increasing request sequence.
        sequence: u64,
    },
}

impl ShellRequest {
    /// The request's sequence.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        match self {
            Self::AdmitFolder { sequence, .. }
            | Self::Ask { sequence, .. }
            | Self::Cancel { sequence }
            | Self::Restart { sequence }
            | Self::Shutdown { sequence } => *sequence,
        }
    }
}

/// One closed event to a presentation. It describes and grants nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShellEvent {
    /// The bridge state changed.
    State {
        /// The new state.
        state: ShellState,
    },
    /// A folder was admitted; it is the current snapshot.
    Folder {
        /// The request it answers.
        sequence: u64,
        /// What the host read and decided.
        admission: FolderAdmissionView,
    },
    /// A folder was not admitted; the earlier snapshot, if any, stays.
    FolderRefused {
        /// The request it answers.
        sequence: u64,
        /// One stable code.
        code: String,
    },
    /// A question began.
    QuestionStarted {
        /// The request it answers.
        sequence: u64,
        /// The bridge's identity for the question.
        question_id: String,
    },
    /// One verified runtime event of the question, by kind only.
    Progress {
        /// The question.
        question_id: String,
        /// The event's sequence in its run.
        event_sequence: u64,
        /// The event kind's name.
        event: String,
    },
    /// The question ended.
    Answer {
        /// The request it answers.
        sequence: u64,
        /// What the presentation shows.
        answer: AnswerView,
    },
    /// A request was refused before reaching the host.
    Refused {
        /// The refused request's sequence, when it had one.
        sequence: Option<u64>,
        /// One stable code.
        code: String,
    },
}

/// How a question ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerState {
    /// The host's verifier admitted the cited statements.
    Verified,
    /// The run ended without verification; nothing is shown as evidence.
    Unverified,
    /// The person cancelled the question.
    Cancelled,
    /// The run failed.
    Failed,
}

/// One statement and the cards it cites.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementView {
    /// Statement text.
    pub text: String,
    /// The evidence cards that support it.
    pub card_ids: Vec<String>,
}

/// One evidence card: an exact quoted range of one admitted file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCardView {
    /// Card identity within the answer.
    pub card_id: String,
    /// The file, relative to the admitted folder.
    pub path: String,
    /// Prepared source identity.
    pub source_id: String,
    /// First cited line, counting from one.
    pub first_line: u64,
    /// Last cited line.
    pub last_line: u64,
    /// Exact quoted text.
    pub quote: String,
    /// Digest of the admitted file the quote comes from.
    pub source_sha256: String,
    /// The run's evidence identity.
    pub evidence_id: String,
}

/// What a presentation shows for one ended question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerView {
    /// How the question ended.
    pub state: AnswerState,
    /// How the answer was produced.
    pub label: String,
    /// Verified statements; empty unless verified.
    pub statements: Vec<StatementView>,
    /// Their evidence cards.
    pub cards: Vec<EvidenceCardView>,
    /// Whether the fixture said no admitted source had matching text. This is
    /// its unverified statement, never a verified absence.
    pub no_matching_text: bool,
    /// The run's stable codes.
    pub unresolved_codes: Vec<String>,
    /// Admitted files left out of the model context at any turn.
    pub context_omissions: Option<u32>,
    /// The run.
    pub run_id: String,
    /// The run's outcome digest.
    pub outcome_sha256: String,
}

/// A presentation: it shows events and says whether a cancel was asked.
pub trait ShellPresenter {
    /// Shows one event.
    fn present(&mut self, event: &ShellEvent);

    /// Whether the person asked to cancel the question in progress since the
    /// last poll; each request is reported once. The bridge polls this
    /// between host boundaries.
    fn cancel_requested(&mut self) -> bool;
}

/// A runtime port that remembers the first refusal it returned, so the
/// bridge can name why a session broke.
struct Observed<'a, P: RuntimeTransportPort> {
    inner: &'a mut P,
    first_error: Option<RuntimeTransportError>,
}

impl<P: RuntimeTransportPort> Observed<'_, P> {
    fn note<T>(
        &mut self,
        result: Result<T, RuntimeTransportError>,
    ) -> Result<T, RuntimeTransportError> {
        if let Err(error) = &result
            && self.first_error.is_none()
        {
            self.first_error = Some(*error);
        }
        result
    }
}

impl<P: RuntimeTransportPort> RuntimeTransportPort for Observed<'_, P> {
    fn prepare(
        &mut self,
        input: RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        let result = self.inner.prepare(input);
        self.note(result)
    }

    fn start(
        &mut self,
        request: RuntimeRunRequest,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        let result = self.inner.start(request);
        self.note(result)
    }

    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        let result = self
            .inner
            .advance(run_id, request_sha256, after_event_cursor, response);
        self.note(result)
    }

    fn cancel(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        let result = self
            .inner
            .cancel(run_id, request_sha256, cancellation_id, after_event_cursor);
        self.note(result)
    }

    fn read_artifact_page(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
        offset: u64,
        maximum_bytes: u32,
    ) -> Result<RuntimeArtifactPage, RuntimeTransportError> {
        let result =
            self.inner
                .read_artifact_page(run_id, request_sha256, reference, offset, maximum_bytes);
        self.note(result)
    }

    fn release_artifact(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
    ) -> Result<RuntimeArtifactState, RuntimeTransportError> {
        let result = self
            .inner
            .release_artifact(run_id, request_sha256, reference);
        self.note(result)
    }

    fn run_declarations(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<RuntimeRunDeclarations, RuntimeTransportError> {
        // Declarations are optional; their absence never breaks a session.
        self.inner.run_declarations(run_id, request_sha256)
    }

    fn job_status(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<crate::runtime_transport::RuntimeJobStatus, RuntimeTransportError> {
        // A status read is optional evidence; its refusal never names a
        // broken session.
        self.inner.job_status(run_id, request_sha256)
    }

    fn control_job(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        request: &agentmage_kernel_engine::job_control::JobControlRequest,
    ) -> Result<crate::runtime_transport::RuntimeJobControl, RuntimeTransportError> {
        // Each evidence run is one job; a cancellation goes through its
        // ledger under this client's scope (Decision 0120).
        let result = self.inner.control_job(run_id, request_sha256, request);
        self.note(result)
    }

    fn release(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<(), RuntimeTransportError> {
        let result = self.inner.release(run_id, request_sha256);
        self.note(result)
    }
}

struct Progress<'a> {
    presenter: &'a RefCell<&'a mut dyn ShellPresenter>,
    question_id: &'a str,
}

impl CodingEventSink for Progress<'_> {
    fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError> {
        let name = serde_json::to_value(&event.kind)
            .ok()
            .and_then(|value| {
                value
                    .get("event")
                    .and_then(|name| name.as_str().map(str::to_owned))
            })
            .ok_or(CodingClientError::Presentation)?;
        self.presenter
            .try_borrow_mut()
            .map_err(|_| CodingClientError::Presentation)?
            .present(&ShellEvent::Progress {
                question_id: self.question_id.to_owned(),
                event_sequence: event.sequence,
                event: name,
            });
        Ok(())
    }
}

struct Cancel<'a> {
    presenter: &'a RefCell<&'a mut dyn ShellPresenter>,
    question_id: &'a str,
    requests: u64,
}

impl InteractiveCliCancellationPort for Cancel<'_> {
    /// Each request the person makes is sent once, under its own identity,
    /// as the development CLI sends each signal; a later poll sees only a
    /// newer request.
    fn poll(
        &mut self,
        _request: &RuntimeRunRequest,
    ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError> {
        let requested = self
            .presenter
            .try_borrow_mut()
            .map_err(|_| InteractiveCliRuntimeError::Cancellation)?
            .cancel_requested();
        if !requested {
            return Ok(None);
        }
        self.requests = self
            .requests
            .checked_add(1)
            .ok_or(InteractiveCliRuntimeError::Cancellation)?;
        Ok(Some(CancellationId::from_raw(format!(
            "{}-cancel-{}",
            self.question_id, self.requests
        ))))
    }
}

/// Why a question could not end with an answer: the session is broken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellFailure {
    /// One stable code.
    pub code: String,
}

/// The bridge over one authenticated runtime port.
pub struct StandaloneShell<P: RuntimeTransportPort> {
    runtime: P,
    admission: Option<FolderAdmissionView>,
    session_id: Option<SessionId>,
    questions: u64,
}

impl<P: RuntimeTransportPort> StandaloneShell<P> {
    /// A bridge over an already authenticated port.
    pub const fn over(runtime: P) -> Self {
        Self {
            runtime,
            admission: None,
            session_id: None,
            questions: 0,
        }
    }

    /// The current admitted snapshot, if any.
    #[must_use]
    pub const fn admission(&self) -> Option<&FolderAdmissionView> {
        self.admission.as_ref()
    }

    /// The port, for the owner that shuts the host down.
    pub const fn runtime_mut(&mut self) -> &mut P {
        &mut self.runtime
    }

    /// Asks the host to admit `folder`. A refusal leaves the earlier snapshot.
    pub fn admit_folder(
        &mut self,
        sequence: u64,
        folder: &str,
    ) -> Result<ShellEvent, ShellFailure> {
        let answer = self
            .runtime
            .folder(FolderRequest::Admit {
                folder: folder.to_owned(),
            })
            .map_err(|error| ShellFailure {
                code: error.code().to_owned(),
            })?;
        Ok(match answer {
            FolderAnswer::Admitted { admission } => {
                if admission.verify() && admission.folder == folder {
                    self.admission = Some(admission.clone());
                    ShellEvent::Folder {
                        sequence,
                        admission,
                    }
                } else {
                    ShellEvent::FolderRefused {
                        sequence,
                        code: "standalone.shell.admission-unverified".to_owned(),
                    }
                }
            }
            FolderAnswer::Refused { refusal } => ShellEvent::FolderRefused {
                sequence,
                code: refusal.code().to_owned(),
            },
        })
    }

    /// Asks one question about the admitted snapshot and presents its progress.
    pub fn ask(
        &mut self,
        sequence: u64,
        question: &str,
        presenter: &mut dyn ShellPresenter,
    ) -> Result<ShellEvent, ShellFailure> {
        let refused = |code: &str| {
            Ok(ShellEvent::Refused {
                sequence: Some(sequence),
                code: code.to_owned(),
            })
        };
        let Some(admission) = self.admission.clone() else {
            return refused("standalone.shell.no-folder");
        };
        if question.trim().is_empty() || question.len() > MAX_SHELL_QUESTION_BYTES {
            return refused("standalone.shell.question-invalid");
        }
        if admission.accepted == 0 {
            return refused("standalone.shell.no-accepted-text");
        }
        self.questions += 1;
        let question_id = format!("question-{}", self.questions);
        presenter.present(&ShellEvent::QuestionStarted {
            sequence,
            question_id: question_id.clone(),
        });
        let input = RuntimePrepareInput {
            resume: false,
            record_session: false,
            slow_subscriber_probe: false,
            preauthorization: None,
            engineering_session_id: self.session_id.clone(),
            profile_id: STANDALONE_EVIDENCE_PROFILE_ID.to_owned(),
            expected_entry_sha256: admission.admission_sha256.clone(),
            workspace_id: admission.workspace_id.clone(),
            workspace_root: admission.folder.clone(),
            prompt: question.to_owned(),
            recipe: None,
        };
        let shared: RefCell<&mut dyn ShellPresenter> = RefCell::new(presenter);
        let mut observed = Observed {
            inner: &mut self.runtime,
            first_error: None,
        };
        let result = drive_interactive_cli_runtime(
            &mut observed,
            input,
            &mut DenyHeadlessApproval,
            &mut Progress {
                presenter: &shared,
                question_id: &question_id,
            },
            &mut Cancel {
                presenter: &shared,
                question_id: &question_id,
                requests: 0,
            },
        );
        match result {
            Ok(result) => {
                self.session_id = Some(result.request.session_id.clone());
                Ok(ShellEvent::Answer {
                    sequence,
                    answer: project_answer(&result, &admission),
                })
            }
            Err(error) => Err(ShellFailure {
                code: observed
                    .first_error
                    .map_or(error.code(), RuntimeTransportError::code)
                    .to_owned(),
            }),
        }
    }
}

/// Projects one verified run result into what a presentation shows. Only a
/// verified run's statements become evidence cards; everything else is shown
/// as not verified.
#[must_use]
pub fn project_answer(
    result: &InteractiveCliRuntimeResult,
    admission: &FolderAdmissionView,
) -> AnswerView {
    let outcome = &result.outcome;
    let answer = match &outcome.output {
        Some(RuntimeOutput::Inline { payload })
            if payload.media_type == STANDALONE_EVIDENCE_ANSWER_MEDIA_TYPE =>
        {
            serde_json::from_slice::<EvidenceAnswer>(&payload.bytes).ok()
        }
        _ => None,
    };
    let context_omissions = result
        .declarations
        .as_ref()
        .and_then(|declarations| declarations.context_inspections.as_ref())
        .map(|views| {
            let omitted: std::collections::BTreeSet<&str> = views
                .iter()
                .flat_map(|view| view.omitted.iter())
                .map(|item| item.source_id.as_str())
                .collect();
            u32::try_from(omitted.len()).unwrap_or(u32::MAX)
        });
    let mut view = AnswerView {
        state: match outcome.state {
            AgentStateKind::Success => AnswerState::Verified,
            AgentStateKind::Cancelled => AnswerState::Cancelled,
            AgentStateKind::Failed if answer.is_some() => AnswerState::Unverified,
            _ => AnswerState::Failed,
        },
        label: FIXTURE_ANSWER_LABEL.to_owned(),
        statements: Vec::new(),
        cards: Vec::new(),
        no_matching_text: answer
            .as_ref()
            .is_some_and(|answer| answer.no_matching_text),
        unresolved_codes: outcome.unresolved_codes.clone(),
        context_omissions,
        run_id: outcome.run_id.as_str().to_owned(),
        outcome_sha256: outcome.outcome_sha256.clone(),
    };
    let (AnswerState::Verified, Some(answer)) = (view.state, answer) else {
        return view;
    };
    // A verified run's citations must still name this run's evidence and an
    // accepted file of this snapshot; otherwise nothing is shown as evidence.
    let mut cards = Vec::new();
    let mut statements = Vec::new();
    for statement in &answer.statements {
        let mut card_ids = Vec::new();
        for citation in &statement.citations {
            let entry = admission.entries.iter().find(|entry| {
                entry.disposition == FolderEntryDisposition::Accepted
                    && entry.source_id.as_deref() == Some(citation.source_id.as_str())
            });
            let observed = outcome
                .evidence
                .iter()
                .any(|evidence| evidence.evidence_id.as_str() == citation.evidence_id);
            let (Some(entry), true, Some(quote)) = (entry, observed, statement.quote.as_ref())
            else {
                view.state = AnswerState::Unverified;
                view.unresolved_codes
                    .push("standalone.shell.citation-unmatched".to_owned());
                return view;
            };
            let card_id = format!("card-{}", cards.len() + 1);
            card_ids.push(card_id.clone());
            cards.push(EvidenceCardView {
                card_id,
                path: entry.path.clone(),
                source_id: citation.source_id.clone(),
                first_line: citation.start_line,
                last_line: citation
                    .end_line_exclusive
                    .saturating_sub(1)
                    .max(citation.start_line),
                quote: quote.clone(),
                source_sha256: entry.content_sha256.clone().unwrap_or_default(),
                evidence_id: citation.evidence_id.clone(),
            });
        }
        statements.push(StatementView {
            text: statement.text.clone(),
            card_ids,
        });
    }
    view.statements = statements;
    view.cards = cards;
    view
}

/// Longest request line a presentation may send.
pub const MAX_SHELL_REQUEST_BYTES: usize = 16 * 1024;

/// A request refused by the gate before it reached the bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShellRefusal {
    /// The refused request's sequence, when it had one.
    pub sequence: Option<u64>,
    /// One stable code.
    pub code: &'static str,
}

impl ShellRefusal {
    /// The event a presentation shows for it.
    #[must_use]
    pub fn event(self) -> ShellEvent {
        ShellEvent::Refused {
            sequence: self.sequence,
            code: self.code.to_owned(),
        }
    }
}

/// The receiving side of a presentation's requests: each must decode exactly
/// into one closed request, and sequences must strictly increase.
#[derive(Debug, Default)]
pub struct ShellRequestGate {
    last_sequence: u64,
}

impl ShellRequestGate {
    /// Admits one request line, or returns its refusal.
    pub fn accept(&mut self, line: &[u8]) -> Result<ShellRequest, ShellRefusal> {
        if line.len() > MAX_SHELL_REQUEST_BYTES {
            return Err(ShellRefusal {
                sequence: None,
                code: "standalone.shell.request-too-large",
            });
        }
        let request =
            crate::runtime_transport::decode_exact::<ShellRequest>(line).ok_or(ShellRefusal {
                sequence: None,
                code: "standalone.shell.request-malformed",
            })?;
        let sequence = request.sequence();
        if sequence <= self.last_sequence {
            return Err(ShellRefusal {
                sequence: Some(sequence),
                code: "standalone.shell.sequence-not-increasing",
            });
        }
        self.last_sequence = sequence;
        Ok(request)
    }
}

/// The application side: one owned host process and the bridge over it.
pub struct StandaloneShellApp {
    state_root: std::path::PathBuf,
    disposable_root: std::path::PathBuf,
    step_delay_ms: u16,
    host: Option<LinuxDevelopmentHostProcess>,
    shell: Option<StandaloneShell<LinuxRuntimeIpcClient>>,
    state: ShellState,
}

impl StandaloneShellApp {
    /// Prepares the app for two absolute roots and the fixture's development
    /// delay; it launches nothing yet.
    #[must_use]
    pub fn new(state_root: &Path, disposable_root: &Path, step_delay_ms: u16) -> Self {
        Self {
            state_root: state_root.to_path_buf(),
            disposable_root: disposable_root.to_path_buf(),
            step_delay_ms,
            host: None,
            shell: None,
            state: ShellState::Starting,
        }
    }

    /// The current state.
    #[must_use]
    pub const fn state(&self) -> &ShellState {
        &self.state
    }

    /// Launches and authenticates a new host, replacing any earlier one. A
    /// replaced host's snapshot is gone; the folder must be admitted again.
    pub fn start(&mut self, presenter: &mut dyn ShellPresenter) {
        self.stop_host();
        self.state = ShellState::Starting;
        presenter.present(&ShellEvent::State {
            state: ShellState::Starting,
        });
        let state = match self.launch() {
            Ok((host, shell)) => {
                self.host = Some(host);
                self.shell = Some(shell);
                ShellState::Ready
            }
            Err(code) => ShellState::Unavailable { code },
        };
        self.state = state.clone();
        presenter.present(&ShellEvent::State { state });
    }

    fn launch(
        &self,
    ) -> Result<
        (
            LinuxDevelopmentHostProcess,
            StandaloneShell<LinuxRuntimeIpcClient>,
        ),
        String,
    > {
        let mut host = LinuxDevelopmentHostProcess::launch_standalone_evidence(
            &self.state_root,
            &self.disposable_root,
            self.step_delay_ms,
        )
        .map_err(|error| error.kind().code().to_owned())?;
        let envelope = match host.read_launch_envelope() {
            Ok(envelope) => envelope,
            Err(error) => {
                let _ = host.terminate_and_reap();
                return Err(error.kind().code().to_owned());
            }
        };
        let session = match envelope.connect_development() {
            Ok(session) => session,
            Err(error) => {
                let _ = host.terminate_and_reap();
                return Err(error.kind().code().to_owned());
            }
        };
        Ok((
            host,
            StandaloneShell::over(LinuxRuntimeIpcClient::new(session)),
        ))
    }

    fn break_session(&mut self, code: String, presenter: &mut dyn ShellPresenter) {
        self.shell = None;
        if let Some(mut host) = self.host.take() {
            let _ = host.terminate_and_reap();
        }
        self.state = ShellState::SafeMode { code: code.clone() };
        presenter.present(&ShellEvent::State {
            state: ShellState::SafeMode { code },
        });
    }

    fn stop_host(&mut self) {
        if let Some(mut shell) = self.shell.take() {
            let _ = shell.runtime_mut().shutdown();
        }
        if let Some(mut host) = self.host.take()
            && !matches!(host.wait_success(), Ok(true))
        {
            let _ = host.terminate_and_reap();
        }
    }

    /// Handles one admitted request. Returns `false` once the bridge has
    /// shut down.
    pub fn handle(&mut self, request: &ShellRequest, presenter: &mut dyn ShellPresenter) -> bool {
        let sequence = request.sequence();
        match request {
            ShellRequest::Shutdown { .. } => {
                self.stop_host();
                return false;
            }
            ShellRequest::Restart { .. } => {
                self.start(presenter);
                return true;
            }
            ShellRequest::Cancel { .. } => {
                presenter.present(&ShellEvent::Refused {
                    sequence: Some(sequence),
                    code: "standalone.shell.nothing-to-cancel".to_owned(),
                });
                return true;
            }
            ShellRequest::AdmitFolder { .. } | ShellRequest::Ask { .. } => {}
        }
        let Some(shell) = self.shell.as_mut() else {
            presenter.present(&ShellEvent::Refused {
                sequence: Some(sequence),
                code: "standalone.shell.not-ready".to_owned(),
            });
            return true;
        };
        let result = match request {
            ShellRequest::AdmitFolder { folder, .. } => shell.admit_folder(sequence, folder),
            ShellRequest::Ask { question, .. } => shell.ask(sequence, question, presenter),
            _ => return true,
        };
        match result {
            Ok(event) => presenter.present(&event),
            Err(failure) => self.break_session(failure.code, presenter),
        }
        true
    }

    /// Stops the host, if any, before the bridge ends.
    pub fn shutdown(&mut self) {
        self.stop_host();
    }
}

impl Drop for StandaloneShellApp {
    fn drop(&mut self) {
        self.stop_host();
    }
}

/// One line read by the development presentation.
enum Inbound {
    Line(Vec<u8>),
    TooLong,
    End,
}

/// The development presentation: closed JSON request lines in, closed JSON
/// event lines out. It is the bridge's message channel with no window; it is
/// not the product's presentation.
struct LinePresenter<W: std::io::Write> {
    output: W,
    inbox: std::sync::mpsc::Receiver<Inbound>,
    gate: ShellRequestGate,
    queued: std::collections::VecDeque<ShellRequest>,
    ended: bool,
}

impl<W: std::io::Write> LinePresenter<W> {
    fn receive(&mut self, inbound: Inbound) -> Option<ShellRequest> {
        match inbound {
            Inbound::End => {
                self.ended = true;
                None
            }
            Inbound::TooLong => {
                self.present(&ShellEvent::Refused {
                    sequence: None,
                    code: "standalone.shell.request-too-large".to_owned(),
                });
                None
            }
            Inbound::Line(line) => match self.gate.accept(&line) {
                Ok(request) => Some(request),
                Err(refusal) => {
                    self.present(&refusal.event());
                    None
                }
            },
        }
    }
}

impl<W: std::io::Write> ShellPresenter for LinePresenter<W> {
    fn present(&mut self, event: &ShellEvent) {
        // A presentation that has gone away cannot be told; the bridge goes on
        // and ends at end of input.
        if let Ok(mut line) = serde_json::to_vec(event) {
            line.push(b'\n');
            let _ = self
                .output
                .write_all(&line)
                .and_then(|()| self.output.flush());
        }
    }

    fn cancel_requested(&mut self) -> bool {
        let mut cancel = false;
        while let Ok(inbound) = self.inbox.try_recv() {
            // End of input means no more requests, not a cancel: the
            // question in progress ends and queued requests are handled.
            match self.receive(inbound) {
                Some(ShellRequest::Cancel { .. }) => cancel = true,
                Some(request) => self.queued.push_back(request),
                None => {}
            }
        }
        cancel
    }
}

fn read_lines(mut input: impl std::io::BufRead, sender: &std::sync::mpsc::Sender<Inbound>) {
    use std::io::{BufRead, Read};
    loop {
        let mut line = Vec::new();
        let limit = u64::try_from(MAX_SHELL_REQUEST_BYTES + 1).unwrap_or(u64::MAX);
        let read = (&mut input).take(limit).read_until(b'\n', &mut line);
        let inbound = match read {
            Ok(0) | Err(_) => Inbound::End,
            Ok(_) if line.last() == Some(&b'\n') => {
                line.pop();
                Inbound::Line(line)
            }
            Ok(_) if line.len() > MAX_SHELL_REQUEST_BYTES => {
                // Skip the rest of the oversized line.
                let mut rest = Vec::new();
                if input.read_until(b'\n', &mut rest).is_err() {
                    let _ = sender.send(Inbound::TooLong);
                    let _ = sender.send(Inbound::End);
                    return;
                }
                Inbound::TooLong
            }
            // A final line without a newline is still one request.
            Ok(_) => Inbound::Line(line),
        };
        let ended = matches!(inbound, Inbound::End);
        if sender.send(inbound).is_err() || ended {
            return;
        }
    }
}

/// Runs the bridge with the development presentation until shutdown or end
/// of input, then stops and reaps the host.
pub fn run_line_presentation(
    state_root: &Path,
    disposable_root: &Path,
    step_delay_ms: u16,
    input: impl std::io::BufRead + Send + 'static,
    output: impl std::io::Write,
) {
    let (sender, inbox) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || read_lines(input, &sender));
    let mut presenter = LinePresenter {
        output,
        inbox,
        gate: ShellRequestGate::default(),
        queued: std::collections::VecDeque::new(),
        ended: false,
    };
    let mut app = StandaloneShellApp::new(state_root, disposable_root, step_delay_ms);
    app.start(&mut presenter);
    loop {
        let request = if let Some(request) = presenter.queued.pop_front() {
            request
        } else if presenter.ended {
            break;
        } else {
            let Ok(inbound) = presenter.inbox.recv() else {
                break;
            };
            match presenter.receive(inbound) {
                Some(request) => request,
                None => continue,
            }
        };
        if !app.handle(&request, &mut presenter) {
            break;
        }
    }
    app.shutdown();
    drop(presenter);
    // The reader ends at end of input; a presentation that keeps its input
    // open after shutdown leaves only this thread, which exits with the process.
    if reader.is_finished() {
        let _ = reader.join();
    }
}

#[cfg(test)]
#[path = "standalone_shell_tests.rs"]
mod tests;
