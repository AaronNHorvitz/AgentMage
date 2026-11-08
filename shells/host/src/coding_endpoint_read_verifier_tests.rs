// SPDX-License-Identifier: Apache-2.0
//! Direct endpoint framing and synthetic read receipts; no worker or model admission.

use super::*;
use crate::coding_endpoint_read_verifier::*;
use crate::coding_verifier::{
    CodingCompletionCandidate, CodingTerminalClaim, coding_completion_schema,
};
use agentmage_capability_read_only::{
    NeverCancelled, ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyRequest, ReadOnlyResult,
    ReadOnlyToolKind, execute_read_only, read_only_tool_definition,
};
use agentmage_kernel_engine::runtime_loop::endpoint::{
    RuntimeEndpointContextPacket, RuntimeEndpointMessage, RuntimeEndpointMessageRole,
    RuntimeEndpointModelPort, RuntimeEndpointTurns,
};
use agentmage_kernel_engine::runtime_loop::{
    RuntimeClock, RuntimeCompletionClaim, RuntimePortFailure, RuntimeRunScope,
    RuntimeVerificationInput, RuntimeVerifierPort,
};
use sha2::{Digest, Sha256};
use std::time::Duration;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn check(request: &RuntimeEndpointRunRequest, path: &str, text: &str) -> CodingEndpointReadCheck {
    CodingEndpointReadCheck::new(
        WorkspacePath::new(request.workspace_id.clone(), path.split('/')).unwrap(),
        text.len() as u64,
        hash(text.as_bytes()),
    )
    .unwrap()
}
fn bind(f: &mut Fixture, files: &[(&str, &str)]) -> Vec<CodingEndpointReadCheck> {
    let checks = files
        .iter()
        .map(|(path, text)| check(&f.request, path, text))
        .collect::<Vec<_>>();
    let criteria = checks
        .iter()
        .map(CodingEndpointReadCheck::criterion)
        .collect::<Vec<_>>();
    f.request.task.acceptance_criteria = criteria.clone();
    f.request.work_packet.acceptance_checks = criteria;
    f.request = seal_runtime_endpoint_run_request(f.request.clone()).unwrap();
    checks
}
fn read(path: &str, text: &str, offset: Option<u64>, count: Option<u64>) -> ReadOnlyResult {
    let components = path.split('/').map(str::to_owned).collect::<Vec<_>>();
    let request = ReadOnlyRequest {
        schema_version: 1,
        paths: vec![components.clone()],
        query: None,
        byte_offset: offset,
        byte_count: count,
        encoding: ReadOnlyEncoding::Utf8,
        limits: ReadOnlyLimits::default(),
        call_depth: 1,
    };
    execute_read_only(
        ReadOnlyToolKind::ReadText,
        &serde_json::to_vec(&request).unwrap(),
        &WorkspaceSnapshot {
            entries: vec![SnapshotEntry {
                path: components,
                kind: SnapshotEntryKind::RegularFile,
                bytes: text.as_bytes().to_vec(),
                executable: false,
            }],
        },
        &NeverCancelled,
    )
}
fn result(request: &RuntimeEndpointRunRequest, id: &str, read: &ReadOnlyResult) -> ToolResult {
    let bytes = serde_json::to_vec(read).unwrap();
    let definition = read_only_tool_definition(ReadOnlyToolKind::ReadText);
    ToolResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw(id),
        correlation_id: CorrelationId::from_raw("read-verifier-correlation"),
        outcome: OperationOutcome::Succeeded,
        output: Some(ContractPayload {
            schema: definition.output_schema,
            media_type: "application/json".into(),
            sha256: hash(&bytes),
            bytes,
        }),
        validation_issues: vec![],
        evidence: vec![EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(format!("{id}-evidence")),
            kind: EvidenceKind::ToolOutput,
            source_id: format!(
                "native:{}@{}",
                definition.tool_id.as_str(),
                definition.tool_version
            ),
            object_id: id.into(),
            fragment: None,
            content_sha256: read.result_sha256.clone(),
            observed_revision: Some(request.repository_snapshot_id.as_str().into()),
        }],
        error: None,
        elapsed_ms: 1,
        state_change: StateChange::NotChanged,
    }
}
struct Clock;
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(1000)
    }
}
struct Port<'a> {
    request: EndpointModelRequest,
    text: &'a str,
}
impl RuntimeEndpointModelPort for Port<'_> {
    type Error = RuntimePortFailure;
    fn request(&self) -> &EndpointModelRequest {
        &self.request
    }
    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        deliver: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, Self::Error> {
        control
            .remaining_ms()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        deliver(self.text);
        Ok(EndpointModelResult::from_completed_response(
            self.request,
            EndpointModelFinish::Stop,
            hash(self.text.as_bytes()),
            self.text.len(),
            Duration::from_millis(1),
            None,
        ))
    }
}
fn candidate(request: &RuntimeEndpointRunRequest) -> CodingCompletionCandidate {
    CodingCompletionCandidate {
        schema_version: 1,
        objective_sha256: hash(request.task.objective.as_bytes()),
        terminal_claim: CodingTerminalClaim::NoOp,
        summary: "The exact selected file observations are complete.".into(),
        checks_not_run: vec![],
        residual_risks: vec![],
    }
}
fn verify(
    verifier: &mut CodingEndpointReadVerifier,
    request: &RuntimeEndpointRunRequest,
    candidate: &CodingCompletionCandidate,
    results: &[ToolResult],
    evidence: &[EvidenceReference],
    receipts: &[ReceiptId],
    postconditions: &[PostconditionId],
) -> Result<VerifierCandidate, RuntimePortFailure> {
    let text = serde_json::to_string(candidate).unwrap();
    let context = RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
        role: RuntimeEndpointMessageRole::User,
        content: "Synthetic endpoint completion; not real inference.",
    }])
    .unwrap();
    let model_request = EndpointModelRequest::from_prepared_body(
        request.model_selection.clone(),
        context.messages().clone(),
        EndpointModelBody {
            request_body_bytes: 200,
            request_body_sha256: "b".repeat(64),
        },
        EndpointModelLimits {
            requested_max_output_tokens: 16,
            provider_usage_requested: false,
            total_timeout: Duration::from_millis(100),
            idle_timeout: Duration::from_millis(50),
            response_wire_limit_bytes: 8192,
            response_body_limit_bytes: 8192,
            response_text_limit_bytes: 4096,
        },
    );
    let mut turns = RuntimeEndpointTurns::new(request.clone(), Clock).unwrap();
    let turn = turns
        .execute_turn(
            &model_request,
            context,
            Port {
                request: model_request.clone(),
                text: &text,
            },
            None,
            |_| {},
        )
        .unwrap();
    let scope = RuntimeRunScope::endpoint(request).unwrap();
    verifier.verify_completion(RuntimeVerificationInput {
        request: scope,
        proposal: RuntimeCompletionClaim::endpoint(scope, &turn, &text, coding_completion_schema())
            .unwrap(),
        state_revision: 8,
        postconditions,
        evidence,
        tool_results: results,
        receipt_ids: receipts,
    })
}
fn postconditions(request: &RuntimeEndpointRunRequest) -> Vec<PostconditionId> {
    request
        .task
        .acceptance_criteria
        .iter()
        .enumerate()
        .map(|(index, criterion)| {
            PostconditionId::from_raw(format!(
                "postcondition:{index}:{}",
                &hash(criterion.as_bytes())[..16]
            ))
        })
        .collect()
}
fn evidence(results: &[ToolResult]) -> Vec<EvidenceReference> {
    results.iter().flat_map(|r| r.evidence.clone()).collect()
}
fn receipts(count: usize) -> Vec<ReceiptId> {
    (0..count)
        .map(|i| ReceiptId::from_raw(format!("read-receipt-{i}")))
        .collect()
}
fn assert_failed(result: VerifierCandidate) {
    assert_eq!(result.disposition, VerifierDisposition::Failed);
    assert!(result.postconditions.iter().any(|p| !p.passed));
    assert!(
        result
            .postconditions
            .iter()
            .filter(|p| !p.passed)
            .all(|p| p.evidence.is_empty())
    );
}

