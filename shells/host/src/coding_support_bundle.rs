//! Support bundles of one development CLI invocation (Decision 0128).
//!
//! The development CLI builds a content-free support bundle from what it
//! observed during its own invocation: which source was selected, whether the
//! host served every run or which content-free failure ended it, and the
//! digests the host declared about each run. The CLI does not probe the
//! machine, so every doctor component it did not observe is reported missing.
//! The bundle is previewed and published only through the existing one-use
//! diagnostic export workflow, after the person types `yes`, into a private
//! local directory. Nothing here uploads anything or grants authority.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write;
use std::path::Path;

use agentmage_kernel_contracts::{DiagnosticComponent, DiagnosticObservation, DiagnosticState};
use agentmage_kernel_engine::diagnostics::build_doctor_report;
use agentmage_kernel_engine::job_control::JobObservation;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::cli::{CLI_VERSION, CliOutputFormat};
use crate::diagnostic_export::{
    DiagnosticExportError, DiagnosticExportPreview, DiagnosticExportReceipt,
    DiagnosticExportWorkflow, SupportBundle, SupportBundleEvidence, SupportBundleVersion,
    build_support_bundle,
};
use crate::runtime_transport::RuntimeRunDeclarations;

/// The confirmation word that publishes a previewed bundle.
const CONFIRMATION_WORD: &str = "yes";

/// What the CLI observed about one run the host served.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedRun {
    /// Digest of the exact run request.
    pub request_sha256: String,
    /// The host's verified declarations about the run, when it made them.
    pub declarations: Option<RuntimeRunDeclarations>,
    /// The run's reconciled job state, when the driver read it.
    pub job: Option<JobObservation>,
}

/// What the CLI observed during one invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvocationObservation {
    /// Exact profile identity of the selected source.
    pub profile_id: String,
    /// Whether the source is the scripted fixture rather than a model.
    pub scripted: bool,
    /// Each run the host served, in order.
    pub runs: Vec<ObservedRun>,
    /// Content-free code of the failure that ended the invocation, if any.
    pub failure_code: Option<&'static str>,
}

/// How one offered support bundle ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupportBundleOutcome {
    /// Nothing was asked: the invocation was cancelled.
    Skipped,
    /// The bundle, destination or preview was refused before anything was
    /// shown; nothing was written.
    Refused(DiagnosticExportError),
    /// The person did not confirm; the preview was cancelled and nothing was
    /// written.
    Declined(Box<DiagnosticExportPreview>),
    /// The confirmed preview was published once.
    Published(Box<DiagnosticExportPreview>, DiagnosticExportReceipt),
    /// The confirmed preview could not be published; nothing replaced the
    /// destination.
    Failed(Box<DiagnosticExportPreview>, DiagnosticExportError),
}

/// What the person answered to the confirmation prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportBundleAnswer {
    /// The exact confirmation word.
    Confirmed,
    /// Anything else, end of input or an over-long line.
    Declined,
    /// Cancellation was requested while waiting.
    Cancelled,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn observation(
    component: DiagnosticComponent,
    state: DiagnosticState,
    reason_code: &str,
    identity_sha256: Option<String>,
) -> DiagnosticObservation {
    DiagnosticObservation {
        component,
        state,
        reason_code: reason_code.to_owned(),
        identity_sha256,
        stale: false,
    }
}

