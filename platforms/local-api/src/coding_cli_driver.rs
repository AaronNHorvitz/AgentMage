// SPDX-License-Identifier: Apache-2.0
//! HTTP transport for the existing verified interactive driver. No agent loop.

use super::*;
use agentmage_host::{
    cli_runtime::{
        InteractiveCliCancellationPort, InteractiveCliRuntimeError, drive_interactive_cli_runtime,
    },
    coding_client::{CodingApprovalPort, CodingClientError, CodingEventSink, JobControlNotice},
    runtime_transport::{
        RuntimeJobControl, RuntimeJobStatus, RuntimePrepareInput, RuntimeRunDeclarations,
        RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
    },
};
use agentmage_kernel_contracts::{
    CancellationId, RuntimeApprovalChallenge, RuntimeApprovalDisposition, RuntimeApprovalResponse,
    RuntimeArtifactRef, RuntimeEvent, RuntimeEventCursor, RuntimeRunId, RuntimeRunRequest,
};
use agentmage_kernel_engine::{
    job_control::JobControlRequest, runtime_artifact::RuntimeArtifactPage,
};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

type TransportResult<T> = std::result::Result<T, RuntimeTransportError>;

struct Signals {
    stop: Arc<AtomicBool>,
    registered: Vec<signal_hook::SigId>,
    started: Instant,
    limit: Duration,
}
impl Signals {
    fn new(seconds: u64) -> Result<Self> {
        let mut signals = Self {
            stop: Arc::new(AtomicBool::new(false)),
            registered: Vec::new(),
            started: Instant::now(),
            limit: Duration::from_secs(seconds),
        };
        for signal in [
            signal_hook::consts::SIGINT,
            signal_hook::consts::SIGTERM,
            signal_hook::consts::SIGHUP,
        ] {
            signals
                .registered
                .push(signal_hook::flag::register(signal, signals.stop.clone()).map_err(|_| ())?);
        }
        Ok(signals)
    }
    fn confirm(&self, review: &Value, challenge: &str) -> Result<()> {
        if !std::io::stdin().is_terminal() || self.stopped() {
            return Err(());
        }
        let mut tty = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(
                rustix::fs::OFlags::NONBLOCK
                    .bits()
                    .try_into()
                    .map_err(|_| ())?,
            )
            .open("/dev/tty")
            .map_err(|_| ())?;
        writeln!(
            tty,
            "{}\nType exactly {challenge} to approve; anything else declines:",
            safe(review)?
        )
        .and_then(|()| tty.flush())
        .map_err(|_| ())?;
        let mut line = Vec::new();
        loop {
            if self.stopped() || line.len() == 256 {
                return Err(());
            }
            let mut byte = [0];
            match tty.read(&mut byte) {
                Ok(1) if byte[0] == b'\n' => break,
                Ok(1) => line.push(byte[0]),
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => return Err(()),
            }
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        if self.stopped() || line != challenge.as_bytes() {
            return Err(());
        }
        Ok(())
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed) || self.started.elapsed() >= self.limit
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        for signal in self.registered.drain(..) {
            signal_hook::low_level::unregister(signal);
        }
    }
}
struct Controls<'a> {
    signals: &'a Signals,
    sent: bool,
}
impl InteractiveCliCancellationPort for Controls<'_> {
    fn poll(
        &mut self,
        _: &RuntimeRunRequest,
    ) -> std::result::Result<Option<CancellationId>, InteractiveCliRuntimeError> {
        if !self.signals.stopped() {
            return Ok(None);
        }
        if self.sent {
            return Err(InteractiveCliRuntimeError::Cancellation);
        }
        self.sent = true;
        Ok(Some(CancellationId::from_raw("terminal-stop")))
    }
}