#[test]
fn endpoint_read_verifier_binds_only_exact_supported_checks_after_narrowing() {
    let mut f = Fixture::new();
    let checks = bind(&mut f, &[("src/lib.rs", "synthetic é\n")]);
    let prepared = f.prepare(&f.empty_ledger()).unwrap();
    let verifier = prepared.read_verifier(checks.clone()).unwrap();
    assert_eq!(
        verifier.verifier_id().as_str(),
        format!(
            "endpoint-read-verifier:{}",
            &prepared.request().request_sha256[..24]
        )
    );
    let new = |r: &RuntimeEndpointRunRequest, checks| {
        CodingEndpointReadVerifier::for_request(r, checks).err()
    };
    assert_eq!(
        new(prepared.request(), vec![]),
        Some(RuntimePortFailure::Invalid)
    );
    assert_eq!(
        new(
            prepared.request(),
            vec![checks[0].clone(), checks[0].clone()]
        ),
        Some(RuntimePortFailure::Invalid)
    );
    for mutation in 0..9 {
        let mut request = prepared.request().clone();
        match mutation {
            0 => request.task.acceptance_criteria[0] = "The code is correct".into(),
            1 => request.work_packet.acceptance_checks[0].push_str(" extra"),
            2 => request
                .work_packet
                .required_evidence
                .push(EvidenceKind::Validation),
            3 => request.work_packet.mutable_files.push("src/lib.rs".into()),
            4 => request.mode = RuntimeSessionMode::DurableReadOnly,
            5 => request.limits.max_tool_calls = 0,
            6 => request.visible_tools[0].tool_version = "9.0.0".into(),
            7 => request.task.objective.push_str(" changed"),
            8 => request.work_packet.required_capability_class = AuthorityClass::Execute,
            _ => unreachable!(),
        }
        if mutation == 7 {
            assert_eq!(
                new(&request, checks.clone()),
                Some(RuntimePortFailure::Invalid)
            );
        } else {
            request.tool_catalog_sha256 =
                runtime_tool_catalog_sha256(&request.tool_catalog_id, &request.visible_tools)
                    .unwrap();
            let request = seal_runtime_endpoint_run_request(request).unwrap();
            assert_eq!(
                new(&request, checks.clone()),
                Some(RuntimePortFailure::Invalid),
                "mutation {mutation}"
            );
        }
    }
    let mut foreign = checks.clone();
    foreign[0] = CodingEndpointReadCheck::new(
        WorkspacePath::new(WorkspaceId::from_raw("other-workspace"), ["src", "lib.rs"]).unwrap(),
        13,
        hash("synthetic é\n".as_bytes()),
    )
    .unwrap();
    assert_eq!(
        new(prepared.request(), foreign),
        Some(RuntimePortFailure::Invalid)
    );
    let private = bind(&mut f, &[("private/secret", "confidential")]);
    let ledger = f.ledger(vec![constraint(
        GuidanceConstraintKind::ExcludeScope,
        "private",
    )]);
    assert_eq!(
        f.prepare(&ledger).unwrap().read_verifier(private).err(),
        Some(Error::Acceptance)
    );
    let path = checks[0].path().clone();
    for digest in ["a".repeat(63), "A".repeat(64), "g".repeat(64)] {
        assert!(CodingEndpointReadCheck::new(path.clone(), 0, digest).is_err());
    }
    assert!(CodingEndpointReadCheck::new(path, 2 * 1024 * 1024 + 1, "a".repeat(64)).is_err());
    assert_eq!(
        fs::read(f.root.join("sentinel")).unwrap(),
        b"unchanged synthetic source"
    );
}

