use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    CanonicalArtifactEnvelope, CanonicalArtifactIngestionResult, CanonicalArtifactTransformation,
    CanonicalContextManifest, CanonicalTerminalResult, CanonicalToolObservation,
    CanonicalVerificationResult, CanonicalWorkflowDefinition, CanonicalWorkflowState,
    VersionedContract, from_json,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn assert_schema_bound<T>(schema_text: &str, candidate: Value, expected_version: u16)
where
    T: Serialize + DeserializeOwned + VersionedContract,
{
    let schema: Value = serde_json::from_str(schema_text).expect("schema parses");
    let schema_fields = schema["properties"]
        .as_object()
        .expect("schema has object properties")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();

    let candidate_bytes = serde_json::to_vec(&candidate).expect("candidate serializes");
    let record: T = from_json(&candidate_bytes).expect("Rust boundary admits schema record");
    assert_eq!(record.schema_version(), expected_version);
    let encoded = serde_json::to_value(record).expect("Rust type serializes");
    let rust_fields = encoded
        .as_object()
        .expect("record serializes as an object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        rust_fields, schema_fields,
        "Rust and schema fields diverged"
    );

    let mut widened = candidate;
    widened
        .as_object_mut()
        .expect("candidate is an object")
        .insert("unadmitted_field".to_owned(), Value::Bool(true));
    assert!(
        from_json::<T>(&serde_json::to_vec(&widened).expect("widened candidate serializes"))
            .is_err(),
        "closed Rust record accepted an unadmitted field",
    );
}

fn assert_required_nullable<T>(candidate: &Value, fields: &[&str])
where
    T: DeserializeOwned + Serialize + VersionedContract,
{
    for field in fields {
        let mut missing = candidate.clone();
        assert!(
            missing
                .as_object_mut()
                .expect("candidate is an object")
                .remove(*field)
                .is_some(),
            "fixture did not contain {field}",
        );
        assert!(
            from_json::<T>(&serde_json::to_vec(&missing).expect("missing candidate serializes"))
                .is_err(),
            "schema-required nullable field {field} was treated as optional",
        );
    }
}

fn assert_versions_fail_closed<T>(candidate: &Value)
where
    T: std::fmt::Debug + DeserializeOwned + Serialize + VersionedContract,
{
    for unsupported in [0, 1, 3, u16::MAX] {
        let mut changed = candidate.clone();
        changed["schema_version"] = json!(unsupported);
        let bytes = serde_json::to_vec(&changed).expect("version candidate serializes");
        let failure = from_json::<T>(&bytes).expect_err("unsupported version must fail closed");
        assert_eq!(failure.code, "contract.version.unsupported");
    }
}

fn artifact_envelope() -> Value {
    json!({
        "schema_version": 2,
        "artifact_id": "artifact-1",
        "request_id": "request-1",
        "authority_id": "authority-1",
        "origin": "paste",
        "media_type": "text/plain",
        "classification": "internal",
        "capture_state": "captured",
        "byte_length": 4,
        "sha256": DIGEST,
        "collected_at": "2026-08-29T12:00:00Z"
    })
}

fn artifact_transformation() -> Value {
    json!({
        "schema_version": 2,
        "transformation_id": "transformation-1",
        "artifact_id": "artifact-1",
        "transformer_id": "text-parser",
        "transformer_version": "1",
        "input_sha256": DIGEST,
        "output_sha256": DIGEST,
        "source_ranges": [{"start_byte": 0, "end_byte_exclusive": 4}],
        "warnings": [],
        "reproducible": true
    })
}

fn artifact_ingestion_result() -> Value {
    json!({
        "schema_version": 2,
        "ingestion_id": "ingestion-1",
        "artifact_id": "artifact-1",
        "disposition": "parsed",
        "source": {"artifact_id": "artifact-1", "sha256": DIGEST, "byte_length": 4},
        "transformation_ids": ["transformation-1"],
        "warnings": [],
        "error_code": null,
        "terminal": true
    })
}