/// The doctor observations the CLI made. Components it did not observe are
/// left out, so the report names them missing.
fn invocation_observations(observed: &InvocationObservation) -> Vec<DiagnosticObservation> {
    let last = observed
        .runs
        .last()
        .and_then(|run| run.declarations.as_ref());
    let mut observations = vec![
        observation(
            DiagnosticComponent::Package,
            DiagnosticState::Degraded,
            "diagnostic.package.development-build",
            None,
        ),
        observation(
            DiagnosticComponent::Platform,
            DiagnosticState::Healthy,
            "diagnostic.platform.linux-development",
            None,
        ),
        observation(
            DiagnosticComponent::Model,
            DiagnosticState::Degraded,
            if observed.scripted {
                "diagnostic.model.scripted-fixture"
            } else {
                "diagnostic.model.development-candidate"
            },
            Some(sha256_hex(observed.profile_id.as_bytes())),
        ),
        match (observed.failure_code, observed.runs.is_empty()) {
            (Some(code), _) => observation(
                DiagnosticComponent::Runtime,
                DiagnosticState::Blocked,
                code,
                None,
            ),
            (None, false) => observation(
                DiagnosticComponent::Runtime,
                DiagnosticState::Healthy,
                "diagnostic.runtime.host-served",
                None,
            ),
            (None, true) => observation(
                DiagnosticComponent::Runtime,
                DiagnosticState::Unavailable,
                "diagnostic.runtime.not-served",
                None,
            ),
        },
        match last.and_then(|value| value.route_receipt.as_ref()) {
            Some(receipt) => observation(
                DiagnosticComponent::OfflineBoundary,
                DiagnosticState::Healthy,
                "diagnostic.offline.local-only-route",
                Some(receipt.receipt_sha256.clone()),
            ),
            None => observation(
                DiagnosticComponent::OfflineBoundary,
                DiagnosticState::Unavailable,
                "diagnostic.offline.route-unobserved",
                None,
            ),
        },
    ];
    observations.push(match last {
        None => observation(
            DiagnosticComponent::ReceiptChain,
            DiagnosticState::Unavailable,
            "diagnostic.receipt-chain.unobserved",
            None,
        ),
        Some(declarations) => {
            let heads = [
                declarations.effect_history.as_ref(),
                declarations.job_control_history.as_ref(),
                declarations.route_history.as_ref(),
            ]
            .map(|history| history.map(|value| value.head.head_sha256.clone()));
            if heads.iter().all(Option::is_some) {
                let identity = serde_json::to_vec(&serde_json::json!({
                    "record_type": "agentmage-support-bundle-chain-heads",
                    "heads": heads,
                }))
                .ok()
                .map(|bytes| sha256_hex(&bytes));
                observation(
                    DiagnosticComponent::ReceiptChain,
                    DiagnosticState::Healthy,
                    "diagnostic.receipt-chain.replayed",
                    identity,
                )
            } else {
                observation(
                    DiagnosticComponent::ReceiptChain,
                    DiagnosticState::Degraded,
                    "diagnostic.receipt-chain.incomplete",
                    None,
                )
            }
        }
    });
    observations.push(
        match observed.runs.last().and_then(|run| run.job.as_ref()) {
            Some(job) if job.phase.is_terminal() => observation(
                DiagnosticComponent::Recovery,
                DiagnosticState::Healthy,
                "diagnostic.recovery.job-ended",
                Some(job.head_sha256.clone()),
            ),
            Some(job) => observation(
                DiagnosticComponent::Recovery,
                DiagnosticState::Degraded,
                "diagnostic.recovery.job-open",
                Some(job.head_sha256.clone()),
            ),
            None => observation(
                DiagnosticComponent::Recovery,
                DiagnosticState::Unavailable,
                "diagnostic.recovery.job-unobserved",
                None,
            ),
        },
    );
    observations
}

/// The versions a support bundle names: the CLI, the run declarations schema
/// and the IPC wire version.
fn invocation_versions() -> Vec<SupportBundleVersion> {
    [
        ("agentmage-cli", CLI_VERSION.to_owned()),
        (
            "ipc-wire",
            crate::runtime_ipc::RUNTIME_IPC_WIRE_VERSION.to_string(),
        ),
        (
            "run-declarations",
            crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION.to_string(),
        ),
    ]
    .into_iter()
    .map(|(component, version)| SupportBundleVersion {
        component: component.to_owned(),
        version,
        content_sha256: None,
    })
    .collect()
}

/// The digests of every run: request, recoverability report, route receipt,
/// the three history heads and the job ledger head, sorted and unique.
fn invocation_evidence(observed: &InvocationObservation) -> Vec<SupportBundleEvidence> {
    let mut evidence = BTreeSet::new();
    let mut add = |kind: &str, digest: &str| {
        evidence.insert(SupportBundleEvidence {
            record_kind: kind.to_owned(),
            record_sha256: digest.to_owned(),
        });
    };
    for run in &observed.runs {
        add("run-request", &run.request_sha256);
        if let Some(declarations) = &run.declarations {
            if let Some(report) = &declarations.recoverability {
                add("recoverability-report", &report.report_sha256);
            }
            if let Some(receipt) = &declarations.route_receipt {
                add("route-receipt", &receipt.receipt_sha256);
            }
            for (kind, history) in [
                ("effect-history-head", &declarations.effect_history),
                (
                    "job-control-history-head",
                    &declarations.job_control_history,
                ),
                ("route-history-head", &declarations.route_history),
            ] {
                if let Some(history) = history {
                    add(kind, &history.head.head_sha256);
                }
            }
        }
        if let Some(job) = &run.job {
            add("job-ledger-head", &job.head_sha256);
        }
    }
    evidence.into_iter().collect()
}