#[test]
fn endpoint_read_verifier_proves_full_utf8_and_empty_files_with_per_check_evidence() {
    let mut f = Fixture::new();
    let files = [("src/lib.rs", "synthetic é\n"), ("empty", "")];
    let checks = bind(&mut f, &files);
    let prepared = f.prepare(&f.empty_ledger()).unwrap();
    let request = prepared.request();
    let results = files
        .iter()
        .enumerate()
        .map(|(i, (p, t))| result(request, &format!("read-{i}"), &read(p, t, None, None)))
        .collect::<Vec<_>>();
    for r in &results {
        assert!(
            serde_json::from_slice::<ReadOnlyResult>(&r.output.as_ref().unwrap().bytes)
                .unwrap()
                .verify(ReadOnlyToolKind::ReadText)
        );
    }
    let mut verifier = prepared.read_verifier(checks).unwrap();
    let proof = verify(
        &mut verifier,
        request,
        &candidate(request),
        &results,
        &evidence(&results),
        &receipts(2),
        &postconditions(request),
    )
    .unwrap();
    assert_eq!(proof.disposition, VerifierDisposition::NoOp);
    assert!(proof.postconditions.iter().all(|p| p.passed));
    for (p, r) in proof.postconditions.iter().zip(&results) {
        assert_eq!(p.evidence, r.evidence);
    }
    // A real observation for one file cannot satisfy the other postcondition.
    let partial = verify(
        &mut verifier,
        request,
        &candidate(request),
        &results[..1],
        &evidence(&results[..1]),
        &receipts(1),
        &postconditions(request),
    )
    .unwrap();
    assert_eq!(partial.disposition, VerifierDisposition::Failed);
    assert!(partial.postconditions[0].passed);
    assert!(!partial.postconditions[1].passed);
    // Keep both valid observations/evidence while changing only receipt identity.
    assert_failed(
        verify(
            &mut verifier,
            request,
            &candidate(request),
            &results,
            &evidence(&results),
            &[ReceiptId::from_raw("same"), ReceiptId::from_raw("same")],
            &postconditions(request),
        )
        .unwrap(),
    );
    // Keep both checks satisfied while repeating a call identity, with otherwise
    // distinct, correctly bound evidence. This isolates duplicate-call refusal.
    let mut repeated = results.clone();
    repeated[1].tool_call_id = repeated[0].tool_call_id.clone();
    repeated[1].evidence[0].object_id = repeated[0].tool_call_id.as_str().into();
    assert_failed(
        verify(
            &mut verifier,
            request,
            &candidate(request),
            &repeated,
            &evidence(&repeated),
            &receipts(2),
            &postconditions(request),
        )
        .unwrap(),
    );
}

