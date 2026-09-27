// Test-only native source assembly through the existing canonical owners.
// Full bytes are published only after terminal durability, never from provider metadata.
// All source bytes stay in the existing private full artifact owner. This helper
// does not issue approval, execute an effect, substitute a driver or admit a model.
struct NativeSourceAssembly<'a> {
    call: &'a ToolCall,
    started: &'a RuntimeEvent,
    receipt: &'a Receipt,
    transaction_id: &'a AuthorityTransactionId,
    reservation_sha256: &'a str,
    packet: &'a agentmage_kernel_engine::research_fetch::PublicGetWorkerPacket,
    result: &'a PublicGetResultBinding,
    output_schema: &'a SchemaReference,
}

fn retain_native_source(
    runtime: &mut DurableAuthorityRuntime,
    payloads: &mut crate::runtime_artifact_store::LinuxRuntimeArtifactPayloadStore,
    context: &ResearchBudgetContext,
    pending: PendingRuntimeEffectCommit,
    source: &NativeSourceAssembly<'_>,
    clock: &mut dyn RuntimeClock,
) -> (RuntimeArtifactRef, RuntimeArtifactRef) {
    let completed_at = clock.now_epoch_ms().unwrap();
    assert_eq!(source.receipt.outcome, OperationOutcome::Succeeded);
    assert!(completed_at >= source.result.parent_interval().cleanup_verified_epoch_ms);
    // Prepare references only in memory while the canonical terminal guard is active.
    let manifest = |name: &str, media: &str, bytes: &[u8]| {
        seal_runtime_artifact_manifest(RuntimeArtifactManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw(format!("native-source-{name}")),
            kind: RuntimeArtifactKind::Report,
            payload_sha256: hash(bytes),
            byte_size: bytes.len() as u64,
            media_type: media.into(),
            sensitivity: ContextSensitivity::Private,
            retention: retention(),
            session_id: context.session_id.clone(),
            task_id: context.task_id.clone(),
            producer_run_id: context.run_id.clone(),
            producer_turn_id: source.started.turn_id.clone(),
            producer_operation_id: source.started.operation_id.clone(),
            receipt_id: Some(source.receipt.receipt_id.clone()),
            policy_id: source.started.policy_id.clone(),
            policy_sha256: context.policy_sha256.clone(),
            created_at_epoch_ms: completed_at,
            integrity: RuntimeArtifactIntegrityState::Verified,
            preview: None,
            manifest_sha256: "0".repeat(64),
        })
        .unwrap()
    };
    let call_bytes = to_canonical_json(source.call).unwrap();
    let packet_bytes = source.packet.bytes().to_vec();
    let material_bytes = source.result.redacted_material().to_vec();
    let frame_bytes = source.result.response().frame().to_vec();
    let call = manifest("call", "application/json", &call_bytes);
    let packet = manifest("packet", "application/json", &packet_bytes);
    let material = manifest("material", "application/json", &material_bytes);
    let frame = manifest("frame", "application/octet-stream", &frame_bytes);
    let frame_ref = runtime_artifact_ref(&frame).unwrap();
    let output = PublicGetArtifactOutput {
        schema_version: 1,
        packet: runtime_artifact_ref(&packet).unwrap(),
        material: runtime_artifact_ref(&material).unwrap(),
        frame: frame_ref.clone(),
    };
    let output_bytes = serde_json::to_vec(&output).unwrap();
    let result = ToolResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: source.call.tool_call_id.clone(),
        correlation_id: source.call.correlation_id.clone(),
        outcome: OperationOutcome::Succeeded,
        output: Some(ContractPayload {
            schema: source.output_schema.clone(),
            media_type: "application/json".into(),
            sha256: hash(&output_bytes),
            bytes: output_bytes,
        }),
        validation_issues: vec![],
        evidence: vec![],
        error: None,
        elapsed_ms: completed_at
            .checked_sub(source.started.occurred_at_epoch_ms)
            .unwrap(),
        state_change: StateChange::Changed,
    };
    let result_bytes = to_canonical_json(&result).unwrap();
    let result_manifest = manifest("result", "application/json", &result_bytes);
    let bundle = PublicGetArtifactBundle {
        schema_version: 1,
        transaction_id: source.transaction_id.clone(),
        reservation_sha256: source.reservation_sha256.into(),
        call: runtime_artifact_ref(&call).unwrap(),
        output,
        result: runtime_artifact_ref(&result_manifest).unwrap(),
    };
    let bundle_bytes = serde_json::to_vec(&bundle).unwrap();
    let bundle_manifest = manifest("bundle", "application/json", &bundle_bytes);
    let bundle_ref = runtime_artifact_ref(&bundle_manifest).unwrap();
    let terminal = seal_runtime_event(next(
        source.started,
        RuntimeEventKind::ToolCompleted {
            tool_call_id: source.call.tool_call_id.clone(),
            receipt_id: source.receipt.receipt_id.clone(),
            result_sha256: hash(&result_bytes),
        },
        completed_at,
        true,
    ))
    .unwrap();
    runtime
        .finish_effect_with_runtime_event(pending, terminal.clone())
        .unwrap();
    let mut prior = terminal;
    for (manifest, bytes) in [
        (call, call_bytes),
        (packet, packet_bytes),
        (material, material_bytes),
        (frame, frame_bytes),
        (result_manifest, result_bytes),
        (bundle_manifest, bundle_bytes),
    ] {
        let publication = runtime
            .publish_runtime_artifact(payloads, manifest, &mut bytes.as_slice())
            .unwrap();
        let mut event = next(
            &prior,
            RuntimeEventKind::ArtifactCreated {
                artifact_id: publication.reference.artifact_id.clone(),
                manifest_sha256: publication.reference.manifest_sha256.clone(),
            },
            clock.now_epoch_ms().unwrap(),
            true,
        );
        event.payload_reference = Some(runtime_payload_reference(&publication.manifest).unwrap());
        prior = seal_runtime_event(event).unwrap();
        runtime.record_runtime_event(prior.clone()).unwrap();
    }
    (bundle_ref, frame_ref)
}