/// The support bundle of one invocation.
pub fn build_invocation_support_bundle(
    observed: &InvocationObservation,
) -> Result<SupportBundle, DiagnosticExportError> {
    let doctor = build_doctor_report(invocation_observations(observed))
        .map_err(|_| DiagnosticExportError::ReportDenied)?;
    build_support_bundle(doctor, invocation_versions(), invocation_evidence(observed))
}

#[derive(Serialize)]
struct PreviewRow<'a> {
    #[serde(rename = "type")]
    row_type: &'static str,
    preview_id: &'a str,
    destination_sha256: &'a str,
    payload_sha256: &'a str,
    payload_bytes: u64,
    included_fields: &'a [String],
    redactions: &'a [String],
    sensitivity: &'a str,
    retention: &'a str,
    expires_at_epoch_ms: u64,
    confirmation_sha256: &'a str,
}

fn render_preview(preview: &DiagnosticExportPreview, output: CliOutputFormat) -> String {
    match output {
        CliOutputFormat::Json => format!(
            "{}\n",
            serde_json::to_string(&PreviewRow {
                row_type: "support_bundle_preview",
                preview_id: &preview.preview_id,
                destination_sha256: &preview.destination_sha256,
                payload_sha256: &preview.payload_sha256,
                payload_bytes: preview.payload_bytes,
                included_fields: &preview.included_fields,
                redactions: &preview.redactions,
                sensitivity: &preview.sensitivity,
                retention: &preview.retention,
                expires_at_epoch_ms: preview.expires_at_epoch_ms,
                confirmation_sha256: &preview.confirmation_sha256,
            })
            .unwrap_or_default()
        ),
        CliOutputFormat::Human => format!(
            "support_bundle_preview id={} destination_sha256={} payload_sha256={} bytes={} fields={} redactions={} sensitivity={} retention={} expires_at_epoch_ms={} confirmation_sha256={}\n",
            preview.preview_id,
            preview.destination_sha256,
            preview.payload_sha256,
            preview.payload_bytes,
            preview.included_fields.join(","),
            preview.redactions.join(","),
            preview.sensitivity,
            preview.retention,
            preview.expires_at_epoch_ms,
            preview.confirmation_sha256,
        ),
    }
}

/// The line that ends one offer: what was written, or why nothing was.
#[must_use]
pub fn render_support_bundle_outcome(
    outcome: &SupportBundleOutcome,
    output: CliOutputFormat,
) -> String {
    let (result, code, receipt) = match outcome {
        SupportBundleOutcome::Skipped => ("skipped", "support-bundle.cancelled", None),
        SupportBundleOutcome::Refused(error) => ("refused", error.code(), None),
        SupportBundleOutcome::Declined(_) => ("declined", "support-bundle.not-confirmed", None),
        SupportBundleOutcome::Published(_, receipt) => {
            ("published", "support-bundle.published", Some(receipt))
        }
        SupportBundleOutcome::Failed(_, error) => ("failed", error.code(), None),
    };
    match (output, receipt) {
        (CliOutputFormat::Json, receipt) => format!(
            "{}\n",
            serde_json::json!({
                "type": "support_bundle",
                "result": result,
                "code": code,
                "written": receipt.is_some(),
                "payload_sha256": receipt.map(|value| value.payload_sha256.as_str()),
                "destination_sha256": receipt.map(|value| value.destination_sha256.as_str()),
                "payload_bytes": receipt.map(|value| value.payload_bytes),
                "uploaded": false,
            })
        ),
        (CliOutputFormat::Human, Some(receipt)) => format!(
            "support_bundle published payload_sha256={} destination_sha256={} bytes={} uploaded=false\n",
            receipt.payload_sha256, receipt.destination_sha256, receipt.payload_bytes
        ),
        (CliOutputFormat::Human, None) => {
            format!("support_bundle {result} code={code} written=false uploaded=false\n")
        }
    }
}