fn context_manifest() -> Value {
    json!({
        "schema_version": 2,
        "context_manifest_id": "context-1",
        "session_id": "session-1",
        "turn_id": "turn-1",
        "model_profile_id": "model-1",
        "source_artifact_count": 1,
        "items": [{
            "artifact_id": "artifact-1",
            "disposition": "included",
            "ranges": [{"start_byte": 0, "end_byte_exclusive": 4}],
            "token_count": 1,
            "reason_code": null,
            "reason": null
        }],
        "total_input_tokens": 1,
        "reserved_output_tokens": 1,
        "safety_margin_tokens": 1,
        "manifest_sha256": DIGEST
    })
}

fn workflow_definition() -> Value {
    json!({
        "schema_version": 2,
        "workflow_id": "workflow-1",
        "workflow_version": 1,
        "input_schema": {"schema_id": "input-1", "schema_version": 1, "schema_sha256": DIGEST},
        "output_schema": {"schema_id": "output-1", "schema_version": 1, "schema_sha256": DIGEST},
        "steps": [{
            "step_id": "step-1",
            "depends_on": [],
            "model_role": null,
            "tool_id": "tool-1",
            "effect_class": "read_only",
            "retry_class": "recoverable_read",
            "verifier_ids": ["verifier-1"],
            "budgets": {
                "turns": 1,
                "tokens": 1,
                "duration_ms": 1,
                "tool_calls": 1,
                "attempts": 1,
                "no_progress_events": 1,
                "output_bytes": 1,
                "memory_bytes": 1,
                "cost_minor_units": 0
            }
        }],
        "entry_step_ids": ["step-1"],
        "definition_sha256": DIGEST
    })
}

fn workflow_state() -> Value {
    json!({
        "schema_version": 2,
        "workflow_id": "workflow-1",
        "workflow_version": 1,
        "sequence": 0,
        "state": "ready",
        "active_step_id": null,
        "completed_step_ids": [],
        "attempt_ids": [],
        "consumed_budget_sha256": DIGEST,
        "terminal_result_id": null
    })
}

fn tool_observation() -> Value {
    json!({
        "schema_version": 2,
        "observation_id": "observation-1",
        "tool_call_id": "tool-call-1",
        "attempt_id": "attempt-1",
        "task_id": "task-1",
        "step_id": "step-1",
        "tool_id": "tool-1",
        "tool_version": "1",
        "tool_schema_sha256": DIGEST,
        "arguments_sha256": DIGEST,
        "authority_id": "authority-1",
        "started_at": "2026-08-29T12:00:00Z",
        "completed_at": "2026-08-29T12:00:01Z",
        "outcome": "succeeded",
        "exit_code": 0,
        "signal": null,
        "stdout": null,
        "stderr": null,
        "stdout_excerpt": "",
        "stderr_excerpt": "",
        "stdout_truncated": false,
        "stderr_truncated": false,
        "generated_artifact_ids": [],
        "state_change": "not_changed",
        "descendants_cleaned": true,
        "resource_usage_sha256": DIGEST,
        "retry_disposition": "not_eligible",
        "receipt_sha256": DIGEST,
        "terminal": true
    })
}

fn verification_result() -> Value {
    json!({
        "schema_version": 2,
        "verification_result_id": "verification-1",
        "workflow_id": "workflow-1",
        "step_id": "step-1",
        "verifier_id": "verifier-1",
        "verifier_version": "1",
        "subject_sha256": DIGEST,
        "observed_evidence_sha256s": [DIGEST],
        "preserved_invariants": ["invariant-1"],
        "prohibited_effects_observed": [],
        "outcome": "passed",
        "current": true,
        "result_sha256": DIGEST
    })
}

fn terminal_result() -> Value {
    json!({
        "schema_version": 2,
        "terminal_result_id": "terminal-1",
        "workflow_id": "workflow-1",
        "outcome": "verified_success",
        "verification_result_ids": ["verification-1"],
        "last_verified_state_sha256": DIGEST,
        "diagnostic_code": null,
        "safe_next_action": null,
        "established_by": "agentmage-runtime-verifier",
        "result_sha256": DIGEST
    })
}

