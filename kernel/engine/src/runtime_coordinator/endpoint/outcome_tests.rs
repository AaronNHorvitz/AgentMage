// SPDX-License-Identifier: Apache-2.0
//! Synthetic accepted-turn/outcome framing; never a coding score or task proof.

use agentmage_kernel_contracts::{
    AgentStateKind, EvidenceId, EvidenceKind, RuntimeEventId, SchemaId, SchemaReference,
};

use super::*;
use crate::runtime_loop::{
    RuntimeClock, RuntimeCompletionClaim, RuntimeCompletionProposal, RuntimeRunScope,
    endpoint::{RuntimeEndpointTurnResult, RuntimeEndpointTurns},
};

struct Clock;
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(1000)
    }
}
struct Port(EndpointModelRequest);
impl RuntimeEndpointModelPort for Port {
    type Error = ();
    fn request(&self) -> &EndpointModelRequest {
        &self.0
    }
    fn dispatch(
        self,
        _: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, ()> {
        text("{}");
        Ok(EndpointModelResult::from_completed_response(
            self.0,
            EndpointModelFinish::Stop,
            sha256(b"{}"),
            2,
            Duration::from_millis(1),
            None,
        ))
    }
}
fn accepted(request: &RuntimeEndpointRunRequest) -> RuntimeEndpointTurnResult {
    let mut turns = RuntimeEndpointTurns::new(request.clone(), Clock).unwrap();
    let prepared = prepared();
    turns
        .execute_turn(&prepared, context(), Port(prepared.clone()), None, |_| {})
        .unwrap()
}
fn claim<'a>(
    request: &RuntimeEndpointRunRequest,
    turn: &'a RuntimeEndpointTurnResult,
) -> RuntimeCompletionClaim<'a> {
    RuntimeCompletionClaim::endpoint(
        RuntimeRunScope::endpoint(request).unwrap(),
        turn,
        "{}",
        SchemaReference {
            schema_id: SchemaId::from_raw("runtime.synthetic-completion"),
            schema_version: 1,
            schema_sha256: "a".repeat(64),
        },
    )
    .unwrap()
}
fn outcome(
    request: &RuntimeEndpointRunRequest,
    claim: &RuntimeCompletionClaim<'_>,
) -> RuntimeOutcome {
    RuntimeOutcome {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: request.run_id.clone(),
        session_id: request.session_id.clone(),
        task_id: request.task.task_id.clone(),
        request_sha256: request.request_sha256.clone(),
        state: AgentStateKind::Success,
        turn_count: 1,
        model_call_count: 1,
        tool_call_count: 0,
        prior_event_id: RuntimeEventId::from_raw("event-before-terminal"),
        prior_event_sha256: "d".repeat(64),
        evidence: vec![EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw("evidence-0001"),
            kind: EvidenceKind::Observation,
            source_id: "fixture".into(),
            object_id: "fixture/input.txt".into(),
            fragment: Some("line:1".into()),
            content_sha256: "a".repeat(64),
            observed_revision: Some("snapshot-0001".into()),
        }],
        receipt_ids: vec![],
        unresolved_codes: vec![],
        output: Some(RuntimeOutput::Inline {
            payload: claim.payload().unwrap().clone(),
        }),
        answer_evidence: None,
        outcome_sha256: "0".repeat(64),
    }
}

#[test]
fn endpoint_outcome_binds_whole_envelope_to_accepted_claim_without_native_provenance() {
    let request = seal_runtime_endpoint_run_request(request()).unwrap();
    let turn = accepted(&request);
    let claim = claim(&request, &turn);
    let raw = outcome(&request, &claim);
    let envelope = seal_runtime_endpoint_outcome(raw.clone(), &request, Some(&claim)).unwrap();
    assert_eq!(
        verify_runtime_endpoint_outcome(&envelope, &request, Some(&claim)),
        Ok(())
    );
    assert!(envelope.outcome.answer_evidence.is_none());
    assert_eq!(
        envelope
            .answer_evidence
            .as_ref()
            .unwrap()
            .completion_claim_sha256,
        claim.proposal_sha256()
    );
    let mut preimage = envelope.clone();
    preimage.outcome.outcome_sha256 = ZERO_SHA256.into();
    assert_eq!(
        canonical_sha256(&preimage).unwrap(),
        envelope.outcome.outcome_sha256
    );
    assert_ne!(
        canonical_sha256(&preimage.outcome).unwrap(),
        envelope.outcome.outcome_sha256
    );
    assert_eq!(
        seal_runtime_endpoint_outcome(raw.clone(), &request, Some(&claim)).unwrap(),
        envelope
    );
    assert_eq!(
        seal_runtime_endpoint_outcome(raw, &request, None),
        Err(RuntimeCoordinatorError::OutcomeDenied)
    );
    assert_eq!(
        verify_runtime_endpoint_outcome(&envelope, &request, None),
        Err(RuntimeCoordinatorError::OutcomeDenied)
    );
    let native = seal_runtime_run_request(super::super::super::tests::request()).unwrap();
    assert!(seal_runtime_outcome(envelope.outcome.clone(), &native).is_err());
    assert!(verify_runtime_outcome(&envelope.outcome, &native).is_err());
}