#[test]
fn endpoint_read_verifier_refuses_receipt_evidence_and_output_substitution() {
    let mut f = Fixture::new();
    let checks = bind(&mut f, &[("src/lib.rs", "selected source")]);
    let prepared = f.prepare(&f.empty_ledger()).unwrap();
    let request = prepared.request();
    let good = result(
        request,
        "read-0",
        &read("src/lib.rs", "selected source", None, None),
    );
    let mut verifier = prepared.read_verifier(checks).unwrap();
    for mutation in 0..18 {
        let mut results = vec![good.clone()];
        let mut ledger = evidence(&results);
        let mut receipts = receipts(1);
        match mutation {
            0 => receipts.clear(),
            1 => receipts[0] = ReceiptId::from_raw(""),
            2 => ledger.clear(),
            3 => ledger[0].observed_revision = Some("stale-snapshot".into()),
            4 => results[0].output.as_mut().unwrap().schema.schema_sha256 = "d".repeat(64),
            5 => results[0].output.as_mut().unwrap().sha256 = "e".repeat(64),
            6 => results[0].state_change = StateChange::Changed,
            7 => results[0].state_change = StateChange::Uncertain,
            8 => results[0].outcome = OperationOutcome::Cancelled,
            9 => results[0].evidence[0].source_id = "untrusted".into(),
            10 => results[0].evidence[0].object_id = "other-call".into(),
            11 => results[0].evidence[0].content_sha256 = "f".repeat(64),
            12 => results[0].output.as_mut().unwrap().media_type = "text/plain".into(),
            13 => {
                results[0].output.as_mut().unwrap().bytes = b"{}".to_vec();
                results[0].output.as_mut().unwrap().sha256 = hash(b"{}");
            }
            14 => {
                results.push(good.clone());
                ledger = evidence(&results);
                receipts = vec![receipts[0].clone(); 2];
            }
            15 => {
                let mut second = good.clone();
                second.tool_call_id = ToolCallId::from_raw("second-read");
                results.push(second);
                ledger = evidence(&results);
                receipts = self::receipts(2);
            }
            16 => results[0].output = None,
            17 => {
                let mut extra = ledger[0].clone();
                extra.evidence_id = EvidenceId::from_raw("foreign-evidence");
                ledger.push(extra);
            }
            _ => unreachable!(),
        }
        assert_failed(
            verify(
                &mut verifier,
                request,
                &candidate(request),
                &results,
                &ledger,
                &receipts,
                &postconditions(request),
            )
            .unwrap(),
        );
    }
    assert_eq!(
        verify(
            &mut verifier,
            request,
            &candidate(request),
            std::slice::from_ref(&good),
            &evidence(std::slice::from_ref(&good)),
            &receipts(1),
            &postconditions(request)
        )
        .unwrap()
        .disposition,
        VerifierDisposition::NoOp
    );
}

