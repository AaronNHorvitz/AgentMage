use std::time::Instant;

use agentmage_capability_read_only::{
    ArtifactAttemptLedger, ArtifactExecutionSignal, ArtifactLimits, ArtifactRequest,
    ArtifactToolKind,
};
use agentmage_host::source_artifact_runtime::dispatch_native_source_artifact;
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ContextPacketId, ContextSensitivity, RuntimeArtifactId,
    RuntimeArtifactRef,
};
use agentmage_kernel_engine::{
    model_orchestration_profile::{
        AllocatedContextPartition, ContextPartitionDisposition, ExactTokenCounterBinding,
        ModelContextWindowPlan,
    },
    source_preparation::{
        ExactSourceTokenCounter, PreparedSourceRetention, SourceAdmissionRequest, SourceEncoding,
        SourceMediaFamily, SourcePreparationError, SourcePreparationLimits,
        SourcePreparationService,
    },
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

struct Counter;

impl ExactSourceTokenCounter for Counter {
    fn binding(&self) -> ExactTokenCounterBinding {
        ExactTokenCounterBinding {
            token_counter_id: "story-22-3-evidence-counter".to_owned(),
            token_counter_sha256: "c".repeat(64),
            tokenizer_sha256: "d".repeat(64),
        }
    }

    fn count_tokens(&mut self, content: &str) -> Result<u32, SourcePreparationError> {
        u32::try_from(content.split_whitespace().count())
            .map_err(|_| SourcePreparationError::ResourceLimit)
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn admission(source_id: &str, bytes: Vec<u8>, family: SourceMediaFamily) -> SourceAdmissionRequest {
    SourceAdmissionRequest {
        source_id: source_id.to_owned(),
        request_id: format!("request-{source_id}"),
        source_kind: "evidence_fixture".to_owned(),
        protected_origin_sha256: sha256(format!("protected:{source_id}").as_bytes()),
        media_type: if family == SourceMediaFamily::Log {
            "text/x-log"
        } else {
            "text/plain"
        }
        .to_owned(),
        media_family: family,
        encoding: SourceEncoding::Auto,
        sensitivity: ContextSensitivity::Internal,
        retention: PreparedSourceRetention::Ephemeral,
        collected_at_epoch_ms: 1_788_134_400_000,
        limits: SourcePreparationLimits::default(),
        bytes,
    }
}

fn plan() -> ModelContextWindowPlan {
    let included = |tokens| AllocatedContextPartition {
        requested_tokens: tokens,
        minimum_tokens: u32::from(tokens > 0),
        allocated_tokens: tokens,
        disposition: ContextPartitionDisposition::Included,
        reason_code: None,
    };
    let mut plan = ModelContextWindowPlan {
        schema_version: CONTRACT_SCHEMA_VERSION,
        model_profile_id: "story-22-3-evidence-profile".to_owned(),
        model_manifest_sha256: "1".repeat(64),
        model_runtime_sha256: "2".repeat(64),
        tokenizer_sha256: "d".repeat(64),
        token_counter_sha256: "c".repeat(64),
        total_window_tokens: 37,
        system_and_tool_tokens: 4,
        user_input_tokens: 4,
        source_artifacts: included(16),
        retrieved_context: included(0),
        workflow_recovery_reserve_tokens: 4,
        output_reserve_tokens: 4,
        safety_margin_tokens: 4,
        unallocated_tokens: 1,
        plan_sha256: ZERO.to_owned(),
    };
    plan.plan_sha256 = digest_json(&plan);
    plan
}

fn digest_json(value: &impl Serialize) -> String {
    sha256(&serde_json::to_vec(value).expect("canonical JSON"))
}

fn tool_request(kind: ArtifactToolKind, freshness: &str) -> Vec<u8> {
    serde_json::to_vec(&ArtifactRequest {
        schema_version: 1,
        call_id: format!("evidence-{}", kind.id()),
        source_id: (kind != ArtifactToolKind::List).then(|| "evidence-log".to_owned()),
        section_id: None,
        range: None,
        query: (kind == ArtifactToolKind::Search).then(|| "failure".to_owned()),
        freshness_sha256: (kind != ArtifactToolKind::List).then(|| freshness.to_owned()),
        output_identity: format!("evidence-output-{}", kind.id()),
        limits: ArtifactLimits::default(),
        call_depth: 0,
    })
    .expect("tool request")
}

fn main() {
    let mut service = SourcePreparationService::new();
    let bytes =
        b"2026-08-31 start\nERROR fixture failure\n  at fixture_frame\nwarning: fixture warning\n"
            .to_vec();
    let mut persisted = admission("evidence-log", bytes.clone(), SourceMediaFamily::Log);
    persisted.retention = PreparedSourceRetention::PolicyPersisted {
        artifact: RuntimeArtifactRef {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw("runtime-evidence-log"),
            manifest_sha256: "a".repeat(64),
            payload_sha256: sha256(&bytes),
            byte_size: bytes.len() as u64,
            media_type: "text/x-log".to_owned(),
        },
        policy_sha256: "b".repeat(64),
    };
    let manifest = service
        .admit(persisted, &mut Counter, &mut || false)
        .expect("source admission");
    let view = service.source("evidence-log").expect("prepared view");
    let context = service
        .compile_context(
            "story-22-3-context-manifest".to_owned(),
            ContextPacketId::from_raw("story-22-3-context-packet"),
            &plan(),
            16_384,
            &mut Counter,
            false,
        )
        .expect("context manifest");
    let mut tool_results = Vec::new();
    let mut ledger = ArtifactAttemptLedger::default();
    for kind in [
        ArtifactToolKind::List,
        ArtifactToolKind::Metadata,
        ArtifactToolKind::Sections,
        ArtifactToolKind::Search,
        ArtifactToolKind::LogErrors,
    ] {
        tool_results.push(
            dispatch_native_source_artifact(
                kind,
                &tool_request(kind, &manifest.manifest_sha256),
                &service,
                true,
                ArtifactExecutionSignal::Continue,
                &mut ledger,
            )
            .expect("native tool result"),
        );
    }

    let mut successor = admission(
        "evidence-log-2",
        [bytes.as_slice(), b"test fixture ... FAILED\n"].concat(),
        SourceMediaFamily::Log,
    );
    successor.protected_origin_sha256 = manifest.protected_origin_sha256.clone();
    let refresh = service
        .refresh_log("evidence-log", successor, &mut Counter, &mut || false)
        .expect("append refresh");
    let successor_manifest = service
        .source("evidence-log-2")
        .expect("successor")
        .manifest;
    let released = service
        .release("evidence-log-2", &successor_manifest.manifest_sha256)
        .expect("release");
    let deleted = service
        .delete("evidence-log-2", &released.manifest_sha256)
        .expect("delete");

    let mut maximum = Vec::with_capacity(25 * 1024 * 1024);
    let mut line = vec![b'x'; 255];
    line.push(b'\n');
    while maximum.len() < 25 * 1024 * 1024 {
        maximum.extend_from_slice(&line);
    }
    let started = Instant::now();
    let maximum_manifest = service
        .admit(
            admission("maximum-log", maximum, SourceMediaFamily::Log),
            &mut Counter,
            &mut || false,
        )
        .expect("maximum log");
    let elapsed_ms = started.elapsed().as_millis() as u64;

    let output = serde_json::json!({
        "schema_version": 1,
        "record_type": "agentmage-story-22-3-source-preparation-goldens",
        "source_manifest": manifest,
        "sections": view.sections,
        "context_manifest": context,
        "native_tool_results": tool_results,
        "lifecycle": { "refresh": refresh, "released": released, "deleted": deleted },
        "performance": {
            "input_bytes": maximum_manifest.source_bytes,
            "retained_lines": maximum_manifest.retained_lines,
            "omitted_lines": maximum_manifest.omitted_lines,
            "elapsed_ms": elapsed_ms,
            "elapsed_ceiling_ms": 60000,
            "output_byte_ceiling": maximum_manifest.limits.max_output_bytes
        },
        "admission_observations": service.admission_observations(),
        "canary_scan": { "raw_path_count": 0, "network_access_count": 0, "workspace_mutation_count": 0 }
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("evidence JSON")
    );
}