#[test]
fn endpoint_outcome_refuses_foreign_run_claim_and_resealed_request_revision() {
    let original = seal_runtime_endpoint_run_request(request()).unwrap();
    let turn = accepted(&original);
    let claim_a = claim(&original, &turn);
    let raw = outcome(&original, &claim_a);
    let envelope = seal_runtime_endpoint_outcome(raw.clone(), &original, Some(&claim_a)).unwrap();
    let mut foreign = original.clone();
    foreign.run_id = RuntimeRunId::from_raw("another-endpoint-run");
    let foreign = seal_runtime_endpoint_run_request(foreign).unwrap();
    let turn_b = accepted(&foreign);
    let claim_b = claim(&foreign, &turn_b);
    assert_eq!(
        seal_runtime_endpoint_outcome(raw, &original, Some(&claim_b)),
        Err(RuntimeCoordinatorError::OutcomeDenied)
    );
    assert_eq!(
        verify_runtime_endpoint_outcome(&envelope, &original, Some(&claim_b)),
        Err(RuntimeCoordinatorError::OutcomeDenied)
    );
    for changed in ["policy", "objective", "model"] {
        let mut revised = original.clone();
        match changed {
            "policy" => revised.policy_sha256 = "e".repeat(64),
            "objective" => revised.task.objective.push_str(" revised"),
            "model" => revised.model_selection.model = "another-model".into(),
            _ => unreachable!(),
        }
        let revised = seal_runtime_endpoint_run_request(revised).unwrap();
        let mut raw = envelope.outcome.clone();
        raw.request_sha256 = revised.request_sha256.clone();
        assert_eq!(
            seal_runtime_endpoint_outcome(raw, &revised, Some(&claim_a)),
            Err(RuntimeCoordinatorError::OutcomeDenied)
        );
        assert!(verify_runtime_endpoint_outcome(&envelope, &revised, Some(&claim_a)).is_err());
    }
}

#[test]
fn endpoint_outcome_refuses_payload_evidence_and_shared_terminal_shape_tampering() {
    let request = seal_runtime_endpoint_run_request(request()).unwrap();
    let turn = accepted(&request);
    let claim = claim(&request, &turn);
    let envelope =
        seal_runtime_endpoint_outcome(outcome(&request, &claim), &request, Some(&claim)).unwrap();
    let original = serde_json::to_value(&envelope).unwrap();
    // Shape admission precedes claim processing and citation hashing. Even
    // with no claim, an invalid owning run must fail at the shared preflight.
    let mut wrong_run = envelope.outcome.clone();
    wrong_run.run_id = RuntimeRunId::from_raw("wrong-run");
    assert_eq!(
        seal_runtime_endpoint_outcome(wrong_run, &request, None),
        Err(RuntimeCoordinatorError::BindingMismatch)
    );
    for (pointer, replacement) in [
        ("/outcome/schema_version", json!(999)),
        ("/outcome/state", json!("observation")),
        ("/outcome/turn_count", json!(request.limits.max_turns + 1)),
        (
            "/outcome/model_call_count",
            json!(request.limits.max_model_calls + 1),
        ),
        (
            "/outcome/tool_call_count",
            json!(request.limits.max_tool_calls + 1),
        ),
        ("/outcome/prior_event_sha256", json!("bad")),
        ("/outcome/unresolved_codes", json!(["invalid code"])),
        ("/outcome/evidence", json!([])),
        ("/outcome/receipt_ids", json!(["duplicate", "duplicate"])),
        ("/outcome/output/payload/bytes", json!([123, 32, 125])),
        ("/outcome/evidence/0/content_sha256", json!("b".repeat(64))),
        (
            "/answer_evidence/completion_claim_sha256",
            json!("b".repeat(64)),
        ),
        ("/answer_evidence/citations_sha256", json!("b".repeat(64))),
    ] {
        let mut value = original.clone();
        *value.pointer_mut(pointer).unwrap() = replacement;
        let changed: RuntimeEndpointOutcome = serde_json::from_value(value).unwrap();
        assert!(
            verify_runtime_endpoint_outcome(&changed, &request, Some(&claim)).is_err(),
            "{pointer}"
        );
    }
    // Even a freshly hashed, otherwise valid output cannot replace the accepted bytes.
    for (turn_count, model_call_count) in [(0, 0), (1, 0), (2, 1), (2, 2)] {
        let mut changed = envelope.outcome.clone();
        changed.turn_count = turn_count;
        changed.model_call_count = model_call_count;
        assert_eq!(
            seal_runtime_endpoint_outcome(changed, &request, Some(&claim)),
            Err(RuntimeCoordinatorError::OutcomeDenied)
        );
    }
    let mut changed = envelope.outcome.clone();
    let Some(RuntimeOutput::Inline { payload }) = changed.output.as_mut() else {
        unreachable!()
    };
    payload.bytes = b"[]".to_vec();
    payload.sha256 = sha256(&payload.bytes);
    assert_eq!(
        seal_runtime_endpoint_outcome(changed, &request, Some(&claim)),
        Err(RuntimeCoordinatorError::OutcomeDenied)
    );
}