struct Presentation<'a>(&'a Signals);
impl CodingEventSink for Presentation<'_> {
    fn present(&mut self, event: &RuntimeEvent) -> std::result::Result<(), CodingClientError> {
        show(&json!({"type":"verified-event","event":event}))
            .map_err(|_| CodingClientError::Presentation)
    }
    fn present_job_control(
        &mut self,
        notice: JobControlNotice<'_>,
    ) -> std::result::Result<(), CodingClientError> {
        let value = match notice {
            JobControlNotice::Answered { action, answer } => {
                json!({"notice":"answered","action":action,"answer":answer})
            }
            JobControlNotice::NotTaken { action } => json!({"notice":"not-taken","action":action}),
            JobControlNotice::Suspended(point) => json!({"notice":"suspended","point":point}),
            JobControlNotice::Continued(point) => json!({"notice":"continued","point":point}),
            JobControlNotice::Released { action, point } => {
                json!({"notice":"released","action":action,"point":point})
            }
        };
        show(&json!({"type":"verified-job-control","control":value}))
            .map_err(|_| CodingClientError::Presentation)
    }
}
impl CodingApprovalPort for Presentation<'_> {
    fn decide(
        &mut self,
        challenge: &RuntimeApprovalChallenge,
    ) -> std::result::Result<RuntimeApprovalDisposition, CodingClientError> {
        // The shared driver has already checked the challenge against this run
        // and its canonical permission event. Only a fresh typed answer allows.
        // Cancellation is checked again by the driver before dispatch.
        if self.0.stopped() {
            return Ok(RuntimeApprovalDisposition::Deny);
        }
        let arguments: Value = serde_json::from_slice(&challenge.presentation.arguments.bytes)
            .map_err(|_| CodingClientError::Approval)?;
        if self.0.confirm(&json!({"challenge":challenge,"arguments_json":arguments,
            "warning":"Approves only this exact host-bound operation. Every later operation needs a new decision. A lost reply is uncertain; inspect the run before taking another action."}),
            &format!("allow {}", challenge.challenge_sha256)).is_ok() && !self.0.stopped() {
            Ok(RuntimeApprovalDisposition::Allow)
        } else {
            Ok(RuntimeApprovalDisposition::Deny)
        }
    }
}

