// The bridge over the real in-process evidence service: real folder walk,
// prepared sources, coordinator and verified driver, with the term-match
// fixture as the only model; no process, socket or network.
use std::path::{Path, PathBuf};

use super::*;
use crate::cli_runtime::drive_interactive_cli_runtime;
use crate::standalone_evidence::StandaloneEvidenceService;

const PROJECT: &str = "# Aurora project\n\nAurora launches on 18 October 2026.\nProject lead: Mira Chen.\nThe Aurora budget is 42,000 credits.\n";

#[derive(Default)]
struct Recorder {
    events: Vec<ShellEvent>,
    cancel: bool,
}

impl ShellPresenter for Recorder {
    fn present(&mut self, event: &ShellEvent) {
        self.events.push(event.clone());
    }

    fn cancel_requested(&mut self) -> bool {
        self.cancel
    }
}

/// The evidence service as the IPC service presents it to one client: a job
/// control request is decided under that client's scope (Decision 0120).
struct Scoped(StandaloneEvidenceService);

impl RuntimeTransportPort for Scoped {
    fn prepare(
        &mut self,
        input: RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        self.0.prepare(input)
    }

    fn start(
        &mut self,
        request: RuntimeRunRequest,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.0.start(request)
    }

    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.0
            .advance(run_id, request_sha256, after_event_cursor, response)
    }

    fn cancel(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        self.0
            .cancel(run_id, request_sha256, cancellation_id, after_event_cursor)
    }

    fn run_declarations(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<RuntimeRunDeclarations, RuntimeTransportError> {
        self.0.run_declarations(run_id, request_sha256)
    }

    fn job_status(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<crate::runtime_transport::RuntimeJobStatus, RuntimeTransportError> {
        self.0.job_status(run_id, request_sha256)
    }

    fn control_job(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        request: &agentmage_kernel_engine::job_control::JobControlRequest,
    ) -> Result<crate::runtime_transport::RuntimeJobControl, RuntimeTransportError> {
        self.0.control_job_for_client(
            &crate::runtime_transport::RuntimeClientScope::derived(
                "peer-00112233445566778899aabbccddeeff".to_owned(),
            ),
            run_id,
            request_sha256,
            request,
        )
    }

    fn folder(&mut self, request: FolderRequest) -> Result<FolderAnswer, RuntimeTransportError> {
        self.0.folder(request)
    }

    fn release(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<(), RuntimeTransportError> {
        self.0.release(run_id, request_sha256)
    }
}

fn folder(roots: &fixture::Roots) -> PathBuf {
    let folder = roots.folder("aurora");
    fixture::write(&folder.join("project.md"), PROJECT.as_bytes());
    fixture::write(&folder.join("notes.pdf"), b"%PDF-1.4");
    folder
}

fn shell(roots: &fixture::Roots) -> StandaloneShell<Scoped> {
    StandaloneShell::over(Scoped(
        StandaloneEvidenceService::new(roots.activation(), std::time::Duration::ZERO)
            .expect("service"),
    ))
}

fn admitted<P: RuntimeTransportPort>(shell: &mut StandaloneShell<P>, folder: &Path) {
    match shell
        .admit_folder(1, folder.to_str().unwrap())
        .expect("answers")
    {
        ShellEvent::Folder { admission, .. } => {
            assert!(admission.verify());
            assert_eq!(admission.accepted, 1);
        }
        other => panic!("not admitted: {other:?}"),
    }
}

fn answer_of(event: ShellEvent) -> AnswerView {
    match event {
        ShellEvent::Answer { answer, .. } => answer,
        other => panic!("not an answer: {other:?}"),
    }
}

#[test]
fn a_question_ends_in_a_verified_answer_with_exact_evidence_cards() {
    let roots = fixture::Roots::new("shell-verified");
    let folder = folder(&roots);
    let mut shell = shell(&roots);
    let mut recorder = Recorder::default();
    assert_eq!(
        shell.ask(1, "When does Aurora launch?", &mut recorder),
        Ok(ShellEvent::Refused {
            sequence: Some(1),
            code: "standalone.shell.no-folder".to_owned()
        })
    );
    admitted(&mut shell, &folder);
    let answer = answer_of(
        shell
            .ask(2, "When does Aurora launch?", &mut recorder)
            .expect("answers"),
    );
    assert_eq!(
        answer.state,
        AnswerState::Verified,
        "{:?}",
        answer.unresolved_codes
    );
    assert_eq!(answer.label, FIXTURE_ANSWER_LABEL);
    assert_eq!(answer.statements.len(), 1);
    assert_eq!(
        answer.statements[0].text,
        "Aurora launches on 18 October 2026."
    );
    assert_eq!(answer.cards.len(), 1);
    let card = &answer.cards[0];
    assert_eq!(
        answer.statements[0].card_ids,
        std::slice::from_ref(&card.card_id)
    );
    assert_eq!(card.path, "project.md");
    assert_eq!((card.first_line, card.last_line), (3, 3));
    assert_eq!(card.quote, "Aurora launches on 18 October 2026.");
    assert_eq!(
        PROJECT
            .lines()
            .nth(usize::try_from(card.first_line).unwrap() - 1),
        Some(card.quote.as_str())
    );
    assert_eq!(answer.context_omissions, Some(0));
    assert!(matches!(
        recorder.events.first(),
        Some(ShellEvent::QuestionStarted { sequence: 2, .. })
    ));
    let progress: Vec<&str> = recorder
        .events
        .iter()
        .filter_map(|event| match event {
            ShellEvent::Progress { event, .. } => Some(event.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(progress.first(), Some(&"run_started"));
    assert!(progress.contains(&"tool_completed"));
    // A follow-up continues the same session.
    let follow = answer_of(
        shell
            .ask(3, "What is the budget?", &mut recorder)
            .expect("answers"),
    );
    assert_eq!(follow.state, AnswerState::Verified);
    assert_eq!(
        follow.cards[0].quote,
        "The Aurora budget is 42,000 credits."
    );
}

#[test]
fn unanswerable_invalid_and_cancelled_questions_show_no_evidence() {
    let roots = fixture::Roots::new("shell-refusals");
    let folder = folder(&roots);
    let mut shell = shell(&roots);
    let mut recorder = Recorder::default();
    admitted(&mut shell, &folder);
    for question in ["   ", &"q".repeat(MAX_SHELL_QUESTION_BYTES + 1)] {
        assert_eq!(
            shell.ask(4, question, &mut recorder),
            Ok(ShellEvent::Refused {
                sequence: Some(4),
                code: "standalone.shell.question-invalid".to_owned()
            })
        );
    }
    let unmatched = answer_of(
        shell
            .ask(5, "What is the favorite flavor?", &mut recorder)
            .expect("answers"),
    );
    assert_eq!(unmatched.state, AnswerState::Unverified);
    assert!(unmatched.no_matching_text);
    assert!(unmatched.cards.is_empty() && unmatched.statements.is_empty());
    assert_eq!(unmatched.unresolved_codes, ["runtime.verification.failed"]);

    // The fixture waits two seconds before each proposal and observes the
    // cancellation the bridge requests through the job ledger while it waits.
    let mut gated = StandaloneShell::over(Scoped(
        StandaloneEvidenceService::new(
            roots.activation(),
            crate::standalone_evidence::MAX_FIXTURE_STEP_DELAY,
        )
        .expect("service"),
    ));
    admitted(&mut gated, &folder);
    let mut cancelling = Recorder {
        cancel: true,
        ..Recorder::default()
    };
    let cancelled = answer_of(
        gated
            .ask(6, "When does Aurora launch?", &mut cancelling)
            .expect("answers"),
    );
    assert_eq!(
        cancelled.state,
        AnswerState::Cancelled,
        "{:?}",
        cancelled.unresolved_codes
    );
    assert!(cancelled.cards.is_empty());
}

#[test]
fn a_refused_folder_keeps_the_earlier_snapshot() {
    let roots = fixture::Roots::new("shell-folder");
    let folder = folder(&roots);
    let mut shell = shell(&roots);
    admitted(&mut shell, &folder);
    let before = shell.admission().cloned();
    assert_eq!(
        shell.admit_folder(2, "relative").expect("answers"),
        ShellEvent::FolderRefused {
            sequence: 2,
            code: "standalone.folder.not-absolute".to_owned()
        }
    );
    assert_eq!(shell.admission().cloned(), before);
}

#[test]
fn only_a_verified_run_with_observed_citations_becomes_evidence_cards() {
    let roots = fixture::Roots::new("shell-projection");
    let folder = folder(&roots);
    let mut service = StandaloneEvidenceService::new(roots.activation(), std::time::Duration::ZERO)
        .expect("service");
    let admission = match service
        .folder(FolderRequest::Admit {
            folder: folder.to_str().unwrap().to_owned(),
        })
        .expect("answers")
    {
        FolderAnswer::Admitted { admission } => admission,
        other => panic!("{other:?}"),
    };
    struct Quiet;
    impl CodingEventSink for Quiet {
        fn present(&mut self, _event: &RuntimeEvent) -> Result<(), CodingClientError> {
            Ok(())
        }
    }
    struct Never;
    impl InteractiveCliCancellationPort for Never {
        fn poll(
            &mut self,
            _request: &RuntimeRunRequest,
        ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError> {
            Ok(None)
        }
    }
    let result = drive_interactive_cli_runtime(
        &mut service,
        RuntimePrepareInput {
            resume: false,
            record_session: false,
            slow_subscriber_probe: false,
            preauthorization: None,
            engineering_session_id: None,
            profile_id: STANDALONE_EVIDENCE_PROFILE_ID.to_owned(),
            expected_entry_sha256: admission.admission_sha256.clone(),
            workspace_id: admission.workspace_id.clone(),
            workspace_root: admission.folder.clone(),
            prompt: "When does Aurora launch?".to_owned(),
            recipe: None,
        },
        &mut DenyHeadlessApproval,
        &mut Quiet,
        &mut Never,
    )
    .expect("the run ends");
    assert_eq!(
        project_answer(&result, &admission).state,
        AnswerState::Verified
    );
    // Evidence the run did not observe, or a file outside the snapshot,
    // never becomes a card.
    let mut unobserved = result.clone();
    unobserved.outcome.evidence.clear();
    let view = project_answer(&unobserved, &admission);
    assert_eq!(view.state, AnswerState::Unverified);
    assert!(view.cards.is_empty());
    assert!(
        view.unresolved_codes
            .contains(&"standalone.shell.citation-unmatched".to_owned())
    );
    let mut other = admission.clone();
    for entry in &mut other.entries {
        entry.source_id = entry
            .source_id
            .as_ref()
            .map(|_| "folder-source-999".to_owned());
    }
    assert_eq!(
        project_answer(&result, &other).state,
        AnswerState::Unverified
    );
    // A failed or cancelled outcome never shows statements.
    let mut failed = result;
    failed.outcome.state = AgentStateKind::Failed;
    let view = project_answer(&failed, &admission);
    assert_eq!(view.state, AnswerState::Unverified);
    assert!(view.statements.is_empty() && view.cards.is_empty());
}

#[test]
fn the_request_gate_admits_only_exact_requests_in_increasing_order() {
    let mut gate = ShellRequestGate::default();
    assert_eq!(
        gate.accept(br#"{"type":"admit_folder","sequence":1,"folder":"/x"}"#),
        Ok(ShellRequest::AdmitFolder {
            sequence: 1,
            folder: "/x".to_owned()
        })
    );
    let malformed = |code: &'static str| {
        Err(ShellRefusal {
            sequence: None,
            code,
        })
    };
    for line in [
        &br#"{"type":"ask","sequence":2,"question":"q","extra":1}"#[..],
        br#"{"type":"ask","sequence":2,"sequence":3,"question":"q"}"#,
        br#"{"type":"approve","sequence":2}"#,
        br#"{"type":"ask","sequence":-2,"question":"q"}"#,
        b"not json",
    ] {
        assert_eq!(
            gate.accept(line),
            malformed("standalone.shell.request-malformed")
        );
    }
    let refused = gate
        .accept(br#"{"type":"cancel","sequence":1}"#)
        .unwrap_err();
    assert_eq!(
        refused,
        ShellRefusal {
            sequence: Some(1),
            code: "standalone.shell.sequence-not-increasing"
        }
    );
    assert_eq!(
        refused.event(),
        ShellEvent::Refused {
            sequence: Some(1),
            code: "standalone.shell.sequence-not-increasing".to_owned()
        }
    );
    assert_eq!(
        gate.accept(&vec![b' '; MAX_SHELL_REQUEST_BYTES + 1]),
        malformed("standalone.shell.request-too-large")
    );
    assert_eq!(
        gate.accept(br#"{"type":"shutdown","sequence":7}"#),
        Ok(ShellRequest::Shutdown { sequence: 7 })
    );
}

#[test]
fn every_event_and_request_is_closed() {
    for event in [
        ShellEvent::State {
            state: ShellState::SafeMode {
                code: "host.runtime.failed".to_owned(),
            },
        },
        ShellEvent::Refused {
            sequence: None,
            code: "standalone.shell.request-malformed".to_owned(),
        },
    ] {
        let bytes = serde_json::to_vec(&event).unwrap();
        assert_eq!(
            crate::runtime_transport::decode_exact::<ShellEvent>(&bytes),
            Some(event)
        );
    }
    let state = serde_json::to_value(ShellState::Ready).unwrap();
    assert_eq!(state, serde_json::json!({"state": "ready"}));
}

// The fixture creates files and directories, so it is a test module of its
// own for the effect boundary scan.
#[cfg(test)]
mod fixture {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::standalone_evidence::StandaloneEvidenceActivation;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    pub(super) struct Roots {
        base: PathBuf,
        state: PathBuf,
        disposable: PathBuf,
    }

    impl Roots {
        pub(super) fn new(tag: &str) -> Self {
            let base = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "agentmage-{tag}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let state = base.join("state");
            let disposable = base.join("disposable");
            for path in [&base, &state, &disposable] {
                directory(path);
            }
            Self {
                base,
                state,
                disposable,
            }
        }

        pub(super) fn activation(&self) -> StandaloneEvidenceActivation {
            StandaloneEvidenceActivation::validate(&self.state, &self.disposable)
                .expect("test roots activate")
        }

        pub(super) fn folder(&self, name: &str) -> PathBuf {
            let path = self.disposable.join(name);
            directory(&path);
            path
        }
    }

    impl Drop for Roots {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }

    fn directory(path: &Path) {
        std::fs::create_dir(path).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    pub(super) fn write(path: &Path, bytes: &[u8]) {
        std::fs::write(path, bytes).unwrap();
    }
}