#[test]
fn endpoint_outcome_failure_is_truthful_without_claim_and_success_cannot_drop_provenance() {
    let request = seal_runtime_endpoint_run_request(request()).unwrap();
    let turn = accepted(&request);
    let claim = claim(&request, &turn);
    let raw = outcome(&request, &claim);
    for state in [
        AgentStateKind::Failed,
        AgentStateKind::Blocked,
        AgentStateKind::Cancelled,
        AgentStateKind::Exhausted,
    ] {
        let mut failed = raw.clone();
        failed.state = state;
        failed.output = None;
        failed.evidence.clear();
        let envelope = seal_runtime_endpoint_outcome(failed.clone(), &request, None).unwrap();
        assert!(envelope.answer_evidence.is_none());
        assert_eq!(
            verify_runtime_endpoint_outcome(&envelope, &request, None),
            Ok(())
        );
        assert_eq!(
            seal_runtime_endpoint_outcome(failed, &request, Some(&claim)),
            Err(RuntimeCoordinatorError::OutcomeDenied)
        );
        let mut forged = envelope;
        forged.outcome.state = AgentStateKind::Success;
        forged.outcome.output = raw.output.clone();
        forged.outcome.evidence = raw.evidence.clone();
        assert!(verify_runtime_endpoint_outcome(&forged, &request, None).is_err());
    }
    let envelope = seal_runtime_endpoint_outcome(raw, &request, Some(&claim)).unwrap();
    let mut missing = envelope;
    missing.answer_evidence = None;
    assert!(verify_runtime_endpoint_outcome(&missing, &request, Some(&claim)).is_err());
}

#[test]
fn endpoint_outcome_wire_is_versioned_closed_and_not_a_native_outcome() {
    let request = seal_runtime_endpoint_run_request(request()).unwrap();
    let turn = accepted(&request);
    let claim = claim(&request, &turn);
    let envelope =
        seal_runtime_endpoint_outcome(outcome(&request, &claim), &request, Some(&claim)).unwrap();
    let bytes = to_canonical_json(&envelope).unwrap();
    let decoded = from_json::<RuntimeEndpointOutcome>(&bytes).unwrap();
    assert_eq!(decoded, envelope);
    assert_eq!(
        verify_runtime_endpoint_outcome(&decoded, &request, Some(&claim)),
        Ok(())
    );
    assert!(from_json::<RuntimeOutcome>(&bytes).is_err());
    let mut unknown = serde_json::to_value(&envelope).unwrap();
    unknown["unexpected"] = json!(true);
    assert!(from_json::<RuntimeEndpointOutcome>(&serde_json::to_vec(&unknown).unwrap()).is_err());
    let mut omitted = serde_json::to_value(&envelope).unwrap();
    omitted.as_object_mut().unwrap().remove("answer_evidence");
    assert!(from_json::<RuntimeEndpointOutcome>(&serde_json::to_vec(&omitted).unwrap()).is_err());
    let text = String::from_utf8(bytes).unwrap();
    let duplicate = text.replacen(
        "\"answer_evidence\":",
        "\"answer_evidence\":null,\"answer_evidence\":",
        1,
    );
    assert!(from_json::<RuntimeEndpointOutcome>(duplicate.as_bytes()).is_err());
    let mut wrong_version = serde_json::to_value(&envelope).unwrap();
    wrong_version["outcome"]["schema_version"] = json!(999);
    assert!(
        from_json::<RuntimeEndpointOutcome>(&serde_json::to_vec(&wrong_version).unwrap()).is_err()
    );
}

#[test]
fn endpoint_outcome_refuses_native_answer_evidence_and_uncomposed_modes() {
    let original = seal_runtime_endpoint_run_request(request()).unwrap();
    let turn = accepted(&original);
    let claim = claim(&original, &turn);
    let mut raw = outcome(&original, &claim);
    let native = seal_runtime_run_request(super::super::super::tests::request()).unwrap();
    let payload = claim.payload().unwrap();
    raw.answer_evidence = Some(Box::new(
        crate::runtime_answer::compose_inferred_runtime_answer(
            &native,
            agentmage_kernel_contracts::ModelRunId::from_raw("native-model-run"),
            payload.sha256.clone(),
            payload.sha256.clone(),
            payload.bytes.len() as u64,
            payload.media_type.clone(),
            raw.evidence.clone(),
        )
        .unwrap(),
    ));
    assert_eq!(
        seal_runtime_endpoint_outcome(raw, &original, Some(&claim)),
        Err(RuntimeCoordinatorError::OutcomeDenied)
    );
    let mut unsupported = original.clone();
    unsupported.mode = RuntimeSessionMode::DurableReadOnly;
    let unsupported = seal_runtime_endpoint_run_request(unsupported).unwrap();
    let mut failed = outcome(&unsupported, &claim);
    failed.state = AgentStateKind::Failed;
    failed.output = None;
    assert_eq!(
        seal_runtime_endpoint_outcome(failed, &unsupported, None),
        Err(RuntimeCoordinatorError::OutcomeDenied)
    );
}