struct HttpRuntime<'a> {
    client: &'a Client,
    selection: Value,
    controls: &'a Signals,
}
impl HttpRuntime<'_> {
    fn post<T: serde::de::DeserializeOwned + serde::Serialize>(
        &self,
        route: &str,
        body: Value,
    ) -> TransportResult<T> {
        let value = self
            .client
            .json("POST", route, true, Some(body))
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        let bytes =
            serde_json::to_vec(&value).map_err(|_| RuntimeTransportError::RuntimeEvidenceDenied)?;
        agentmage_host::runtime_transport::decode_exact(&bytes)
            .ok_or(RuntimeTransportError::RuntimeEvidenceDenied)
    }
    fn consent(&self, review: &Value, text: &str) -> TransportResult<()> {
        if self.controls.stopped() {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.controls
            .confirm(review, text)
            .map_err(|_| RuntimeTransportError::RequestDenied)?;
        if self.controls.stopped() {
            return Err(RuntimeTransportError::RequestDenied);
        }
        Ok(())
    }
}
impl RuntimeTransportPort for HttpRuntime<'_> {
    fn prepare(&mut self, input: RuntimePrepareInput) -> TransportResult<RuntimeRunRequest> {
        if input.record_session
            || input.slow_subscriber_probe
            || input.preauthorization.is_some()
            || input.engineering_session_id.is_some()
            || input.recipe.is_some()
            || self.selection["profile_id"] != input.profile_id
            || self.selection["expected_entry_sha256"] != input.expected_entry_sha256
            || self.selection["workspace_id"] != input.workspace_id
            || self.selection["workspace_root"] != input.workspace_root
            || self.selection["resume"] != input.resume
        {
            return Err(RuntimeTransportError::RequestDenied);
        }
        self.consent(&json!({"selection":self.selection,"task":input.prompt,
            "warning":"Inspects the startup-selected disposable repository. This is the fixed failed-test-repair scenario, not a general repository task. No model or tool approval is granted."}),
            &format!("inspect {}", input.expected_entry_sha256))?;
        self.post(
            "/v1/code/runtime/prepare",
            json!({"prompt":input.prompt,"approve_inspection":true}),
        )
    }
    fn start(&mut self, request: RuntimeRunRequest) -> TransportResult<RuntimeTransportStep> {
        self.consent(&json!({"request":request,
            "warning":"Sends this task, repository context and subsequent observations to the startup-selected native evaluation model. Not the configured chat endpoint. Every tool still needs fresh exact approval."}),
            &format!("start {}", request.request_sha256))?;
        show(&json!({"type":"run-prepared","run_id":request.run_id,"request_sha256":request.request_sha256})).map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        self.post(
            "/v1/code/runtime/start",
            json!({"request":request,"approve_model_context":true}),
        )
    }
    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> TransportResult<RuntimeTransportStep> {
        self.post("/v1/code/runtime/advance", json!({"run_id":run_id,"request_sha256":request_sha256,"after_event_cursor":after_event_cursor,"response":response}))
    }
    fn cancel(
        &mut self,
        _: &RuntimeRunId,
        _: &str,
        _: CancellationId,
        _: Option<&RuntimeEventCursor>,
    ) -> TransportResult<RuntimeTransportStep> {
        // The shared driver uses revision-bound canonical job control when it
        // is offered. There is no generic cancellation authority on this API.
        Err(RuntimeTransportError::RequestDenied)
    }
    fn read_artifact_page(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
        offset: u64,
        maximum_bytes: u32,
    ) -> TransportResult<RuntimeArtifactPage> {
        self.post("/v1/code/runtime/artifact", json!({"run_id":run_id,"request_sha256":request_sha256,"reference":reference,"offset":offset,"maximum_bytes":maximum_bytes}))
    }
    fn run_declarations(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> TransportResult<RuntimeRunDeclarations> {
        self.post(
            "/v1/code/runtime/declarations",
            json!({"run_id":run_id,"request_sha256":request_sha256}),
        )
    }
    fn job_status(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> TransportResult<RuntimeJobStatus> {
        self.post(
            "/v1/code/runtime/job",
            json!({"run_id":run_id,"request_sha256":request_sha256}),
        )
    }
    fn control_job(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        request: &JobControlRequest,
    ) -> TransportResult<RuntimeJobControl> {
        self.post(
            "/v1/code/runtime/control",
            json!({"run_id":run_id,"request_sha256":request_sha256,"request":request}),
        )
    }
    fn release(&mut self, run_id: &RuntimeRunId, request_sha256: &str) -> TransportResult<()> {
        let reply = self
            .client
            .json(
                "POST",
                "/v1/code/runtime/release",
                true,
                Some(json!({"run_id":run_id,"request_sha256":request_sha256})),
            )
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        if reply != json!({"released":true}) {
            return Err(RuntimeTransportError::RuntimeEvidenceDenied);
        }
        Ok(())
    }
}

pub(super) fn run(client: &Client, seconds: &str, task: &str) -> Result<()> {
    // Do not inspect or prepare a run when no person can answer its decisions.
    if !std::io::stdin().is_terminal() {
        return Err(());
    }
    let seconds = seconds.parse::<u64>().map_err(|_| ())?;
    if !(1..=3600).contains(&seconds) || task.is_empty() || task.len() > 16 * 1024 {
        return Err(());
    }
    let controls = Signals::new(seconds)?;
    let selection = client.json("GET", "/v1/code/runtime", true, None)?;
    if selection["version"] != 1
        || selection["activation"] != "coding-development-v1"
        || selection["scenario"] != "failed-test-repair"
        || selection["chat_endpoint_used"] != false
        || selection["requires_control"] != true
        || selection["preauthorization"] != false
    {
        return Err(());
    }
    let string = |key| selection[key].as_str().map(str::to_owned).ok_or(());
    let entry = string("expected_entry_sha256")?;
    if !digest(&entry) {
        return Err(());
    }
    let input = RuntimePrepareInput {
        resume: selection["resume"].as_bool().ok_or(())?,
        record_session: false,
        slow_subscriber_probe: false,
        preauthorization: None,
        engineering_session_id: None,
        recipe: None,
        profile_id: string("profile_id")?,
        expected_entry_sha256: entry,
        workspace_id: string("workspace_id")?,
        workspace_root: string("workspace_root")?,
        prompt: task.to_owned(),
    };
    let mut runtime = HttpRuntime {
        client,
        selection,
        controls: &controls,
    };
    let result = drive_interactive_cli_runtime(
        &mut runtime,
        input,
        &mut Presentation(&controls),
        &mut Presentation(&controls),
        &mut Controls {
            signals: &controls,
            sent: false,
        },
    )
    .map_err(|e| {
        eprintln!(
            "{}; inspect canonical run state before further action",
            e.code()
        );
    })?;
    let succeeded = result.outcome.state.is_success();
    show(
        &json!({"type":"verified-outcome","request":result.request,"outcome":result.outcome,
        "artifacts":result.artifacts,"presented_events":result.presented_events,
        "verified_artifact_count":result.verified_artifacts.len(),"declarations":result.declarations,
        "job":result.job.as_ref().ok(),"released":true,"native_workflow_accepted":false,
        "job_controls":result.job_controls.iter().map(|control|json!({"action":control.action,"answer":control.answer})).collect::<Vec<_>>()}),
    )?;
    if succeeded { Ok(()) } else { Err(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_is_latched_and_cannot_resend_cancellation_after_a_nonterminal_answer() {
        let signals = Signals {
            stop: Arc::new(AtomicBool::new(false)),
            registered: Vec::new(),
            started: Instant::now(),
            limit: Duration::from_secs(30),
        };
        let request: RuntimeRunRequest = agentmage_host::runtime_transport::decode_exact(
            include_bytes!("../../../fixtures/runtime-producer/v5/run-request.json"),
        )
        .unwrap();
        let mut controls = Controls {
            signals: &signals,
            sent: false,
        };
        assert_eq!(controls.poll(&request).unwrap(), None);
        signals.stop.store(true, Ordering::Relaxed);
        assert_eq!(
            controls.poll(&request).unwrap(),
            Some(CancellationId::from_raw("terminal-stop"))
        );
        assert_eq!(
            controls.poll(&request),
            Err(InteractiveCliRuntimeError::Cancellation)
        );
    }
}
