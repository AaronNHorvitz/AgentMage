use std::{collections::BTreeMap, fs, path::PathBuf};

use agentmage_kernel_contracts::{
    CanonicalArtifactEnvelope, CanonicalArtifactIngestionResult, CanonicalArtifactTransformation,
    CanonicalContextManifest, CanonicalTerminalResult, CanonicalToolObservation,
    CanonicalVerificationResult, CanonicalWorkflowDefinition, CanonicalWorkflowState,
    VersionedContract, from_json, to_canonical_json,
};
use agentmage_kernel_engine::engineering_records::{
    CanonicalRuntimeIdentityBindings, CanonicalRuntimeRecordRef, CanonicalRuntimeRecordSet,
    ValidateCanonicalRecord,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
struct FixtureManifest {
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    case_id: String,
    schema_name: String,
    category: String,
    path: String,
    expected_boundary: String,
    expected_code: Option<String>,
    canonical_sha256: Option<String>,
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/engineering-runtime/v2")
}

fn manifest() -> FixtureManifest {
    serde_json::from_slice(
        &fs::read(fixture_root().join("manifest.json")).expect("fixture manifest"),
    )
    .expect("closed fixture manifest")
}

fn bytes(case: &FixtureCase) -> Vec<u8> {
    fs::read(fixture_root().join(&case.path)).expect("fixture bytes")
}

fn sha256(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_validate_encode<T>(input: &[u8]) -> Result<Vec<u8>, String>
where
    T: VersionedContract + ValidateCanonicalRecord + PartialEq + std::fmt::Debug,
{
    let record = from_json::<T>(input).map_err(|error| error.code.clone())?;
    record
        .validate_canonical()
        .map_err(|error| error.code.to_owned())?;
    let canonical = to_canonical_json(&record).map_err(|error| error.code.clone())?;
    let reopened = from_json::<T>(&canonical).map_err(|error| error.code.clone())?;
    if record != reopened {
        return Err("engineering.fixture.round_trip_drift".to_owned());
    }
    Ok(canonical)
}

fn exercise(case: &FixtureCase) -> Result<Vec<u8>, String> {
    let input = bytes(case);
    match case.schema_name.as_str() {
        "artifact-envelope" => decode_validate_encode::<CanonicalArtifactEnvelope>(&input),
        "artifact-transformation" => {
            decode_validate_encode::<CanonicalArtifactTransformation>(&input)
        }
        "artifact-ingestion-result" => {
            decode_validate_encode::<CanonicalArtifactIngestionResult>(&input)
        }
        "context-manifest" => decode_validate_encode::<CanonicalContextManifest>(&input),
        "workflow-definition" => decode_validate_encode::<CanonicalWorkflowDefinition>(&input),
        "workflow-state" => decode_validate_encode::<CanonicalWorkflowState>(&input),
        "tool-observation" => decode_validate_encode::<CanonicalToolObservation>(&input),
        "verification-result" => decode_validate_encode::<CanonicalVerificationResult>(&input),
        "terminal-result" => decode_validate_encode::<CanonicalTerminalResult>(&input),
        unknown => Err(format!("unknown fixture schema: {unknown}")),
    }
}

fn valid_case_by_schema(manifest: &FixtureManifest) -> BTreeMap<&str, &FixtureCase> {
    manifest
        .cases
        .iter()
        .filter(|case| case.category == "valid")
        .map(|case| (case.schema_name.as_str(), case))
        .collect()
}

fn parse<T: VersionedContract>(case: &FixtureCase) -> T {
    from_json(&bytes(case)).expect("valid corpus record")
}

#[test]
fn valid_records_round_trip_with_exact_cross_language_canonical_bytes() {
    let manifest = manifest();
    let valid = manifest
        .cases
        .iter()
        .filter(|case| case.expected_boundary == "admit")
        .collect::<Vec<_>>();
    assert_eq!(valid.len(), 9);
    for case in valid {
        let canonical = exercise(case).unwrap_or_else(|code| panic!("{}: {code}", case.case_id));
        assert_eq!(
            sha256(&canonical),
            case.canonical_sha256.as_deref().expect("canonical digest"),
            "{} changed canonical bytes between JavaScript and Rust",
            case.case_id
        );
        let value: serde_json::Value = serde_json::from_slice(&canonical).expect("canonical JSON");
        assert_eq!(value["schema_version"], 2);
    }
}

#[test]
fn every_declared_schema_failure_fails_before_canonical_publication() {
    let manifest = manifest();
    let rejected = manifest
        .cases
        .iter()
        .filter(|case| case.expected_boundary == "reject-schema")
        .collect::<Vec<_>>();
    assert_eq!(rejected.len(), 45);
    for case in rejected {
        let result = exercise(case);
        assert!(
            result.is_err(),
            "{} was unexpectedly admitted",
            case.case_id
        );
        assert!(case.canonical_sha256.is_none());
        if case.category == "unsupported-version" {
            assert_eq!(result.unwrap_err(), "contract.version.unsupported");
        }
    }
}

#[test]
fn cyclic_and_stale_records_fail_at_their_exact_trusted_boundaries() {
    let manifest = manifest();
    let cyclic = manifest
        .cases
        .iter()
        .find(|case| case.category == "cyclic")
        .expect("cyclic fixture");
    assert_eq!(
        exercise(cyclic).unwrap_err(),
        "engineering.workflow.dependency_cycle"
    );
    assert_eq!(
        cyclic.expected_code.as_deref(),
        Some("engineering.workflow.dependency_cycle")
    );

    let cases = valid_case_by_schema(&manifest);
    let stale = manifest
        .cases
        .iter()
        .find(|case| case.category == "stale")
        .expect("stale fixture");
    let artifact: CanonicalArtifactEnvelope = parse(stale);
    assert_eq!(artifact.validate_canonical(), Ok(()));
    let transformation: CanonicalArtifactTransformation = parse(cases["artifact-transformation"]);
    let ingestion: CanonicalArtifactIngestionResult = parse(cases["artifact-ingestion-result"]);
    let context: CanonicalContextManifest = parse(cases["context-manifest"]);
    let definition: CanonicalWorkflowDefinition = parse(cases["workflow-definition"]);
    let state: CanonicalWorkflowState = parse(cases["workflow-state"]);
    let observation: CanonicalToolObservation = parse(cases["tool-observation"]);
    let verification: CanonicalVerificationResult = parse(cases["verification-result"]);
    let terminal: CanonicalTerminalResult = parse(cases["terminal-result"]);
    let transformations = [transformation];
    let observations = [observation];
    let verifications = [verification];
    let set = CanonicalRuntimeRecordSet {
        bindings: CanonicalRuntimeIdentityBindings {
            request_id: "request:1",
            task_id: "task:1",
            session_id: "session:1",
            source_sha256: &"a".repeat(64),
            workflow_definition_sha256: &"a".repeat(64),
        },
        artifact: &artifact,
        transformations: &transformations,
        ingestion: &ingestion,
        context: &context,
        workflow_definition: &definition,
        workflow_state: &state,
        observations: &observations,
        verifications: &verifications,
        terminal_result: &terminal,
    };
    let error = set.validate().expect_err("stale request binding must fail");
    assert_eq!(error.code, "engineering.record.binding_mismatch");
    assert_eq!(stale.expected_code.as_deref(), Some(error.code));
}

#[test]
fn borrowed_record_boundary_matches_the_canonical_contract_encoder_for_every_family() {
    let manifest = manifest();
    let cases = valid_case_by_schema(&manifest);
    let artifact: CanonicalArtifactEnvelope = parse(cases["artifact-envelope"]);
    let transformation: CanonicalArtifactTransformation = parse(cases["artifact-transformation"]);
    let ingestion: CanonicalArtifactIngestionResult = parse(cases["artifact-ingestion-result"]);
    let context: CanonicalContextManifest = parse(cases["context-manifest"]);
    let definition: CanonicalWorkflowDefinition = parse(cases["workflow-definition"]);
    let state: CanonicalWorkflowState = parse(cases["workflow-state"]);
    let observation: CanonicalToolObservation = parse(cases["tool-observation"]);
    let verification: CanonicalVerificationResult = parse(cases["verification-result"]);
    let terminal: CanonicalTerminalResult = parse(cases["terminal-result"]);
    let records = [
        (
            CanonicalRuntimeRecordRef::ArtifactEnvelope(&artifact),
            cases["artifact-envelope"],
        ),
        (
            CanonicalRuntimeRecordRef::ArtifactTransformation(&transformation),
            cases["artifact-transformation"],
        ),
        (
            CanonicalRuntimeRecordRef::ArtifactIngestionResult(&ingestion),
            cases["artifact-ingestion-result"],
        ),
        (
            CanonicalRuntimeRecordRef::ContextManifest(&context),
            cases["context-manifest"],
        ),
        (
            CanonicalRuntimeRecordRef::WorkflowDefinition(&definition),
            cases["workflow-definition"],
        ),
        (
            CanonicalRuntimeRecordRef::WorkflowState(&state),
            cases["workflow-state"],
        ),
        (
            CanonicalRuntimeRecordRef::ToolObservation(&observation),
            cases["tool-observation"],
        ),
        (
            CanonicalRuntimeRecordRef::VerificationResult(&verification),
            cases["verification-result"],
        ),
        (
            CanonicalRuntimeRecordRef::TerminalResult(&terminal),
            cases["terminal-result"],
        ),
    ];
    for (record, case) in records {
        let payload = record.artifact_payload().expect("admitted record payload");
        assert_eq!(sha256(&payload), case.canonical_sha256.as_deref().unwrap());
    }
}