#[test]
fn all_nine_runtime_record_families_match_their_admitted_schema_shape() {
    assert_schema_bound::<CanonicalArtifactEnvelope>(
        include_str!("../../../schemas/engineering-runtime/artifact-envelope.schema.json"),
        artifact_envelope(),
        2,
    );
    assert_schema_bound::<CanonicalArtifactTransformation>(
        include_str!("../../../schemas/engineering-runtime/artifact-transformation.schema.json"),
        artifact_transformation(),
        2,
    );
    assert_schema_bound::<CanonicalArtifactIngestionResult>(
        include_str!("../../../schemas/engineering-runtime/artifact-ingestion-result.schema.json"),
        artifact_ingestion_result(),
        2,
    );
    assert_schema_bound::<CanonicalContextManifest>(
        include_str!("../../../schemas/engineering-runtime/context-manifest.schema.json"),
        context_manifest(),
        2,
    );
    assert_schema_bound::<CanonicalWorkflowDefinition>(
        include_str!("../../../schemas/engineering-runtime/workflow-definition.schema.json"),
        workflow_definition(),
        2,
    );
    assert_schema_bound::<CanonicalWorkflowState>(
        include_str!("../../../schemas/engineering-runtime/workflow-state.schema.json"),
        workflow_state(),
        2,
    );
    assert_schema_bound::<CanonicalToolObservation>(
        include_str!("../../../schemas/engineering-runtime/tool-observation.schema.json"),
        tool_observation(),
        2,
    );
    assert_schema_bound::<CanonicalVerificationResult>(
        include_str!("../../../schemas/engineering-runtime/verification-result.schema.json"),
        verification_result(),
        2,
    );
    assert_schema_bound::<CanonicalTerminalResult>(
        include_str!("../../../schemas/engineering-runtime/terminal-result.schema.json"),
        terminal_result(),
        2,
    );
}

#[test]
fn schema_required_nullable_fields_cannot_be_omitted() {
    assert_required_nullable::<CanonicalArtifactEnvelope>(
        &artifact_envelope(),
        &["byte_length", "sha256"],
    );
    assert_required_nullable::<CanonicalArtifactTransformation>(
        &artifact_transformation(),
        &["output_sha256"],
    );
    assert_required_nullable::<CanonicalArtifactIngestionResult>(
        &artifact_ingestion_result(),
        &["source", "error_code"],
    );
    assert_required_nullable::<CanonicalWorkflowState>(
        &workflow_state(),
        &["active_step_id", "terminal_result_id"],
    );
    assert_required_nullable::<CanonicalToolObservation>(
        &tool_observation(),
        &["exit_code", "signal", "stdout", "stderr"],
    );
    assert_required_nullable::<CanonicalVerificationResult>(&verification_result(), &["step_id"]);
    assert_required_nullable::<CanonicalTerminalResult>(
        &terminal_result(),
        &["diagnostic_code", "safe_next_action"],
    );

    for field in ["model_role", "tool_id"] {
        let mut missing = workflow_definition();
        missing["steps"][0]
            .as_object_mut()
            .expect("workflow step is an object")
            .remove(field);
        assert!(
            from_json::<CanonicalWorkflowDefinition>(
                &serde_json::to_vec(&missing).expect("workflow candidate serializes"),
            )
            .is_err(),
            "schema-required nullable workflow field {field} was treated as optional",
        );
    }
}

#[test]
fn every_runtime_record_family_rejects_unsupported_versions() {
    assert_versions_fail_closed::<CanonicalArtifactEnvelope>(&artifact_envelope());
    assert_versions_fail_closed::<CanonicalArtifactTransformation>(&artifact_transformation());
    assert_versions_fail_closed::<CanonicalArtifactIngestionResult>(&artifact_ingestion_result());
    assert_versions_fail_closed::<CanonicalContextManifest>(&context_manifest());
    assert_versions_fail_closed::<CanonicalWorkflowDefinition>(&workflow_definition());
    assert_versions_fail_closed::<CanonicalWorkflowState>(&workflow_state());
    assert_versions_fail_closed::<CanonicalToolObservation>(&tool_observation());
    assert_versions_fail_closed::<CanonicalVerificationResult>(&verification_result());
    assert_versions_fail_closed::<CanonicalTerminalResult>(&terminal_result());
}