#[test]
fn endpoint_read_verifier_refuses_partial_wrong_and_conflicting_file_observations() {
    let mut f = Fixture::new();
    let checks = bind(&mut f, &[("src/lib.rs", "selected source")]);
    let prepared = f.prepare(&f.empty_ledger()).unwrap();
    let request = prepared.request();
    let mut verifier = prepared.read_verifier(checks).unwrap();
    for read in [
        read("src/lib.rs", "selected source", Some(1), None),
        read("src/lib.rs", "selected source", None, Some(3)),
        read("src/lib.rs", "changed source", None, None),
        read("other.rs", "selected source", None, None),
    ] {
        assert!(read.verify(ReadOnlyToolKind::ReadText));
        let results = vec![result(request, "read-0", &read)];
        assert_failed(
            verify(
                &mut verifier,
                request,
                &candidate(request),
                &results,
                &evidence(&results),
                &receipts(1),
                &postconditions(request),
            )
            .unwrap(),
        );
    }
    let results = vec![
        result(
            request,
            "read-0",
            &read("src/lib.rs", "selected source", None, None),
        ),
        result(
            request,
            "read-1",
            &read("src/lib.rs", "changed source", None, None),
        ),
    ];
    assert_failed(
        verify(
            &mut verifier,
            request,
            &candidate(request),
            &results,
            &evidence(&results),
            &receipts(2),
            &postconditions(request),
        )
        .unwrap(),
    );
    assert_failed(
        verify(
            &mut verifier,
            request,
            &candidate(request),
            &[],
            &[],
            &[],
            &postconditions(request),
        )
        .unwrap(),
    );
}

#[test]
fn endpoint_read_verifier_refuses_false_claims_foreign_runs_and_postconditions() {
    let mut f = Fixture::new();
    let checks = bind(&mut f, &[("src/lib.rs", "selected source")]);
    let prepared = f.prepare(&f.empty_ledger()).unwrap();
    let request = prepared.request();
    let results = [result(
        request,
        "read-0",
        &read("src/lib.rs", "selected source", None, None),
    )];
    let mut verifier = prepared.read_verifier(checks).unwrap();
    for mutation in 0..5 {
        let mut claim = candidate(request);
        match mutation {
            0 => claim.terminal_claim = CodingTerminalClaim::Changed,
            1 => claim.objective_sha256 = "a".repeat(64),
            2 => claim.checks_not_run.push("selected read".into()),
            3 => claim.schema_version = 2,
            4 => claim.summary.clear(),
            _ => unreachable!(),
        }
        assert_failed(
            verify(
                &mut verifier,
                request,
                &claim,
                &results,
                &evidence(&results),
                &receipts(1),
                &postconditions(request),
            )
            .unwrap(),
        );
    }
    for mutation in 0..5 {
        let mut foreign = request.clone();
        match mutation {
            0 => foreign.run_id = RuntimeRunId::from_raw("foreign-run"),
            1 => foreign.workspace_snapshot_sha256 = hash(b"foreign workspace snapshot"),
            2 => foreign.repository_snapshot_sha256 = hash(b"foreign repository snapshot"),
            3 => foreign.policy_sha256 = hash(b"foreign policy"),
            4 => foreign.model_selection.model.push_str("-foreign"),
            _ => unreachable!(),
        }
        let foreign = seal_runtime_endpoint_run_request(foreign).unwrap();
        assert_ne!(
            foreign.request_sha256, request.request_sha256,
            "mutation must change request"
        );
        assert_eq!(
            verify(
                &mut verifier,
                &foreign,
                &candidate(&foreign),
                &results,
                &evidence(&results),
                &receipts(1),
                &postconditions(&foreign)
            ),
            Err(RuntimePortFailure::Invalid)
        );
    }
    assert_eq!(
        verify(
            &mut verifier,
            request,
            &candidate(request),
            &results,
            &evidence(&results),
            &receipts(1),
            &[PostconditionId::from_raw("foreign-postcondition")]
        ),
        Err(RuntimePortFailure::Invalid)
    );
}