/// Offers one support bundle: builds it, previews a new file named by the
/// preview identity in `directory`, shows the preview and asks for the
/// confirmation word, then publishes it once or cancels it. Every line goes
/// to `prompts`, which is standard error. `answer` is asked only after the
/// preview was shown; `now` is read at preview and again at approval.
pub fn offer_support_bundle(
    observed: &InvocationObservation,
    directory: &Path,
    preview_id: &str,
    output: CliOutputFormat,
    prompts: &mut dyn Write,
    now: &mut dyn FnMut() -> Option<u64>,
    answer: &mut dyn FnMut() -> SupportBundleAnswer,
) -> SupportBundleOutcome {
    let mut workflow = DiagnosticExportWorkflow::new();
    let preview = build_invocation_support_bundle(observed).and_then(|bundle| {
        let now = now().ok_or(DiagnosticExportError::ApprovalDenied)?;
        workflow.preview_support_bundle(
            preview_id.to_owned(),
            &bundle,
            &directory.join(format!("agentmage-support-bundle-{preview_id}.json")),
            now,
        )
    });
    let preview = match preview {
        Ok(preview) => preview,
        Err(error) => return SupportBundleOutcome::Refused(error),
    };
    let _ = prompts.write_all(render_preview(&preview, output).as_bytes());
    let _ = write!(
        prompts,
        "Write this support bundle once into the directory? Type {CONFIRMATION_WORD} to write it; anything else writes nothing: "
    );
    let _ = prompts.flush();
    match answer() {
        SupportBundleAnswer::Confirmed => {}
        SupportBundleAnswer::Declined | SupportBundleAnswer::Cancelled => {
            workflow.cancel(&preview.preview_id);
            return SupportBundleOutcome::Declined(Box::new(preview));
        }
    }
    let approved = now()
        .ok_or(DiagnosticExportError::ApprovalDenied)
        .and_then(|now| workflow.approve(&preview.preview_id, &preview.confirmation_sha256, now));
    match approved {
        Ok(receipt) => SupportBundleOutcome::Published(Box::new(preview), receipt),
        Err(error) => SupportBundleOutcome::Failed(Box::new(preview), error),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::DoctorReport;
    use agentmage_kernel_engine::job_control::JobPhase;

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    /// A private directory the export workflow admits.
    struct PrivateDirectory(PathBuf);

    impl PrivateDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-support-bundle-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }

        fn entries(&self) -> Vec<PathBuf> {
            let mut entries = fs::read_dir(&self.0)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            entries.sort();
            entries
        }
    }

    impl Drop for PrivateDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn history(
        kind: agentmage_kernel_engine::action_history::ActionKind,
        digit: char,
    ) -> crate::coding_action_history::RunActionHistory {
        use agentmage_kernel_engine::action_history::{
            ActionAuthorization, ActionOutcome, ActionRecordDraft,
        };
        let mut recorder = crate::coding_action_history::RunActionRecorder::new();
        recorder.record(Some(ActionRecordDraft {
            action_kind: kind,
            action_id: format!("action-{digit}"),
            authorization: ActionAuthorization::PersonDecision {
                decision_sha256: digit.to_string().repeat(64),
            },
            effect_sha256: "e".repeat(64),
            outcome: ActionOutcome::Succeeded,
            reason_code: "coding.approved.succeeded".to_owned(),
            evidence_sha256s: Vec::new(),
            recorded_at_epoch_ms: 1,
            retain_until_epoch_ms: 2,
        }));
        recorder.declare().unwrap()
    }

    fn job(phase: JobPhase) -> JobObservation {
        JobObservation {
            job_id: "run-bundle".to_owned(),
            phase,
            revision: 3,
            cancellation_requested: false,
            head_sha256: "a".repeat(64),
        }
    }

    fn served() -> InvocationObservation {
        use agentmage_kernel_engine::action_history::ActionKind;
        InvocationObservation {
            profile_id: "scripted-executable-fixture-32k-v1".to_owned(),
            scripted: true,
            runs: vec![ObservedRun {
                request_sha256: "1".repeat(64),
                declarations: Some(RuntimeRunDeclarations {
                    schema_version: crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION,
                    run_id: agentmage_kernel_contracts::RuntimeRunId::from_raw("run-bundle"),
                    request_sha256: "1".repeat(64),
                    recoverability: None,
                    context_inspections: None,
                    effect_history: Some(history(ActionKind::FileWrite, '2')),
                    job_control_history: Some(history(ActionKind::JobControl, '3')),
                    route_receipt: None,
                    route_history: Some(history(ActionKind::ModelRoute, '4')),
                }),
                job: Some(job(JobPhase::Completed)),
            }],
            failure_code: None,
        }
    }

    fn item(report: &DoctorReport, component: DiagnosticComponent) -> (DiagnosticState, &str) {
        let item = report
            .items
            .iter()
            .find(|item| item.component == component)
            .unwrap();
        (item.state, item.reason_code.as_str())
    }

    #[test]
    fn the_bundle_names_only_what_the_invocation_observed() {
        let bundle = build_invocation_support_bundle(&served()).unwrap();
        let report = &bundle.doctor;
        assert_eq!(report.items.len(), DiagnosticComponent::ALL.len());
        assert_eq!(
            item(report, DiagnosticComponent::Package),
            (
                DiagnosticState::Degraded,
                "diagnostic.package.development-build"
            )
        );
        assert_eq!(
            item(report, DiagnosticComponent::Model),
            (
                DiagnosticState::Degraded,
                "diagnostic.model.scripted-fixture"
            )
        );
        assert_eq!(
            item(report, DiagnosticComponent::Runtime),
            (DiagnosticState::Healthy, "diagnostic.runtime.host-served")
        );
        // No verified route receipt was declared, so the offline boundary is
        // not claimed.
        assert_eq!(
            item(report, DiagnosticComponent::OfflineBoundary),
            (
                DiagnosticState::Unavailable,
                "diagnostic.offline.route-unobserved"
            )
        );
        assert_eq!(
            item(report, DiagnosticComponent::ReceiptChain),
            (
                DiagnosticState::Healthy,
                "diagnostic.receipt-chain.replayed"
            )
        );
        assert_eq!(
            item(report, DiagnosticComponent::Recovery),
            (DiagnosticState::Healthy, "diagnostic.recovery.job-ended")
        );
        // Unobserved components are missing, never guessed.
        for component in [
            DiagnosticComponent::HardwareFit,
            DiagnosticComponent::SandboxHelper,
            DiagnosticComponent::WorkspaceGrant,
            DiagnosticComponent::Capabilities,
            DiagnosticComponent::RepositoryMap,
            DiagnosticComponent::EncryptedStore,
        ] {
            assert_eq!(
                item(report, component),
                (DiagnosticState::Unavailable, "diagnostic.component.missing")
            );
        }
        let wire = crate::runtime_ipc::RUNTIME_IPC_WIRE_VERSION.to_string();
        assert_eq!(
            bundle
                .versions
                .iter()
                .map(|version| (version.component.as_str(), version.version.as_str()))
                .collect::<Vec<_>>(),
            [
                ("agentmage-cli", CLI_VERSION),
                ("ipc-wire", wire.as_str()),
                ("run-declarations", "3"),
            ]
        );
        assert_eq!(
            bundle
                .evidence
                .iter()
                .map(|record| record.record_kind.as_str())
                .collect::<Vec<_>>(),
            [
                "effect-history-head",
                "job-control-history-head",
                "job-ledger-head",
                "route-history-head",
                "run-request",
            ]
        );
        assert!(!bundle.automatic_upload);

        // A startup failure is the runtime's blocked reason; nothing else
        // about runs is claimed.
        let failed = InvocationObservation {
            runs: Vec::new(),
            failure_code: Some("linux.development.launch-envelope.failed"),
            scripted: false,
            profile_id: "muse-glimmer-30b-q4-k-m-text-32k-fedora-coding-development".to_owned(),
        };
        let bundle = build_invocation_support_bundle(&failed).unwrap();
        let report = &bundle.doctor;
        assert_eq!(
            item(report, DiagnosticComponent::Runtime),
            (
                DiagnosticState::Blocked,
                "linux.development.launch-envelope.failed"
            )
        );
        assert_eq!(
            item(report, DiagnosticComponent::Model),
            (
                DiagnosticState::Degraded,
                "diagnostic.model.development-candidate"
            )
        );
        assert_eq!(
            item(report, DiagnosticComponent::ReceiptChain),
            (
                DiagnosticState::Unavailable,
                "diagnostic.receipt-chain.unobserved"
            )
        );
        assert_eq!(
            item(report, DiagnosticComponent::Recovery),
            (
                DiagnosticState::Unavailable,
                "diagnostic.recovery.job-unobserved"
            )
        );
        assert!(bundle.evidence.is_empty());

        // An open job and a missing chain are said to be so.
        let mut open = served();
        open.runs[0].job = Some(job(JobPhase::Suspended));
        open.runs[0]
            .declarations
            .as_mut()
            .unwrap()
            .job_control_history = None;
        let report = build_invocation_support_bundle(&open).unwrap().doctor;
        assert_eq!(
            item(&report, DiagnosticComponent::Recovery),
            (DiagnosticState::Degraded, "diagnostic.recovery.job-open")
        );
        assert_eq!(
            item(&report, DiagnosticComponent::ReceiptChain),
            (
                DiagnosticState::Degraded,
                "diagnostic.receipt-chain.incomplete"
            )
        );
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn a_verified_local_only_route_is_the_offline_observation() {
        let (request, _, _, _) = crate::runtime_read_tests::completed_native_read_fixture();
        let route =
            crate::coding_route::route_development_run(&request, "contract-test", 5_000).unwrap();
        let mut observed = served();
        let declarations = observed.runs[0].declarations.as_mut().unwrap();
        declarations.route_receipt = Some(route.receipt.clone());
        let bundle = build_invocation_support_bundle(&observed).unwrap();
        let offline = bundle
            .doctor
            .items
            .iter()
            .find(|item| item.component == DiagnosticComponent::OfflineBoundary)
            .unwrap();
        assert_eq!(offline.state, DiagnosticState::Healthy);
        assert_eq!(offline.reason_code, "diagnostic.offline.local-only-route");
        assert_eq!(
            offline.identity_sha256.as_deref(),
            Some(route.receipt.receipt_sha256.as_str())
        );
        assert!(bundle.evidence.contains(&SupportBundleEvidence {
            record_kind: "route-receipt".to_owned(),
            record_sha256: route.receipt.receipt_sha256.clone(),
        }));
    }

    fn offer(
        directory: &Path,
        output: CliOutputFormat,
        answer: SupportBundleAnswer,
    ) -> (SupportBundleOutcome, String, usize) {
        let mut prompts = Vec::new();
        let mut asked = 0;
        let outcome = offer_support_bundle(
            &served(),
            directory,
            "support-bundle-1",
            output,
            &mut prompts,
            &mut || Some(10_000),
            &mut || {
                asked += 1;
                answer
            },
        );
        (outcome, String::from_utf8(prompts).unwrap(), asked)
    }

    #[test]
    fn only_a_confirmed_preview_is_published_once_into_the_private_directory() {
        for output in [CliOutputFormat::Human, CliOutputFormat::Json] {
            let directory = PrivateDirectory::new("publish");
            let (outcome, prompts, asked) =
                offer(&directory.0, output, SupportBundleAnswer::Confirmed);
            assert_eq!(asked, 1);
            let SupportBundleOutcome::Published(preview, receipt) = &outcome else {
                panic!("published: {outcome:?}");
            };
            // The preview was shown before the question.
            let preview_line = prompts.lines().next().unwrap();
            match output {
                CliOutputFormat::Human => {
                    assert!(
                        preview_line.starts_with("support_bundle_preview id=support-bundle-1 ")
                    );
                }
                CliOutputFormat::Json => {
                    let row: serde_json::Value = serde_json::from_str(preview_line).unwrap();
                    assert_eq!(row["type"], "support_bundle_preview");
                    assert_eq!(row["payload_sha256"], preview.payload_sha256);
                    assert_eq!(row["confirmation_sha256"], preview.confirmation_sha256);
                }
            }
            assert!(prompts.contains(&preview.confirmation_sha256));
            assert!(prompts.ends_with("anything else writes nothing: "));
            // Exactly one private file with exactly the previewed bytes.
            let entries = directory.entries();
            assert_eq!(
                entries,
                [directory
                    .0
                    .join("agentmage-support-bundle-support-bundle-1.json")]
            );
            let bytes = fs::read(&entries[0]).unwrap();
            assert_eq!(sha256_hex(&bytes), preview.payload_sha256);
            assert_eq!(bytes.len() as u64, receipt.payload_bytes);
            assert_eq!(
                fs::metadata(&entries[0]).unwrap().permissions().mode() & 0o777,
                0o600
            );
            let bundle: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(bundle["bundle_kind"], "agentmage.support-bundle.v1");
            assert_eq!(bundle["automatic_upload"], false);
            // The bundle names no path of the directory.
            let text = String::from_utf8(bytes).unwrap();
            assert!(!text.contains(directory.0.to_str().unwrap()));
            let rendered = render_support_bundle_outcome(&outcome, output);
            match output {
                CliOutputFormat::Human => assert_eq!(
                    rendered,
                    format!(
                        "support_bundle published payload_sha256={} destination_sha256={} bytes={} uploaded=false\n",
                        receipt.payload_sha256, receipt.destination_sha256, receipt.payload_bytes
                    )
                ),
                CliOutputFormat::Json => {
                    let row: serde_json::Value = serde_json::from_str(rendered.trim_end()).unwrap();
                    assert_eq!(row["result"], "published");
                    assert_eq!(row["written"], true);
                    assert_eq!(row["uploaded"], false);
                    assert_eq!(row["payload_sha256"], receipt.payload_sha256);
                }
            }
            // The same preview identity is never published twice.
            let (again, _, _) = offer(&directory.0, output, SupportBundleAnswer::Confirmed);
            assert_eq!(
                again,
                SupportBundleOutcome::Refused(DiagnosticExportError::DestinationDenied)
            );
            assert_eq!(directory.entries().len(), 1);
        }
    }

    #[test]
    fn a_declined_cancelled_or_refused_offer_writes_nothing() {
        for output in [CliOutputFormat::Human, CliOutputFormat::Json] {
            for answer in [
                SupportBundleAnswer::Declined,
                SupportBundleAnswer::Cancelled,
            ] {
                let directory = PrivateDirectory::new("declined");
                let (outcome, prompts, asked) = offer(&directory.0, output, answer);
                assert_eq!(asked, 1);
                assert!(matches!(outcome, SupportBundleOutcome::Declined(_)));
                assert!(prompts.contains("support_bundle_preview"));
                assert!(directory.entries().is_empty());
                let rendered = render_support_bundle_outcome(&outcome, output);
                assert!(rendered.contains("declined"));
                assert!(rendered.contains("support-bundle.not-confirmed"));
                match output {
                    CliOutputFormat::Human => assert!(rendered.contains(" written=false ")),
                    CliOutputFormat::Json => {
                        let row: serde_json::Value =
                            serde_json::from_str(rendered.trim_end()).unwrap();
                        assert_eq!(row["written"], false);
                        assert!(row["payload_sha256"].is_null());
                    }
                }
            }
            // A directory others can read, a relative path and a missing
            // directory are refused before any preview or question.
            let open = PrivateDirectory::new("open");
            fs::set_permissions(&open.0, fs::Permissions::from_mode(0o755)).unwrap();
            let missing = open.0.join("missing");
            for directory in [open.0.as_path(), Path::new("relative"), missing.as_path()] {
                let (outcome, prompts, asked) =
                    offer(directory, output, SupportBundleAnswer::Confirmed);
                assert_eq!(
                    outcome,
                    SupportBundleOutcome::Refused(DiagnosticExportError::DestinationDenied)
                );
                assert_eq!((prompts.as_str(), asked), ("", 0));
            }
            assert!(open.entries().is_empty());
            let rendered = render_support_bundle_outcome(
                &SupportBundleOutcome::Refused(DiagnosticExportError::DestinationDenied),
                output,
            );
            assert!(rendered.contains("diagnostic.export.destination-denied"));
            assert!(rendered.contains("refused"));
            let skipped = render_support_bundle_outcome(&SupportBundleOutcome::Skipped, output);
            assert!(skipped.contains("skipped") && skipped.contains("support-bundle.cancelled"));
        }
    }

    #[test]
    fn an_expired_preview_is_not_published() {
        let directory = PrivateDirectory::new("expired");
        let mut prompts = Vec::new();
        let mut times = [10_000, 10_000 + 60_000].into_iter();
        let outcome = offer_support_bundle(
            &served(),
            &directory.0,
            "support-bundle-expired",
            CliOutputFormat::Human,
            &mut prompts,
            &mut || times.next(),
            &mut || SupportBundleAnswer::Confirmed,
        );
        assert!(matches!(
            outcome,
            SupportBundleOutcome::Failed(_, DiagnosticExportError::ApprovalDenied)
        ));
        assert!(directory.entries().is_empty());
    }
}
